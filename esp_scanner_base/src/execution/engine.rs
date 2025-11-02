//! # Execution Engine
//!
//! Orchestrates TEST-driven compliance validation with CTN contracts and tree traversal.
use crate::execution::deferred_ops;
use crate::execution::behavior::{extract_behavior_hints, BehaviorHints};
use crate::ffi::log_consumer_debug;
use crate::results::{
    ComplianceFinding, FindingSeverity, HostContext, IcsMetadata, ResultGenerationError,
    ScanResult, UserContext,
};
use crate::strategies::{
    CollectedData, ComplianceStatus, CtnContract, CtnExecutionResult, CtnStrategyRegistry,
};
use crate::types::common::{LogicalOp, ResolvedValue};
use crate::types::criterion::CtnNodeId;
use crate::types::execution_context::{
    ExecutableCriteriaTree, ExecutableCriterion, ExecutableObject, ExecutionContext,
};
use crate::execution::comparisons::{string, ComparisonExt};
use crate::types::FilterAction;
use crate::CtnExecutionError;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
/// Main execution engine that orchestrates compliance scanning
pub struct ExecutionEngine {
    context: ExecutionContext,
    registry: Arc<CtnStrategyRegistry>,
    start_time: Option<Instant>,
}
impl ExecutionEngine {
    /// Create with strategy registry
    pub fn new(context: ExecutionContext, registry: Arc<CtnStrategyRegistry>) -> Self {
        Self {
            context,
            registry,
            start_time: None,
        }
    }

    /// Main execution entry point
    /// Executes the entire criteria tree and produces a complete scan result
    pub fn execute(&mut self) -> Result<ScanResult, ExecutionError> {
        // Validate execution context before starting
        self.context
            .validate()
            .map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: "context_validation".to_string(),
                reason: e,
            })?;

        // Execute the criteria tree recursively
        let tree_result = self.execute_tree(&self.context.criteria_tree.clone())?;

        // Calculate flat statistics from tree (for metrics/dashboards)
        let stats = tree_result.calculate_stats();

        // Convert tree results to findings
        let findings = self.tree_result_to_findings(&tree_result, vec![])?;

        // Extract metadata
        let ics_metadata = self.extract_ics_metadata()?;
        let host = HostContext::from_system();
        let user_context = UserContext::from_environment();

        // Generate scan ID
        let scan_id = format!(
            "scan_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );

        // Create scan result
        let mut scan_result = ScanResult::new(scan_id, ics_metadata, host, user_context);

        // Update criteria counts with flat statistics
        scan_result.update_criteria_counts(stats.total, stats.passed, stats.failed, stats.errors);

        // Set findings
        scan_result.results.findings = findings;

        // Finalize calculates timestamps and sets check.status based on flat stats
        scan_result.finalize();

        // CRITICAL: Override with tree logic for overall pass/fail
        // This respects CRI OR/AND/NOT structure, done AFTER finalize()
        scan_result.results.passed = tree_result.status == ComplianceStatus::Pass;

        Ok(scan_result)
    }

    /// Extract ICS metadata from execution context
    fn extract_ics_metadata(&self) -> Result<IcsMetadata, ExecutionError> {
        let metadata =
            self.context
                .metadata
                .as_ref()
                .ok_or_else(|| ExecutionError::ExecutorFailed {
                    ctn_type: "metadata_extraction".to_string(),
                    reason: "Missing metadata in execution context".to_string(),
                })?;

        IcsMetadata::from_metadata_block(metadata).map_err(|e| ExecutionError::ExecutorFailed {
            ctn_type: "metadata_extraction".to_string(),
            reason: format!("Failed to extract ICS metadata: {}", e),
        })
    }

    /// Recursive tree traversal with logical operator application
    fn execute_tree(
        &mut self,
        tree: &ExecutableCriteriaTree,
    ) -> Result<TreeResult, ExecutionError> {
        match tree {
            ExecutableCriteriaTree::Criterion(criterion) => {
                let start = Instant::now();

                // Execute CTN - returns CtnExecutionResult
                let ctn_execution_result = self.execute_single_criterion(criterion)?;

                // Wrap in CtnResult for tree tracking
                let ctn_result = CtnResult {
                    ctn_node_id: criterion.ctn_node_id,
                    criterion_type: criterion.criterion_type.clone(),
                    status: ctn_execution_result.status,
                    execution_result: ctn_execution_result,
                    execution_time_ms: start.elapsed().as_millis() as u64,
                };

                Ok(TreeResult {
                    status: ctn_result.status,
                    logical_op: None,
                    negated: false,
                    ctn_results: vec![ctn_result],
                    child_results: vec![],
                })
            }
            ExecutableCriteriaTree::Block {
                logical_op,
                negate,
                children,
            } => {
                // Recursive case: process children
                let mut child_results = Vec::new();

                for child in children {
                    let child_tree_result = self.execute_tree(child)?;
                    child_results.push(child_tree_result);
                }

                // Apply logical operator to child statuses
                let combined = self.apply_logical_op(&child_results, *logical_op);

                // Apply negation if present
                let final_status = if *negate { combined.negate() } else { combined };

                Ok(TreeResult {
                    status: final_status,
                    logical_op: Some(*logical_op),
                    negated: *negate,
                    ctn_results: vec![], // Empty at block level - no flattening
                    child_results,
                })
            }
        }
    }

    /// Apply logical operator to child tree results
    fn apply_logical_op(&self, children: &[TreeResult], op: LogicalOp) -> ComplianceStatus {
        if children.is_empty() {
            return ComplianceStatus::Error;
        }

        match op {
            LogicalOp::And => {
                // ALL children must pass
                if children.iter().all(|c| c.status == ComplianceStatus::Pass) {
                    ComplianceStatus::Pass
                } else if children.iter().any(|c| c.status == ComplianceStatus::Error) {
                    ComplianceStatus::Error
                } else {
                    ComplianceStatus::Fail
                }
            }
            LogicalOp::Or => {
                // ANY child passes = pass
                if children.iter().any(|c| c.status == ComplianceStatus::Pass) {
                    ComplianceStatus::Pass
                } else if children.iter().all(|c| c.status == ComplianceStatus::Error) {
                    ComplianceStatus::Error
                } else {
                    ComplianceStatus::Fail
                }
            }
        }
    }

    /// Execute a single criterion with timeout protection
    /// Execute a single criterion with timeout protection
    fn execute_single_criterion(
        &mut self,
        criterion: &ExecutableCriterion,
    ) -> Result<CtnExecutionResult, ExecutionError> {
        use std::time::Instant;

        const CTN_TIMEOUT_SECS: u64 = 30;
        let start = Instant::now();

        let _ = log_consumer_debug(
            "Starting CTN execution",
            &[
                ("ctn_type", &criterion.criterion_type),
                ("ctn_node_id", &criterion.ctn_node_id.to_string()),
            ],
        );

        // Get contract for this CTN type
        let contract = self
            .registry
            .get_ctn_contract(&criterion.criterion_type)
            .map_err(|e| ExecutionError::NoContractRegistered {
                ctn_type: criterion.criterion_type.clone(),
                reason: e.to_string(),
            })?;

        // Clone contract Arc to avoid borrow conflicts
        let contract_clone = Arc::clone(&contract);

        // Get collector for this CTN type
        let collector = self
            .registry
            .get_collector_for_ctn(&criterion.criterion_type)
            .map_err(|e| ExecutionError::NoCollectorRegistered {
                ctn_type: criterion.criterion_type.clone(),
                reason: e.to_string(),
            })?;

        // Check timeout after setup
        if start.elapsed().as_secs() > CTN_TIMEOUT_SECS {
            return Err(ExecutionError::ExecutorFailed {
                ctn_type: criterion.criterion_type.clone(),
                reason: format!(
                    "CTN execution exceeded timeout of {}s during setup",
                    CTN_TIMEOUT_SECS
                ),
            });
        }

        // Attempt batch collection if supported
        let mut collected_data =
            if collector.supports_batch_collection() && !criterion.objects.is_empty() {
                let _ = log_consumer_debug(
                    "Attempting batch collection",
                    &[
                        ("ctn_type", &criterion.criterion_type),
                        ("object_count", &criterion.objects.len().to_string()),
                    ],
                );

                let object_refs: Vec<&ExecutableObject> = criterion.objects.iter().collect();
                match collector.collect_batch(object_refs, &contract) {
                    Ok(batch_data) => {
                        let _ = log_consumer_debug(
                            "Batch collection successful",
                            &[
                                ("ctn_type", &criterion.criterion_type),
                                ("objects_collected", &batch_data.len().to_string()),
                            ],
                        );
                        batch_data
                    }
                    Err(e) => {
                        let _ = log_consumer_debug(
                            "Batch collection failed, falling back to individual",
                            &[
                                ("ctn_type", &criterion.criterion_type),
                                ("error", &e.to_string()),
                            ],
                        );
                        HashMap::new()
                    }
                }
            } else {
                HashMap::new()
            };

        // Individual collection for any objects not batch-collected
        for object in &criterion.objects {
            if !collected_data.contains_key(&object.identifier) {
                let data = self.collect_data_for_object(object, &contract)?;
                collected_data.insert(object.identifier.clone(), data);
            }
        }

        // Apply filters if any
        collected_data = self.apply_object_filters(collected_data, criterion, &contract)?;

        // Check timeout after collection
        if start.elapsed().as_secs() > CTN_TIMEOUT_SECS {
            return Err(ExecutionError::ExecutorFailed {
                ctn_type: criterion.criterion_type.clone(),
                reason: format!(
                    "CTN execution exceeded timeout of {}s during collection",
                    CTN_TIMEOUT_SECS
                ),
            });
        }

        // Execute deferred operations if any
        self.execute_deferred_operations_for_criterion(criterion, &collected_data)?;

        // Get executor AFTER mutable borrow completes
        let executor = self
            .registry
            .get_executor_for_ctn(&criterion.criterion_type)
            .map_err(|e| ExecutionError::NoExecutorRegistered {
                ctn_type: criterion.criterion_type.clone(),
                reason: e.to_string(),
            })?;

        // Execute validation using the cloned contract
        let result = executor.execute_with_contract(criterion, &collected_data, &contract_clone)?;

        // Check timeout after execution
        let elapsed = start.elapsed();
        if elapsed.as_secs() > CTN_TIMEOUT_SECS {
            return Err(ExecutionError::ExecutorFailed {
                ctn_type: criterion.criterion_type.clone(),
                reason: format!(
                    "CTN execution exceeded timeout of {}s during validation",
                    CTN_TIMEOUT_SECS
                ),
            });
        }

        let _ = log_consumer_debug(
            "CTN execution completed",
            &[
                ("ctn_type", &criterion.criterion_type),
                ("status", &format!("{:?}", result.status)),
                ("duration_ms", &elapsed.as_millis().to_string()),
            ],
        );

        Ok(result)
    }

    /// Convert CTN execution result to compliance finding
    fn ctn_result_to_finding(
        &self,
        ctn_result: &CtnExecutionResult,
        field_path: Vec<String>,
    ) -> Result<ComplianceFinding, ExecutionError> {
        // Extract failed field information from state results
        let mut expected_values = serde_json::Map::new();
        let mut actual_values = serde_json::Map::new();

        for state_result in &ctn_result.state_results {
            for field_result in &state_result.state_results {
                if !field_result.passed {
                    // Add to expected/actual maps
                    let expected_str = format!("{:?}", field_result.expected_value);
                    let actual_str = format!("{:?}", field_result.actual_value);

                    expected_values.insert(
                        field_result.field_name.clone(),
                        serde_json::Value::String(expected_str),
                    );
                    actual_values.insert(
                        field_result.field_name.clone(),
                        serde_json::Value::String(actual_str),
                    );
                }
            }
        }

        // Determine severity from CTN metadata
        let severity = match ctn_result.status {
            ComplianceStatus::Fail => FindingSeverity::High,
            ComplianceStatus::Error => FindingSeverity::Critical,
            _ => FindingSeverity::Medium,
        };

        // Build title and description
        let title = format!("{} validation failed", ctn_result.ctn_type);
        let description = ctn_result.message.clone();

        // Convert to JSON values
        let expected_json =
            serde_json::to_value(&expected_values).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: ctn_result.ctn_type.clone(),
                reason: format!("Failed to serialize expected values: {}", e),
            })?;
        let actual_json =
            serde_json::to_value(&actual_values).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: ctn_result.ctn_type.clone(),
                reason: format!("Failed to serialize actual values: {}", e),
            })?;

        // Truncate large values to prevent JSON bloat
        let expected_truncated = Self::truncate_large_values(&expected_json);
        let actual_truncated = Self::truncate_large_values(&actual_json);

        Ok(ComplianceFinding::auto_id(
            severity,
            title,
            description,
            expected_truncated,
            actual_truncated,
        )
        .with_field_path(field_path.join(" > ")))
    }

    /// Truncate large values in findings to prevent JSON bloat
    fn truncate_large_values(value: &serde_json::Value) -> serde_json::Value {
        const MAX_FIELD_LENGTH: usize = 200;

        match value {
            serde_json::Value::String(s) if s.len() > MAX_FIELD_LENGTH => {
                serde_json::Value::String(format!(
                    "{}... [truncated: {} total chars]",
                    &s[..MAX_FIELD_LENGTH],
                    s.len()
                ))
            }
            serde_json::Value::Object(map) => {
                let truncated: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::truncate_large_values(v)))
                    .collect();
                serde_json::Value::Object(truncated)
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter().map(|v| Self::truncate_large_values(v)).collect(),
            ),
            other => other.clone(),
        }
    }

    /// Collect data for a single object
    fn collect_data_for_object(
        &self,
        object: &ExecutableObject,
        contract: &Arc<CtnContract>,
    ) -> Result<CollectedData, ExecutionError> {
        let collector = self
            .registry
            .get_collector_for_ctn(&contract.ctn_type)
            .map_err(|e| ExecutionError::NoCollectorRegistered {
                ctn_type: contract.ctn_type.clone(),
                reason: e.to_string(),
            })?;

        // Extract behavior hints from the object
        let hints = extract_behavior_hints(object);

        // Call the new method with hints
        collector
            .collect_for_ctn_with_hints(object, contract, &hints)
            .map_err(|e| ExecutionError::DataCollectionFailed {
                object_id: object.identifier.clone(),
                reason: e.to_string(),
            })
    }

    /// Apply object filters to collected data
    fn apply_object_filters(
        &self,
        mut collected: HashMap<String, CollectedData>,
        criterion: &ExecutableCriterion,
        contract: &Arc<CtnContract>,
    ) -> Result<HashMap<String, CollectedData>, ExecutionError> {
        for object in &criterion.objects {
            let filters = object.get_filters();
            eprintln!("🔍 Object '{}' has {} filters", object.identifier, filters.len());
            if filters.is_empty() {
                continue;
            }

            for filter in filters {
                if let Some(data) = collected.get(&object.identifier) {
                    let should_keep =
                        self.evaluate_filter_against_global_states(filter, data, contract)?;

                    let should_remove = match filter.action {
                        FilterAction::Include => !should_keep,
                        FilterAction::Exclude => should_keep,
                    };

                    if should_remove {
                        collected.remove(&object.identifier);
                    }
                }
            }
        }

        Ok(collected)
    }

    /// Evaluate filter against GLOBAL states (not CTN-local states)
fn evaluate_filter_against_global_states(
    &self,
    filter: &crate::types::filter::ResolvedFilterSpec,
    data: &CollectedData,
    contract: &CtnContract,
) -> Result<bool, ExecutionError> {
    // AND logic: ALL state refs must match for filter to pass
    for state_ref in &filter.state_refs {
        // Look up in GLOBAL states, not local states
        let state = self.context.global_states
            .get(state_ref)
            .ok_or_else(|| ExecutionError::StateNotFound {
                state_id: state_ref.clone(),
            })?;

        // Convert ResolvedState to ExecutableState fields for evaluation
        for field in &state.resolved_fields {
            // Map state field name to data field name using contract
            let data_field_name = contract
                .field_mappings
                .validation_mappings
                .state_to_data
                .get(&field.name)
                .unwrap_or(&field.name);

            // Get collected value for this field
            if let Some(collected_value) = data.get_field(data_field_name) {
                let matches = self.compare_for_filter(
                    collected_value,
                    &field.value,
                    field.operation,
                )?;

                // Short-circuit: If any field fails, entire filter fails
                if !matches {
                    return Ok(false);
                }
            } else {
                // Field not collected - treat as filter failure
                return Ok(false);
            }
        }
    }
    
    // All states matched
    Ok(true)
}

    /// Evaluate a single filter against collected data
/// NOW SUPPORTS ALL ICS OPERATIONS
fn evaluate_filter(
    &self,
    filter: &crate::types::filter::ResolvedFilterSpec,
    data: &CollectedData,
    contract: &CtnContract,
    available_states: &[crate::types::execution_context::ExecutableState],
) -> Result<bool, ExecutionError> {
    // AND logic: ALL state refs must match for filter to pass
    for state_ref in &filter.state_refs {
        let state = available_states
            .iter()
            .find(|s| s.identifier == *state_ref)
            .ok_or_else(|| ExecutionError::StateNotFound {
                state_id: state_ref.clone(),
            })?;

        // All fields in the state must match (AND within state)
        for field in &state.fields {
            // Map state field name to data field name using contract
            let data_field_name = contract
                .field_mappings
                .validation_mappings
                .state_to_data
                .get(&field.name)
                .unwrap_or(&field.name);

            // Get collected value for this field
            if let Some(collected_value) = data.get_field(data_field_name) {
                // ✅ FIX: Use comprehensive comparison logic
                let matches = self.compare_for_filter(
                    collected_value,
                    &field.value,
                    field.operation,
                )?;

                // Short-circuit: If any field fails, entire filter fails
                if !matches {
                    return Ok(false);
                }
            } else {
                // Field not collected - treat as filter failure
                return Ok(false);
            }
        }
    }
    
    // All states matched
    Ok(true)
}

/// Compare values for filter evaluation with full operation support
/// Similar to executor comparison but returns Result<bool>
fn compare_for_filter(
    &self,
    actual: &ResolvedValue,
    expected: &ResolvedValue,
    operation: crate::types::common::Operation,
) -> Result<bool, ExecutionError> {
    use crate::types::common::Operation;
    
    let result = match (actual, expected, operation) {
        // ============================================================
        // String operations (all supported)
        // ============================================================
        (ResolvedValue::String(a), ResolvedValue::String(e), op) => {
            string::compare(a, e, op).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: "filter_evaluation".to_string(),
                reason: format!("String comparison failed: {}", e),
            })?
        }
        
       // Integer operations - direct comparison
        (ResolvedValue::Integer(a), ResolvedValue::Integer(e), Operation::Equals) => a == e,
        (ResolvedValue::Integer(a), ResolvedValue::Integer(e), Operation::NotEqual) => a != e,
        (ResolvedValue::Integer(a), ResolvedValue::Integer(e), Operation::GreaterThan) => a > e,
        (ResolvedValue::Integer(a), ResolvedValue::Integer(e), Operation::LessThan) => a < e,
        (ResolvedValue::Integer(a), ResolvedValue::Integer(e), Operation::GreaterThanOrEqual) => a >= e,
        (ResolvedValue::Integer(a), ResolvedValue::Integer(e), Operation::LessThanOrEqual) => a <= e,
        
        // Float operations - direct comparison
        (ResolvedValue::Float(a), ResolvedValue::Float(e), Operation::Equals) => a == e,
        (ResolvedValue::Float(a), ResolvedValue::Float(e), Operation::NotEqual) => a != e,
        (ResolvedValue::Float(a), ResolvedValue::Float(e), Operation::GreaterThan) => a > e,
        (ResolvedValue::Float(a), ResolvedValue::Float(e), Operation::LessThan) => a < e,
        (ResolvedValue::Float(a), ResolvedValue::Float(e), Operation::GreaterThanOrEqual) => a >= e,
        (ResolvedValue::Float(a), ResolvedValue::Float(e), Operation::LessThanOrEqual) => a <= e,
        
        // Boolean operations - direct comparison  
        (ResolvedValue::Boolean(a), ResolvedValue::Boolean(e), Operation::Equals) => a == e,
        (ResolvedValue::Boolean(a), ResolvedValue::Boolean(e), Operation::NotEqual) => a != e,

        // Version comparisons - use ResolvedValue's compare_with trait
        (ResolvedValue::Version(_), ResolvedValue::Version(_), _) => {
            actual.compare_with(expected, operation).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: "filter_evaluation".to_string(),
                reason: format!("Version comparison failed: {}", e),
            })?
        }

        // Collection operations - use collection module
        (ResolvedValue::Collection(a), ResolvedValue::Collection(e), op) => {
            use crate::execution::comparisons::collection;
            collection::compare(a, e, op).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: "filter_evaluation".to_string(),
                reason: format!("Collection comparison failed: {}", e),
            })?
        }
        
        // ============================================================
        // EVR string comparisons (RPM-style versions)
        // ============================================================
        (ResolvedValue::EvrString(a), ResolvedValue::EvrString(e), op) => {
            use crate::execution::comparisons::evr;
            evr::compare(a, e, op).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: "filter_evaluation".to_string(),
                reason: format!("EVR comparison failed: {}", e),
            })?
        }
        
        // ============================================================
        // Binary operations (contains operation for byte sequences)
        // ============================================================
        (ResolvedValue::Binary(a), ResolvedValue::Binary(e), op) => {
            use crate::execution::comparisons::binary;
            binary::compare(a, e, op).map_err(|e| ExecutionError::ExecutorFailed {
                ctn_type: "filter_evaluation".to_string(),
                reason: format!("Binary comparison failed: {}", e),
            })?
        }
        
        // ============================================================
        // Type mismatch or unsupported operation
        // ============================================================
        _ => {
            return Err(ExecutionError::ExecutorFailed {
                ctn_type: "filter_evaluation".to_string(),
                reason: format!(
                    "Type mismatch in filter: actual={:?}, expected={:?}, operation={:?}",
                    actual, expected, operation
                ),
            })
        }
    };
    
    Ok(result)
}

    /// Execute deferred operations for this criterion ONLY
    /// FIXED: Only executes operations relevant to this CTN's collected objects
    fn execute_deferred_operations_for_criterion(
        &mut self,
        criterion: &ExecutableCriterion,
        collected_data: &HashMap<String, CollectedData>,
    ) -> Result<(), ExecutionError> {
        // Simply call the deferred ops module - it will execute ALL deferred operations
        // This is correct because deferred operations update global context variables
        // that are visible to all subsequent CTNs
        deferred_ops::execute_all_deferred_operations(
            &mut self.context,
            &self.registry,
            collected_data,
        )?;

        Ok(())
    }

    /// Convert tree result to findings with logical paths
    fn tree_result_to_findings(
        &self,
        tree_result: &TreeResult,
        current_path: Vec<String>,
    ) -> Result<Vec<ComplianceFinding>, ExecutionError> {
        let mut findings = Vec::new();

        // Add current level to path if it's a block
        let path = if let Some(logical_op) = tree_result.logical_op {
            let mut new_path = current_path.clone();
            let block_name = if tree_result.negated {
                format!("CRI_{}_NOT", logical_op_to_string(logical_op))
            } else {
                format!("CRI_{}", logical_op_to_string(logical_op))
            };
            new_path.push(block_name);
            new_path
        } else {
            current_path.clone()
        };

        // If this is a leaf node (has CTN results), process them
        if !tree_result.ctn_results.is_empty() {
            for ctn_result in &tree_result.ctn_results {
                if ctn_result.status != ComplianceStatus::Pass {
                    let mut finding_path = path.clone();
                    finding_path.push(format!("CTN_{}", ctn_result.criterion_type));

                    // Pass the execution_result (CtnExecutionResult), not the wrapper (CtnResult)
                    let finding =
                        self.ctn_result_to_finding(&ctn_result.execution_result, finding_path)?;
                    findings.push(finding);
                }
            }
        }

        // Recurse into child trees
        for child in &tree_result.child_results {
            let child_findings = self.tree_result_to_findings(child, path.clone())?;
            findings.extend(child_findings);
        }

        Ok(findings)
    }

    /// Extract failure details from CTN execution result
    fn extract_failure_details(
        &self,
        ctn_result: &CtnResult,
    ) -> (serde_json::Value, serde_json::Value, String) {
        let exec_result = &ctn_result.execution_result;

        // Build expected from state results
        let mut expected_map = serde_json::Map::new();
        let mut actual_map = serde_json::Map::new();
        let mut description_parts = vec![exec_result.message.clone()];

        for state_result in &exec_result.state_results {
            if !state_result.combined_result {
                for field_result in &state_result.state_results {
                    if !field_result.passed {
                        expected_map.insert(
                            field_result.field_name.clone(),
                            serde_json::json!(format!("{:?}", field_result.expected_value)),
                        );
                        actual_map.insert(
                            field_result.field_name.clone(),
                            serde_json::json!(format!("{:?}", field_result.actual_value)),
                        );

                        description_parts.push(format!(
                            "Field '{}': expected {:?}, got {:?}",
                            field_result.field_name,
                            field_result.expected_value,
                            field_result.actual_value
                        ));
                    }
                }
            }
        }

        (
            serde_json::Value::Object(expected_map),
            serde_json::Value::Object(actual_map),
            description_parts.join("\n"),
        )
    }

    /// Determine severity from ICS metadata
    fn determine_severity_from_metadata(&self) -> Result<FindingSeverity, ExecutionError> {
        if let Some(metadata) = &self.context.metadata {
            if let Some(criticality) = metadata.fields.get("criticality") {
                return Ok(match criticality.to_lowercase().as_str() {
                    "critical" => FindingSeverity::Critical,
                    "high" => FindingSeverity::High,
                    "medium" => FindingSeverity::Medium,
                    "low" => FindingSeverity::Low,
                    "info" => FindingSeverity::Info,
                    _ => FindingSeverity::Medium,
                });
            }
        }
        Ok(FindingSeverity::Medium)
    }

    /// Generate unique scan ID
    fn generate_scan_id(&self) -> String {
        format!("scan_{}", chrono::Utc::now().timestamp())
    }
}
// ============================================================================
// Result Types
// ============================================================================
/// CTN execution result with tree context
#[derive(Debug, Clone)]
pub struct CtnResult {
    pub ctn_node_id: CtnNodeId,
    pub criterion_type: String,
    pub status: ComplianceStatus,
    pub execution_result: CtnExecutionResult,
    pub execution_time_ms: u64,
}
/// Tree traversal result (internal)
#[derive(Debug, Clone)]
struct TreeResult {
    pub status: ComplianceStatus,
    pub logical_op: Option<LogicalOp>,
    pub negated: bool,
    pub ctn_results: Vec<CtnResult>,
    pub child_results: Vec<TreeResult>,
}
impl TreeResult {
    fn calculate_stats(&self) -> TreeStats {
        let mut stats = TreeStats::default();
        for ctn in &self.ctn_results {
            stats.total += 1;
            match ctn.status {
                ComplianceStatus::Pass => stats.passed += 1,
                ComplianceStatus::Fail => stats.failed += 1,
                ComplianceStatus::Error => stats.errors += 1,
                _ => {}
            }
        }

        // Recurse into children
        for child in &self.child_results {
            let child_stats = child.calculate_stats();
            stats.total += child_stats.total;
            stats.passed += child_stats.passed;
            stats.failed += child_stats.failed;
            stats.errors += child_stats.errors;
        }

        stats
    }
}
#[derive(Debug, Default)]
struct TreeStats {
    total: u32,
    passed: u32,
    failed: u32,
    errors: u32,
}
// ============================================================================
// Error Types
// ============================================================================
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("No contract registered for CTN type '{ctn_type}': {reason}")]
    NoContractRegistered { ctn_type: String, reason: String },
    #[error("Contract validation failed for '{ctn_type}': {errors:?}")]
    ContractValidationFailed {
        ctn_type: String,
        errors: Vec<String>,
    },

    #[error("No collector registered for CTN type '{ctn_type}': {reason}")]
    NoCollectorRegistered { ctn_type: String, reason: String },

    #[error("Data collection failed for object '{object_id}': {reason}")]
    DataCollectionFailed { object_id: String, reason: String },

    #[error("Deferred operation failed: {operation} - {reason}")]
    DeferredOperationFailed { operation: String, reason: String },

    #[error("State '{state_id}' not found")]
    StateNotFound { state_id: String },

    #[error("Executor failed for CTN type '{ctn_type}': {reason}")]
    ExecutorFailed { ctn_type: String, reason: String },

    #[error("No executor registered for CTN type '{ctn_type}': {reason}")]
    NoExecutorRegistered { ctn_type: String, reason: String },

    #[error("Result generation failed: {0}")]
    ResultGenerationError(#[from] ResultGenerationError),
}

impl From<CtnExecutionError> for ExecutionError {
    fn from(err: CtnExecutionError) -> Self {
        ExecutionError::ExecutorFailed {
            ctn_type: "unknown".to_string(),
            reason: err.to_string(),
        }
    }
}
// ============================================================================
// Helper Functions
// ============================================================================
fn logical_op_to_string(op: LogicalOp) -> &'static str {
    match op {
        LogicalOp::And => "AND",
        LogicalOp::Or => "OR",
    }
}
