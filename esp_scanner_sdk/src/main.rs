//! # ESP Scanner CLI
//!

use esp_compiler::{log_error, log_info, log_success, logging, pipeline};
use esp_scanner_base::execution::ExecutionEngine;
use esp_scanner_base::resolution::engine::ResolutionEngine;
use esp_scanner_base::types::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::init_global_logging()?;
    log_info!("ESP Scanner starting");

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    if args[1] == "--help" || args[1] == "-h" {
        print_help(&args[0]);
        return Ok(());
    }

    let input_path = Path::new(&args[1]);

    if input_path.is_file() {
        scan_single_file(input_path)?;
    } else if input_path.is_dir() {
        scan_directory(input_path)?;
    } else {
        eprintln!("Error: Input must be an ESP file or directory");
        eprintln!("  Path: {}", input_path.display());
        std::process::exit(1);
    }

    logging::print_cargo_style_summary();
    Ok(())
}

fn print_usage(program_name: &str) {
    eprintln!("Usage: {} <file.esp|directory>", program_name);
    eprintln!("       {} --help", program_name);
}

fn print_help(program_name: &str) {
    println!("ESP Scanner v{}", env!("CARGO_PKG_VERSION"));
    println!("Compliance scanning for ESP (Endpoint State Policy) files\n");
    println!("USAGE:");
    println!("    {} <file.esp>       Scan single ESP file", program_name);
    println!(
        "    {} <directory>      Scan all ESP files in directory",
        program_name
    );
    println!(
        "    {} --help           Show this help message\n",
        program_name
    );

    println!("EXAMPLES:");
    println!("    {} policy.esp", program_name);
    println!("    {} /etc/esp/policies/", program_name);
}

/// Convert PipelineResult AST to scanner types
fn convert_ast_to_scanner_types(
    pipeline_result: &pipeline::PipelineResult,
) -> Result<
    (
        Vec<VariableDeclaration>,
        Vec<StateDeclaration>,
        Vec<ObjectDeclaration>,
        Vec<RuntimeOperation>,
        Vec<SetOperation>,
        CriteriaRoot,
        MetaDataBlock,
    ),
    Box<dyn std::error::Error>,
> {
    let ast = &pipeline_result.ast;

    // Metadata: field.name not field.key
    let metadata = if let Some(meta) = &ast.metadata {
        let mut fields = std::collections::HashMap::new();
        for field in &meta.fields {
            fields.insert(field.name.clone(), field.value.clone());
        }
        MetaDataBlock { fields }
    } else {
        MetaDataBlock::default()
    };

    // Variables
    let variables: Vec<VariableDeclaration> = ast
        .definition
        .variables
        .iter()
        .map(|v| VariableDeclaration {
            name: v.name.clone(),
            data_type: v.data_type,
            initial_value: v.initial_value.clone(),
        })
        .collect();

    // States
    let states: Vec<StateDeclaration> = ast
        .definition
        .states
        .iter()
        .map(|s| {
            let fields: Vec<StateField> = s
                .fields
                .iter()
                .map(|f| StateField {
                    name: f.name.clone(),
                    data_type: f.data_type,
                    operation: f.operation,
                    value: f.value.clone(),
                    entity_check: f.entity_check.clone(),
                })
                .collect();

            StateDeclaration {
                identifier: s.id.clone(),
                fields,
                record_checks: s.record_checks.clone(),
                is_global: s.is_global,
            }
        })
        .collect();

    // Objects
    let objects: Vec<ObjectDeclaration> = ast
        .definition
        .objects
        .iter()
        .map(|o| ObjectDeclaration {
            identifier: o.id.clone(),
            elements: o.elements.clone(),
            is_global: o.is_global,
        })
        .collect();

    // Runtime operations
    let runtime_operations: Vec<RuntimeOperation> = ast
        .definition
        .runtime_operations
        .iter()
        .map(|r| RuntimeOperation {
            target_variable: r.target_variable.clone(),
            operation_type: r.operation_type,
            parameters: r.parameters.clone(),
        })
        .collect();

    // Sets: field is set_operations not sets!
    let sets: Vec<SetOperation> = ast
        .definition
        .set_operations
        .iter()
        .map(|s| SetOperation {
            set_id: s.set_id.clone(),
            operation: s.operation,
            operands: s.operands.clone(),
            filter: s.filter.clone(),
        })
        .collect();

    // Build CriteriaRoot tree structure instead of flattening
    let mut node_id_counter = 1;
    let criteria_root =
        build_criteria_root_from_ast(&ast.definition.criteria, &mut node_id_counter)?;

    Ok((
        variables,
        states,
        objects,
        runtime_operations,
        sets,
        criteria_root,
        metadata,
    ))
}

/// Build CriteriaRoot tree structure from compiler AST
/// This preserves the hierarchical CRI/CTN structure
fn build_criteria_root_from_ast(
    criteria_nodes: &[esp_compiler::grammar::ast::nodes::CriteriaNode],
    node_id_counter: &mut usize,
) -> Result<CriteriaRoot, Box<dyn std::error::Error>> {
    let mut trees = Vec::new();

    // Convert each CRI block to a CriteriaTree
    for cri_node in criteria_nodes {
        let tree = convert_criteria_node_to_tree(cri_node, node_id_counter)?;
        trees.push(tree);
    }

    Ok(CriteriaRoot {
        trees,
        root_logical_op: LogicalOp::And, // Default: combine top-level CRIs with AND
    })
}

/// Convert a compiler CriteriaNode to scanner CriteriaTree
fn convert_criteria_node_to_tree(
    cri_node: &esp_compiler::grammar::ast::nodes::CriteriaNode,
    node_id_counter: &mut usize,
) -> Result<CriteriaTree, Box<dyn std::error::Error>> {
    use esp_compiler::grammar::ast::nodes::CriteriaContent;

    let mut children = Vec::new();

    // Process all content items
    for content in &cri_node.content {
        match content {
            CriteriaContent::Criterion(ctn_node) => {
                // Leaf node: CTN
                let node_id = *node_id_counter;
                *node_id_counter += 1;

                let mut declaration = convert_ctn_to_declaration(ctn_node)?;
                // Assign the node_id to the declaration
                declaration.ctn_node_id = Some(node_id);

                children.push(CriteriaTree::Criterion {
                    declaration,
                    node_id,
                });
            }
            CriteriaContent::Criteria(nested_cri) => {
                // Recursive: nested CRI block
                let nested_tree = convert_criteria_node_to_tree(nested_cri, node_id_counter)?;
                children.push(nested_tree);
            }
        }
    }

    // If this CRI has only one child and it's already a Block or Criterion, unwrap it
    if children.len() == 1 {
        return Ok(children.into_iter().next().unwrap());
    }

    // Create Block node with proper logical operator
    let logical_op = match cri_node.logical_op {
        esp_compiler::grammar::ast::nodes::LogicalOp::And => LogicalOp::And,
        esp_compiler::grammar::ast::nodes::LogicalOp::Or => LogicalOp::Or,
    };

    Ok(CriteriaTree::Block {
        logical_op,
        negate: cri_node.negate,
        children,
    })
}

/// Convert compiler CriterionNode to scanner CriterionDeclaration
fn convert_ctn_to_declaration(
    ctn_node: &esp_compiler::grammar::ast::nodes::CriterionNode,
) -> Result<CriterionDeclaration, Box<dyn std::error::Error>> {
    // Convert local states
    let local_states: Vec<StateDeclaration> = ctn_node
        .local_states
        .iter()
        .map(|ls| {
            let fields: Vec<StateField> = ls
                .fields
                .iter()
                .map(|f| StateField {
                    name: f.name.clone(),
                    data_type: f.data_type,
                    operation: f.operation,
                    value: f.value.clone(),
                    entity_check: f.entity_check.clone(),
                })
                .collect();

            StateDeclaration {
                identifier: ls.id.clone(),
                fields,
                record_checks: ls.record_checks.clone(),
                is_global: false,
            }
        })
        .collect();

    // Convert local object
    let local_object = ctn_node.local_object.as_ref().map(|lo| ObjectDeclaration {
        identifier: lo.id.clone(),
        elements: lo.elements.clone(),
        is_global: false,
    });

    Ok(CriterionDeclaration {
        criterion_type: ctn_node.criterion_type.clone(),
        test: ctn_node.test.clone(),
        state_refs: ctn_node.state_refs.clone(),
        object_refs: ctn_node.object_refs.clone(),
        local_states,
        local_object,
        ctn_node_id: None, // Will be assigned during tree construction
    })
}

fn scan_single_file(file_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    let file_path_str = file_path.display().to_string();
    logging::set_file_context(file_path.to_path_buf(), 1);

    log_info!("Scanning ESP file", "path" => &file_path_str);

    // Phase 1: Compile
    log_info!("Phase 1: Compiling ESP file");
    let pipeline_result = pipeline::process_file(&file_path_str).map_err(|e| {
        log_error!(
            esp_compiler::logging::codes::file_processing::FILE_NOT_FOUND,
            "ESP compilation failed",
            "error" => e.to_string()
        );
        logging::clear_file_context();
        format!("Compilation failed: {}", e)
    })?;

    log_success!(
        esp_compiler::logging::codes::success::FILE_PROCESSING_SUCCESS,
        "ESP compilation successful"
    );

    // Phase 2: Create execution context
    log_info!("Phase 2: Creating execution context");

    // FIXED: Now receives CriteriaRoot instead of Vec<CriterionDeclaration>
    let (variables, states, objects, runtime_operations, sets, criteria_root, metadata) =
        convert_ast_to_scanner_types(&pipeline_result)?;

    // FIXED: Use new constructor that takes CriteriaRoot
    let mut resolution_context = ResolutionContext::from_ast_with_criteria_root(
        variables,
        states,
        objects,
        runtime_operations,
        sets,
        criteria_root,
        metadata,
    );

    let mut resolution_engine = ResolutionEngine::new();
    let execution_context = resolution_engine
        .resolve_context(&mut resolution_context)
        .map_err(|e| {
            log_error!(
                esp_compiler::logging::codes::system::INTERNAL_ERROR,
                "Failed to create execution context",
                "error" => e.to_string()
            );
            logging::clear_file_context();
            format!("Resolution failed: {}", e)
        })?;

    log_success!(
        esp_compiler::logging::codes::success::SEMANTIC_ANALYSIS_COMPLETE,
        "Execution context created",
        "criteria_count" => execution_context.count_criteria()
    );

    // Phase 3: Create scanner registry
    log_info!("Phase 3: Initializing scanner registry");
    let registry = esp_scanner_sdk::create_scanner_registry().map_err(|e| {
        log_error!(
            esp_compiler::logging::codes::system::INTERNAL_ERROR,
            "Failed to create scanner registry",
            "error" => e.to_string()
        );
        logging::clear_file_context();
        format!("Registry creation failed: {}", e)
    })?;

    let stats = registry.get_statistics();
    log_info!(
        "Registry initialized",
        "strategies" => stats.total_ctn_types,
        "healthy" => stats.registry_health.is_healthy()
    );

    // Phase 4: Execute scan
    log_info!("Phase 4: Executing compliance scan");
    let mut engine = ExecutionEngine::new(execution_context, Arc::new(registry));
    let scan_result = engine.execute().map_err(|e| {
        log_error!(
            esp_compiler::logging::codes::system::INTERNAL_ERROR,
            "Scan execution failed",
            "error" => e.to_string()
        );
        logging::clear_file_context();
        format!("Execution failed: {}", e)
    })?;

    let duration = start.elapsed();

    // Phase 5: Report
    let status = if scan_result.results.passed {
        "COMPLIANT"
    } else {
        "NON-COMPLIANT"
    };
    println!("\n=== Scan Results ===");
    println!("Status: {}", status);
    println!(
        "Total Criteria: {}",
        scan_result.results.check.total_criteria
    );
    println!("Passed: {}", scan_result.results.check.passed_criteria);
    println!("Failed: {}", scan_result.results.check.failed_criteria);
    println!(
        "Pass Rate: {:.1}%",
        scan_result.results.check.pass_percentage
    );
    println!("Findings: {}", scan_result.results.findings.len());
    println!("Duration: {:.2}s", duration.as_secs_f64());

    let json = scan_result.to_json()?;
    std::fs::write("scan_result.json", &json)?;
    println!("\n[OK] Results saved to: scan_result.json");

    if scan_result.results.passed {
        log_success!(
            esp_compiler::logging::codes::success::STRUCTURAL_VALIDATION_COMPLETE,
            "Compliance scan passed",
            "duration_ms" => duration.as_millis(),
            "criteria" => scan_result.results.check.total_criteria
        );
    } else {
        log_error!(
            esp_compiler::logging::codes::structural::INCOMPLETE_DEFINITION_STRUCTURE,
            "Compliance scan failed",
            "failed_criteria" => scan_result.results.check.failed_criteria,
            "findings" => scan_result.results.findings.len()
        );
    }

    logging::clear_file_context();

    if !scan_result.results.passed {
        std::process::exit(1);
    }

    Ok(())
}

fn scan_directory(dir_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    log_info!("Starting batch directory scan", "path" => dir_path.display().to_string());

    let esp_files = discover_esp_files(dir_path)?;
    if esp_files.is_empty() {
        println!("No ESP files found in directory: {}", dir_path.display());
        return Ok(());
    }

    log_info!("Discovered ESP files", "count" => esp_files.len(), "directory" => dir_path.display().to_string());
    println!("Scanning {} ESP files...", esp_files.len());

    let registry = esp_scanner_sdk::create_scanner_registry().map_err(|e| {
        log_error!(esp_compiler::logging::codes::system::INTERNAL_ERROR, "Failed to create scanner registry", "error" => e.to_string());
        format!("Registry creation failed: {}", e)
    })?;
    let registry = Arc::new(registry);

    let mut successful_scans = 0;
    let mut failed_scans = 0;
    let mut compliant_scans = 0;
    let mut non_compliant_scans = 0;
    let mut all_results = Vec::new();

    for (file_id, esp_file) in esp_files.iter().enumerate() {
        let file_id = file_id + 1;
        println!(
            "\n[{}/{}] Scanning: {}",
            file_id,
            esp_files.len(),
            esp_file.display()
        );
        logging::set_file_context(esp_file.clone(), file_id);

        match scan_file_for_batch(esp_file, registry.clone()) {
            Ok(scan_result) => {
                successful_scans += 1;
                if scan_result.results.passed {
                    compliant_scans += 1;
                    println!(
                        "  ✓ COMPLIANT ({} criteria)",
                        scan_result.results.check.total_criteria
                    );
                } else {
                    non_compliant_scans += 1;
                    println!(
                        "  ✗ NON-COMPLIANT ({} findings)",
                        scan_result.results.findings.len()
                    );
                }
                all_results.push(scan_result);
            }
            Err(e) => {
                failed_scans += 1;
                println!("  ✗ FAILED: {}", e);
                log_error!(esp_compiler::logging::codes::system::INTERNAL_ERROR, "File scan failed", "file" => esp_file.display().to_string(), "error" => e.to_string());
            }
        }

        logging::clear_file_context();
    }

    let duration = start.elapsed();

    println!("\n=== Batch Scan Summary ===");
    println!("Directory: {}", dir_path.display());
    println!("Files Scanned: {}", esp_files.len());
    println!("Successful: {}", successful_scans);
    println!("Failed: {}", failed_scans);
    println!("Compliant: {}", compliant_scans);
    println!("Non-Compliant: {}", non_compliant_scans);
    println!("Duration: {:.2}s", duration.as_secs_f64());

    let json = serde_json::to_string_pretty(&all_results)?;
    std::fs::write("batch_results.json", &json)?;
    println!("\n[OK] Results saved to: batch_results.json");

    log_success!(esp_compiler::logging::codes::success::FILE_PROCESSING_SUCCESS, "Batch scan completed", "total_files" => esp_files.len(), "successful" => successful_scans, "compliant" => compliant_scans, "duration_ms" => duration.as_millis());

    if failed_scans > 0 || non_compliant_scans > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn scan_file_for_batch(
    file_path: &Path,
    registry: Arc<esp_scanner_base::strategies::CtnStrategyRegistry>,
) -> Result<esp_scanner_base::results::ScanResult, Box<dyn std::error::Error>> {
    let file_path_str = file_path.display().to_string();
    let pipeline_result =
        pipeline::process_file(&file_path_str).map_err(|e| format!("Compilation failed: {}", e))?;

    // FIXED: Now receives CriteriaRoot
    let (variables, states, objects, runtime_operations, sets, criteria_root, metadata) =
        convert_ast_to_scanner_types(&pipeline_result)?;

    // FIXED: Use new constructor
    let mut resolution_context = ResolutionContext::from_ast_with_criteria_root(
        variables,
        states,
        objects,
        runtime_operations,
        sets,
        criteria_root,
        metadata,
    );

    let mut resolution_engine = ResolutionEngine::new();
    let execution_context = resolution_engine
        .resolve_context(&mut resolution_context)
        .map_err(|e| format!("Resolution failed: {}", e))?;

    let mut engine = ExecutionEngine::new(execution_context, registry);
    let scan_result = engine
        .execute()
        .map_err(|e| format!("Execution failed: {}", e))?;

    Ok(scan_result)
}

fn discover_esp_files(dir_path: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut esp_files = Vec::new();
    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "esp" {
                    esp_files.push(path);
                }
            }
        }
    }
    esp_files.sort();
    Ok(esp_files)
}
