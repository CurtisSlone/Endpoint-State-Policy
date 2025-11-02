// parser/runtime_operation/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::resolution::ResolutionError;
use crate::types::common::Value;
use crate::types::resolution_context::{DeferredValidation, ValidationType};
use crate::types::runtime_operation::{
    ArithmeticOperator, RunParameter, RuntimeOperation, RuntimeOperationType,
};

/// Enhanced context for runtime operation parsing with DAG support
#[derive(Debug, Clone)]
pub struct RuntimeOperationParsingContext {
    /// Parsing errors that are non-fatal
    pub parsing_errors: Vec<String>,
    /// Deferred validations to be processed later
    pub deferred_validations: Vec<DeferredValidation>,
    /// Variable references found during parsing (source, target, context)
    pub variable_references: Vec<(String, String, String)>,
    /// Object references found during parsing (source, target, context)
    pub object_references: Vec<(String, String, String)>,
    /// RUN operation target variables (for computed variable tracking)
    pub run_operation_targets: Vec<String>,
}

impl RuntimeOperationParsingContext {
    pub fn new() -> Self {
        Self {
            parsing_errors: Vec::new(),
            deferred_validations: Vec::new(),
            variable_references: Vec::new(),
            object_references: Vec::new(),
            run_operation_targets: Vec::new(),
        }
    }

    /// Add a parsing error (non-fatal)
    pub fn add_parsing_error(&mut self, error: String) {
        self.parsing_errors.push(error);
    }

    /// Add deferred variable reference validation
    pub fn defer_variable_reference(&mut self, source: String, target: String, context: String) {
        self.variable_references
            .push((source.clone(), target.clone(), context.clone()));
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::VariableReference,
            source_symbol: source,
            target_symbol: target,
            context,
        });
    }

    /// Add deferred object field extraction validation
    pub fn defer_object_field_extraction(
        &mut self,
        source: String,
        target: String,
        context: String,
    ) {
        self.object_references
            .push((source.clone(), target.clone(), context.clone()));
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::ObjectFieldExtraction,
            source_symbol: source,
            target_symbol: target,
            context,
        });
    }

    /// Add deferred RUN operation target validation
    pub fn defer_run_operation_target(&mut self, source: String, target: String, context: String) {
        self.run_operation_targets.push(target.clone());
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::RunOperationTarget,
            source_symbol: source,
            target_symbol: target,
            context,
        });
    }

    /// Apply collected data to ResolutionContext
    pub fn apply_to_resolution_context(
        self,
        resolution_context: &mut crate::types::resolution_context::ResolutionContext,
    ) {
        // Add parsing errors
        for error in self.parsing_errors {
            resolution_context.add_parsing_error(error);
        }

        // Add deferred validations
        for validation in self.deferred_validations {
            resolution_context.add_deferred_validation(validation);
        }

        // Track computed variables (RUN operation targets without initial values)
        for target_var in self.run_operation_targets {
            // Note: The actual determination of whether this is computed vs. reference initialization
            // happens when we process the variable declarations and see if they have initial values
            // For now, we just track that this variable is targeted by a RUN operation
        }
    }
}

/// Result of runtime operation parsing with collected context
#[derive(Debug)]
pub struct RuntimeOperationParsingResult {
    pub runtime_operations: Vec<RuntimeOperation>,
    pub context: RuntimeOperationParsingContext,
}

impl RuntimeOperationParsingResult {
    pub fn new(
        runtime_operations: Vec<RuntimeOperation>,
        context: RuntimeOperationParsingContext,
    ) -> Self {
        Self {
            runtime_operations,
            context,
        }
    }
}

/// Extract runtime operations from AST JSON with DAG-aware context collection
/// All runtime operations are global scope per ICS design
pub fn extract_runtime_operations_from_json_with_context(
    ast_json: &serde_json::Value,
) -> Result<RuntimeOperationParsingResult, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting runtime operation extraction from AST JSON with DAG context",
        &[("ast_is_object", &ast_json.is_object().to_string())],
    );

    let mut parsing_context = RuntimeOperationParsingContext::new();

    let definition = ast_json.get("definition").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "No 'definition' key found in AST JSON",
            &[],
        );
        ResolutionError::InvalidInput {
            message: "No definition found in AST".to_string(),
        }
    })?;

    let empty_vec = Vec::new();
    let runtime_operations_array = definition
        .get("runtime_operations")
        .and_then(|runs| runs.as_array())
        .unwrap_or(&empty_vec); // No runtime operations is valid, return empty vec

    let _ = log_consumer_debug(
        "Found runtime operations array",
        &[(
            "runtime_operation_count",
            &runtime_operations_array.len().to_string(),
        )],
    );

    let mut runtime_operations = Vec::new();

    for (index, run_json) in runtime_operations_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing runtime operation",
            &[("index", &index.to_string())],
        );

        match parse_runtime_operation_from_json_with_context(run_json, &mut parsing_context) {
            Ok(runtime_operation) => {
                let _ = log_consumer_debug(
                    "Successfully parsed runtime operation",
                    &[
                        ("target_variable", &runtime_operation.target_variable),
                        ("operation_type", runtime_operation.operation_type.as_str()),
                        (
                            "parameter_count",
                            &runtime_operation.parameters.len().to_string(),
                        ),
                        (
                            "has_variable_refs",
                            &runtime_operation.has_variable_references().to_string(),
                        ),
                        (
                            "object_dependencies",
                            &runtime_operation
                                .get_object_dependencies()
                                .len()
                                .to_string(),
                        ),
                    ],
                );
                runtime_operations.push(runtime_operation);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to parse runtime operation at index {}: {:?}",
                    index, e
                );
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[("index", &index.to_string())],
                );

                // For DAG resolution, continue parsing and collect error
                parsing_context.add_parsing_error(error_msg);
            }
        }
    }

    let _ = log_consumer_debug(
        "Runtime operation extraction completed with DAG context",
        &[
            ("total_extracted", &runtime_operations.len().to_string()),
            (
                "parsing_errors",
                &parsing_context.parsing_errors.len().to_string(),
            ),
            (
                "deferred_validations",
                &parsing_context.deferred_validations.len().to_string(),
            ),
            (
                "variable_references",
                &parsing_context.variable_references.len().to_string(),
            ),
            (
                "object_references",
                &parsing_context.object_references.len().to_string(),
            ),
        ],
    );

    Ok(RuntimeOperationParsingResult::new(
        runtime_operations,
        parsing_context,
    ))
}

/// Backward compatibility wrapper
pub fn extract_runtime_operations_from_json(
    ast_json: &serde_json::Value,
) -> Result<Vec<RuntimeOperation>, ResolutionError> {
    let result = extract_runtime_operations_from_json_with_context(ast_json)?;
    // Note: This loses the parsing context information, but maintains backward compatibility
    Ok(result.runtime_operations)
}

/// Parse a single runtime operation from JSON with DAG-aware context collection
/// EBNF: run_block ::= "RUN" space variable_name space operation_type statement_end run_parameters "RUN_END" statement_end
fn parse_runtime_operation_from_json_with_context(
    run_json: &serde_json::Value,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<RuntimeOperation, ResolutionError> {
    let target_variable = run_json
        .get("target_variable")
        .and_then(|tv| tv.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Runtime operation missing 'target_variable' field",
                &[],
            );
            ResolutionError::InvalidInput {
                message: "Runtime operation missing target_variable".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Parsing runtime operation with DAG context",
        &[("target_variable", target_variable)],
    );

    // Validate target variable identifier format (structural validation)
    if !is_valid_identifier(target_variable) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid target variable identifier format: '{}'",
                target_variable
            ),
            &[("target_variable", target_variable)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Invalid target variable identifier format: '{}'",
                target_variable
            ),
        });
    }

    // DEFER target variable existence validation - it might be a computed variable
    let _ = log_consumer_debug(
        "Deferring target variable existence validation",
        &[("target_variable", target_variable)],
    );

    parsing_context.defer_run_operation_target(
        target_variable.to_string(),
        target_variable.to_string(),
        format!("RUN operation target variable '{}'", target_variable),
    );

    let operation_type_str = run_json
        .get("operation_type")
        .and_then(|ot| ot.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Runtime operation '{}' missing 'operation_type' field",
                    target_variable
                ),
                &[("target_variable", target_variable)],
            );
            ResolutionError::InvalidInput {
                message: "Runtime operation missing operation_type".to_string(),
            }
        })?;

    let operation_type = RuntimeOperationType::from_str(operation_type_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid runtime operation type '{}' for variable '{}'",
                operation_type_str, target_variable
            ),
            &[
                ("target_variable", target_variable),
                ("operation_type", operation_type_str),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid runtime operation type: {}", operation_type_str),
        }
    })?;

    // Parse parameters with deferred validation
    let parameters = parse_run_parameters_with_context(
        run_json,
        target_variable,
        operation_type,
        parsing_context,
    )?;

    // Validate parameter requirements for operation type (structural validation)
    if let Err(validation_error) =
        validate_operation_parameters(&operation_type, &parameters, target_variable)
    {
        let error_msg = format!(
            "Parameter validation failed for runtime operation '{}': {}",
            target_variable, validation_error
        );
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &error_msg,
            &[("target_variable", target_variable)],
        );

        // For DAG resolution, continue parsing and collect error
        parsing_context.add_parsing_error(error_msg);
    }

    let _ = log_consumer_debug(
        "Runtime operation parsing completed with DAG context",
        &[
            ("target_variable", target_variable),
            ("operation_type", operation_type_str),
            ("parameter_count", &parameters.len().to_string()),
        ],
    );

    Ok(RuntimeOperation {
        target_variable: target_variable.to_string(),
        operation_type,
        parameters,
    })
}

/// Parse run parameters from JSON with deferred validation
/// EBNF: run_parameters ::= run_parameter+
fn parse_run_parameters_with_context(
    run_json: &serde_json::Value,
    target_variable: &str,
    operation_type: RuntimeOperationType,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<Vec<RunParameter>, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing run parameters with DAG context",
        &[("target_variable", target_variable)],
    );

    let parameters_array = run_json
        .get("parameters")
        .and_then(|params| params.as_array())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Runtime operation '{}' missing 'parameters' array",
                    target_variable
                ),
                &[("target_variable", target_variable)],
            );
            ResolutionError::InvalidInput {
                message: "Runtime operation missing parameters".to_string(),
            }
        })?;

    // Check if operation requires parameters (structural validation)
    if operation_type.requires_parameters() && parameters_array.is_empty() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Runtime operation '{}' of type {} requires parameters but none provided",
                target_variable,
                operation_type.as_str()
            ),
            &[
                ("target_variable", target_variable),
                ("operation_type", operation_type.as_str()),
            ],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Runtime operation '{}' of type {} requires parameters",
                target_variable,
                operation_type.as_str()
            ),
        });
    }

    let _ = log_consumer_debug(
        "Found run parameters array",
        &[
            ("target_variable", target_variable),
            ("parameter_count", &parameters_array.len().to_string()),
        ],
    );

    let mut parameters = Vec::new();

    for (parameter_index, parameter_json) in parameters_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing run parameter",
            &[
                ("target_variable", target_variable),
                ("parameter_index", &parameter_index.to_string()),
            ],
        );

        match parse_run_parameter_from_json_with_context(
            parameter_json,
            target_variable,
            parameter_index,
            parsing_context,
        ) {
            Ok(parameter) => {
                let _ = log_consumer_debug(
                    "Successfully parsed run parameter",
                    &[
                        ("target_variable", target_variable),
                        ("parameter_index", &parameter_index.to_string()),
                        ("parameter_type", parameter.parameter_type_name()),
                    ],
                );
                parameters.push(parameter);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to parse parameter {} in runtime operation '{}': {:?}",
                    parameter_index, target_variable, e
                );
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[
                        ("target_variable", target_variable),
                        ("parameter_index", &parameter_index.to_string()),
                    ],
                );

                // For DAG resolution, continue parsing and collect error
                parsing_context.add_parsing_error(error_msg);
            }
        }
    }

    let _ = log_consumer_debug(
        "Run parameters parsing completed with DAG context",
        &[
            ("target_variable", target_variable),
            ("total_parameters", &parameters.len().to_string()),
        ],
    );

    Ok(parameters)
}

/// Parse a single run parameter from JSON using tagged union format with deferred validation
/// EBNF: run_parameter ::= parameter_line statement_end
fn parse_run_parameter_from_json_with_context(
    parameter_json: &serde_json::Value,
    target_variable: &str,
    parameter_index: usize,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<RunParameter, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing run parameter with DAG context",
        &[
            ("target_variable", target_variable),
            ("parameter_index", &parameter_index.to_string()),
            (
                "parameter_keys",
                &parameter_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            ),
        ],
    );

    // Handle Literal parameter
    if let Some(literal_json) = parameter_json.get("Literal") {
        let _ = log_consumer_debug(
            "Parsing Literal parameter",
            &[("target_variable", target_variable)],
        );
        return parse_literal_parameter_with_context(
            literal_json,
            target_variable,
            parsing_context,
        );
    }

    // Handle Variable parameter with deferred validation
    if let Some(variable_name) = parameter_json.get("Variable").and_then(|v| v.as_str()) {
        let _ = log_consumer_debug(
            "Parsing Variable parameter with deferred validation",
            &[
                ("target_variable", target_variable),
                ("variable_name", variable_name),
            ],
        );

        if !is_valid_identifier(variable_name) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid variable identifier '{}' in parameter {} of runtime operation '{}'",
                    variable_name, parameter_index, target_variable
                ),
                &[
                    ("target_variable", target_variable),
                    ("variable_name", variable_name),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid variable identifier format: '{}'", variable_name),
            });
        }

        // DEFER variable existence validation - it might be a computed variable
        parsing_context.defer_variable_reference(
            target_variable.to_string(),
            variable_name.to_string(),
            format!(
                "parameter {} in RUN operation '{}'",
                parameter_index, target_variable
            ),
        );

        return Ok(RunParameter::Variable(variable_name.to_string()));
    }

    // Handle ObjectExtraction parameter with deferred validation
    if let Some(obj_extraction_json) = parameter_json.get("ObjectExtraction") {
        let _ = log_consumer_debug(
            "Parsing ObjectExtraction parameter with deferred validation",
            &[("target_variable", target_variable)],
        );
        return parse_object_extraction_parameter_with_context(
            obj_extraction_json,
            target_variable,
            parameter_index,
            parsing_context,
        );
    }

    // Handle Pattern parameter
    if let Some(pattern_str) = parameter_json.get("Pattern").and_then(|p| p.as_str()) {
        let _ = log_consumer_debug(
            "Parsing Pattern parameter",
            &[
                ("target_variable", target_variable),
                ("pattern", pattern_str),
            ],
        );

        if pattern_str.trim().is_empty() {
            let error_msg = format!(
                "Empty pattern in parameter {} of runtime operation '{}'",
                parameter_index, target_variable
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &error_msg,
                &[("target_variable", target_variable)],
            );

            // For DAG resolution, continue and collect error
            parsing_context.add_parsing_error(error_msg);
            return Ok(RunParameter::Pattern(pattern_str.to_string()));
        }

        return Ok(RunParameter::Pattern(pattern_str.to_string()));
    }

    // Handle Delimiter parameter
    if let Some(delimiter_str) = parameter_json.get("Delimiter").and_then(|d| d.as_str()) {
        let _ = log_consumer_debug(
            "Parsing Delimiter parameter",
            &[
                ("target_variable", target_variable),
                ("delimiter", delimiter_str),
            ],
        );
        return Ok(RunParameter::Delimiter(delimiter_str.to_string()));
    }

    // Handle Character parameter
    if let Some(character_str) = parameter_json.get("Character").and_then(|c| c.as_str()) {
        let _ = log_consumer_debug(
            "Parsing Character parameter",
            &[
                ("target_variable", target_variable),
                ("character", character_str),
            ],
        );
        return Ok(RunParameter::Character(character_str.to_string()));
    }

    // Handle StartPosition parameter
    if let Some(start_pos) = parameter_json
        .get("StartPosition")
        .and_then(|sp| sp.as_i64())
    {
        let _ = log_consumer_debug(
            "Parsing StartPosition parameter",
            &[
                ("target_variable", target_variable),
                ("start_position", &start_pos.to_string()),
            ],
        );

        if start_pos < 0 {
            let error_msg = format!(
                "Negative start position {} in parameter {} of runtime operation '{}'",
                start_pos, parameter_index, target_variable
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &error_msg,
                &[
                    ("target_variable", target_variable),
                    ("start_position", &start_pos.to_string()),
                ],
            );

            // For DAG resolution, continue and collect error
            parsing_context.add_parsing_error(error_msg);
        }

        return Ok(RunParameter::StartPosition(start_pos));
    }

    // Handle Length parameter
    if let Some(length_val) = parameter_json.get("Length").and_then(|l| l.as_i64()) {
        let _ = log_consumer_debug(
            "Parsing Length parameter",
            &[
                ("target_variable", target_variable),
                ("length", &length_val.to_string()),
            ],
        );

        if length_val <= 0 {
            let error_msg = format!(
                "Non-positive length {} in parameter {} of runtime operation '{}'",
                length_val, parameter_index, target_variable
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &error_msg,
                &[
                    ("target_variable", target_variable),
                    ("length", &length_val.to_string()),
                ],
            );

            // For DAG resolution, continue and collect error
            parsing_context.add_parsing_error(error_msg);
        }

        return Ok(RunParameter::Length(length_val));
    }

    // Handle ArithmeticOp parameter
    if let Some(arithmetic_op_json) = parameter_json.get("ArithmeticOp") {
        let _ = log_consumer_debug(
            "Parsing ArithmeticOp parameter",
            &[("target_variable", target_variable)],
        );
        return parse_arithmetic_op_parameter_with_context(
            arithmetic_op_json,
            target_variable,
            parameter_index,
            parsing_context,
        );
    }

    // Unknown parameter type
    let available_keys = parameter_json
        .as_object()
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);

    let error_msg = format!(
        "Unknown run parameter type at index {} in runtime operation '{}'",
        parameter_index, target_variable
    );
    let _ = log_consumer_error(
        consumer_codes::CONSUMER_FORMAT_ERROR,
        &error_msg,
        &[
            ("target_variable", target_variable),
            ("parameter_index", &parameter_index.to_string()),
            ("available_keys", &available_keys.join(",")),
        ],
    );

    Err(ResolutionError::InvalidInput { message: error_msg })
}

/// Parse literal parameter with context
fn parse_literal_parameter_with_context(
    literal_json: &serde_json::Value,
    target_variable: &str,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<RunParameter, ResolutionError> {
    match parse_value_from_json_with_context(literal_json, target_variable, parsing_context) {
        Ok(value) => {
            let _ = log_consumer_debug(
                "Successfully parsed literal parameter",
                &[("target_variable", target_variable)],
            );
            Ok(RunParameter::Literal(value))
        }
        Err(e) => {
            let error_msg = format!("Failed to parse literal parameter: {}", e);
            parsing_context.add_parsing_error(error_msg.clone());
            Err(e)
        }
    }
}

/// Parse object extraction parameter with deferred validation
fn parse_object_extraction_parameter_with_context(
    obj_extraction_json: &serde_json::Value,
    target_variable: &str,
    parameter_index: usize,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<RunParameter, ResolutionError> {
    let object_id = obj_extraction_json
        .get("object_id")
        .and_then(|oid| oid.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "ObjectExtraction parameter {} missing 'object_id' in runtime operation '{}'",
                    parameter_index, target_variable
                ),
                &[
                    ("target_variable", target_variable),
                    ("parameter_index", &parameter_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "ObjectExtraction missing object_id".to_string(),
            }
        })?;

    let field = obj_extraction_json
        .get("field")
        .and_then(|f| f.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "ObjectExtraction parameter {} missing 'field' in runtime operation '{}'",
                    parameter_index, target_variable
                ),
                &[
                    ("target_variable", target_variable),
                    ("parameter_index", &parameter_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "ObjectExtraction missing field".to_string(),
            }
        })?;

    if !is_valid_identifier(object_id) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!("Invalid object identifier '{}' in ObjectExtraction parameter {} of runtime operation '{}'", object_id, parameter_index, target_variable),
            &[("target_variable", target_variable), ("object_id", object_id)]
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Invalid object identifier format: '{}'", object_id),
        });
    }

    if !is_valid_identifier(field) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!("Invalid field identifier '{}' in ObjectExtraction parameter {} of runtime operation '{}'", field, parameter_index, target_variable),
            &[("target_variable", target_variable), ("field", field)]
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Invalid field identifier format: '{}'", field),
        });
    }

    // DEFER object existence validation - object might not be resolved yet
    let _ = log_consumer_debug(
        "Deferring object field extraction validation",
        &[
            ("target_variable", target_variable),
            ("object_id", object_id),
            ("field", field),
        ],
    );

    parsing_context.defer_object_field_extraction(
        target_variable.to_string(),
        object_id.to_string(),
        format!(
            "ObjectExtraction parameter {} in RUN operation '{}' extracting field '{}'",
            parameter_index, target_variable, field
        ),
    );

    let _ = log_consumer_debug(
        "Successfully parsed ObjectExtraction parameter with deferred validation",
        &[
            ("target_variable", target_variable),
            ("object_id", object_id),
            ("field", field),
        ],
    );

    Ok(RunParameter::ObjectExtraction {
        object_id: object_id.to_string(),
        field: field.to_string(),
    })
}

/// Parse arithmetic operation parameter with context
fn parse_arithmetic_op_parameter_with_context(
    arithmetic_op_json: &serde_json::Value,
    target_variable: &str,
    parameter_index: usize,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<RunParameter, ResolutionError> {
    // Handle array format [operator, value]
    if let Some(op_array) = arithmetic_op_json.as_array() {
        if op_array.len() != 2 {
            let error_msg = format!(
                "ArithmeticOp parameter {} must be array of [operator, value] in runtime operation '{}'", 
                parameter_index, target_variable
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &error_msg,
                &[
                    ("target_variable", target_variable),
                    ("array_length", &op_array.len().to_string()),
                ],
            );
            parsing_context.add_parsing_error(error_msg);
            return Err(ResolutionError::InvalidInput {
                message: "ArithmeticOp parameter must be [operator, value] array".to_string(),
            });
        }

        let operator_str = op_array[0].as_str().ok_or_else(|| {
            let error_msg = format!(
                "ArithmeticOp operator is not a string in parameter {} of runtime operation '{}'",
                parameter_index, target_variable
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &error_msg,
                &[("target_variable", target_variable)],
            );
            parsing_context.add_parsing_error(error_msg);
            ResolutionError::InvalidInput {
                message: "ArithmeticOp operator must be a string".to_string(),
            }
        })?;

        let operator = ArithmeticOperator::from_str(operator_str).ok_or_else(|| {
            let error_msg = format!(
                "Invalid arithmetic operator '{}' in parameter {} of runtime operation '{}'",
                operator_str, parameter_index, target_variable
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &error_msg,
                &[
                    ("target_variable", target_variable),
                    ("operator", operator_str),
                ],
            );
            parsing_context.add_parsing_error(error_msg);
            ResolutionError::InvalidInput {
                message: format!("Invalid arithmetic operator: {}", operator_str),
            }
        })?;

        let operand =
            parse_value_from_json_with_context(&op_array[1], target_variable, parsing_context)?;

        let _ = log_consumer_debug(
            "Successfully parsed ArithmeticOp parameter with operand",
            &[
                ("target_variable", target_variable),
                ("operator", operator_str),
                (
                    "operand_type",
                    match &operand {
                        Value::String(_) => "string",
                        Value::Integer(_) => "integer",
                        Value::Float(_) => "float",
                        Value::Boolean(_) => "boolean",
                        Value::Variable(_) => "variable",
                    },
                ),
            ],
        );

        return Ok(RunParameter::ArithmeticOp { operator, operand });
    }

    // Handle legacy object format for backwards compatibility
    if let Some(operator_str) = arithmetic_op_json
        .get("operator")
        .and_then(|op| op.as_str())
    {
        let operator = ArithmeticOperator::from_str(operator_str).ok_or_else(|| {
            ResolutionError::InvalidInput {
                message: format!("Invalid arithmetic operator: {}", operator_str),
            }
        })?;

        let value_json =
            arithmetic_op_json
                .get("value")
                .ok_or_else(|| ResolutionError::InvalidInput {
                    message: "ArithmeticOp missing value".to_string(),
                })?;

        let operand =
            parse_value_from_json_with_context(value_json, target_variable, parsing_context)?;

        return Ok(RunParameter::ArithmeticOp { operator, operand });
    }

    Err(ResolutionError::InvalidInput {
        message: format!(
            "Invalid ArithmeticOp format in runtime operation '{}'",
            target_variable
        ),
    })
}

/// Validate parameters for specific operation types (structural validation only)
fn validate_operation_parameters(
    operation_type: &RuntimeOperationType,
    parameters: &[RunParameter],
    target_variable: &str,
) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating operation parameters (structural only)",
        &[
            ("target_variable", target_variable),
            ("operation_type", operation_type.as_str()),
            ("parameter_count", &parameters.len().to_string()),
        ],
    );

    match operation_type {
        RuntimeOperationType::Split => {
            if !has_delimiter_parameter(parameters) {
                return Err(ResolutionError::InvalidInput {
                    message: format!(
                        "SPLIT operation '{}' requires a delimiter parameter",
                        target_variable
                    ),
                });
            }
        }
        RuntimeOperationType::Substring => {
            if !has_start_parameter(parameters) || !has_length_parameter(parameters) {
                return Err(ResolutionError::InvalidInput {
                    message: format!(
                        "SUBSTRING operation '{}' requires start and length parameters",
                        target_variable
                    ),
                });
            }
        }
        RuntimeOperationType::RegexCapture => {
            if !has_pattern_parameter(parameters) {
                return Err(ResolutionError::InvalidInput {
                    message: format!(
                        "REGEX_CAPTURE operation '{}' requires a pattern parameter",
                        target_variable
                    ),
                });
            }
        }
        RuntimeOperationType::Extract => {
            if !has_object_extraction_parameter(parameters) {
                return Err(ResolutionError::InvalidInput {
                    message: format!(
                        "EXTRACT operation '{}' requires an object extraction parameter",
                        target_variable
                    ),
                });
            }
        }
        RuntimeOperationType::Arithmetic => {
            if !has_arithmetic_parameter(parameters) {
                return Err(ResolutionError::InvalidInput {
                    message: format!(
                        "ARITHMETIC operation '{}' requires arithmetic operator parameters",
                        target_variable
                    ),
                });
            }
        }
        _ => {
            // Other operations have flexible parameter requirements
        }
    }

    let _ = log_consumer_debug(
        "Operation parameter validation completed successfully",
        &[("target_variable", target_variable)],
    );

    Ok(())
}

/// Helper functions to check for specific parameter types
fn has_delimiter_parameter(parameters: &[RunParameter]) -> bool {
    parameters
        .iter()
        .any(|param| matches!(param, RunParameter::Delimiter(_)))
}

fn has_pattern_parameter(parameters: &[RunParameter]) -> bool {
    parameters
        .iter()
        .any(|param| matches!(param, RunParameter::Pattern(_)))
}

fn has_start_parameter(parameters: &[RunParameter]) -> bool {
    parameters
        .iter()
        .any(|param| matches!(param, RunParameter::StartPosition(_)))
}

fn has_length_parameter(parameters: &[RunParameter]) -> bool {
    parameters
        .iter()
        .any(|param| matches!(param, RunParameter::Length(_)))
}

fn has_object_extraction_parameter(parameters: &[RunParameter]) -> bool {
    parameters
        .iter()
        .any(|param| matches!(param, RunParameter::ObjectExtraction { .. }))
}

fn has_arithmetic_parameter(parameters: &[RunParameter]) -> bool {
    parameters
        .iter()
        .any(|param| matches!(param, RunParameter::ArithmeticOp { .. }))
}

/// Parse Value enum from JSON with variable reference tracking
fn parse_value_from_json_with_context(
    value_json: &serde_json::Value,
    target_variable: &str,
    parsing_context: &mut RuntimeOperationParsingContext,
) -> Result<Value, ResolutionError> {
    if let Some(string_val) = value_json.get("String").and_then(|s| s.as_str()) {
        Ok(Value::String(string_val.to_string()))
    } else if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
        Ok(Value::Integer(int_val))
    } else if let Some(float_val) = value_json.get("Float").and_then(|f| f.as_f64()) {
        Ok(Value::Float(float_val))
    } else if let Some(bool_val) = value_json.get("Boolean").and_then(|b| b.as_bool()) {
        Ok(Value::Boolean(bool_val))
    } else if let Some(var_name) = value_json.get("Variable").and_then(|v| v.as_str()) {
        let _ = log_consumer_debug(
            "Found variable reference in value",
            &[
                ("target_variable", target_variable),
                ("variable_name", var_name),
            ],
        );

        // DEFER variable reference validation - variable might be computed or not exist yet
        parsing_context.defer_variable_reference(
            target_variable.to_string(),
            var_name.to_string(),
            format!("value in runtime operation '{}'", target_variable),
        );

        Ok(Value::Variable(var_name.to_string()))
    } else {
        let available_keys = value_json
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
            .unwrap_or_else(|| "none".to_string());

        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "Unknown value type in JSON",
            &[
                ("target_variable", target_variable),
                ("available_keys", &available_keys),
            ],
        );
        Err(ResolutionError::InvalidInput {
            message: "Unknown value type in JSON".to_string(),
        })
    }
}

/// Backward compatibility: Parse Value enum from JSON (without context)
fn parse_value_from_json(value_json: &serde_json::Value) -> Result<Value, ResolutionError> {
    let mut dummy_context = RuntimeOperationParsingContext::new();
    parse_value_from_json_with_context(value_json, "unknown", &mut dummy_context)
}

/// Validate identifier format (basic validation matching ICS EBNF identifier rules)
fn is_valid_identifier(id: &str) -> bool {
    if id.is_empty() {
        return false;
    }

    let chars: Vec<char> = id.chars().collect();

    // First character must be letter or underscore
    if !chars[0].is_ascii_alphabetic() && chars[0] != '_' {
        return false;
    }

    // Remaining characters must be alphanumeric or underscore
    for &ch in &chars[1..] {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return false;
        }
    }

    true
}

/// Check if a runtime operation has variable references in any of its parameters (helper for existing code)
pub fn has_variable_references_in_runtime_operation(runtime_operation: &RuntimeOperation) -> bool {
    runtime_operation.has_variable_references()
}

/// Get all variable references from a runtime operation (for dependency analysis)
pub fn get_variable_references_from_runtime_operation(
    runtime_operation: &RuntimeOperation,
) -> Vec<String> {
    runtime_operation.get_variable_references()
}

/// Get all object dependencies from a runtime operation (for dependency analysis)
pub fn get_object_dependencies_from_runtime_operation(
    runtime_operation: &RuntimeOperation,
) -> Vec<String> {
    runtime_operation.get_object_dependencies()
}

/// Validate runtime operation structure and consistency (backward compatibility)
pub fn validate_runtime_operation(
    runtime_operation: &RuntimeOperation,
) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating runtime operation",
        &[
            ("target_variable", &runtime_operation.target_variable),
            ("operation_type", runtime_operation.operation_type.as_str()),
        ],
    );

    // Use the built-in validation method
    if let Err(validation_error) = runtime_operation.validate() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Runtime operation validation failed for '{}': {}",
                runtime_operation.target_variable, validation_error
            ),
            &[("target_variable", &runtime_operation.target_variable)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Runtime operation '{}' validation failed: {}",
                runtime_operation.target_variable, validation_error
            ),
        });
    }

    let _ = log_consumer_debug(
        "Runtime operation validation completed successfully",
        &[("target_variable", &runtime_operation.target_variable)],
    );

    Ok(())
}

/// Apply runtime operation parsing results to ResolutionContext
pub fn apply_runtime_operation_parsing_to_context(
    parsing_result: RuntimeOperationParsingResult,
    resolution_context: &mut crate::types::resolution_context::ResolutionContext,
) {
    // Add runtime operations to resolution context
    resolution_context.runtime_operations = parsing_result.runtime_operations;

    // Apply parsing context (errors and deferred validations)
    parsing_result
        .context
        .apply_to_resolution_context(resolution_context);
}
