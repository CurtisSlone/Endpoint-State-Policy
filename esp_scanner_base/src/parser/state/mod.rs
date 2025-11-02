// parser/state/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::resolution::ResolutionError;
use crate::types::common::{DataType, FieldPath, Operation, Value};
use crate::types::state::{
    EntityCheck, RecordCheck, RecordContent, RecordField, StateDeclaration, StateField, StateRef,
};
use crate::types::ResolutionContext;
use std::collections::HashMap;

/// Extract states from AST JSON, separating global and local states
/// Returns (global_states, local_states_by_ctn_id)
pub fn extract_states_from_json(
    ast_json: &serde_json::Value,
    context: &mut ResolutionContext,
) -> Result<(Vec<StateDeclaration>, HashMap<usize, Vec<StateDeclaration>>), ResolutionError> {
    let _ = log_consumer_debug(
        "Starting state extraction from AST JSON with deferred validation",
        &[("ast_is_object", &ast_json.is_object().to_string())],
    );

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

    // Extract global states from definition.states
    let global_states = extract_global_states(definition, context)?;

    // Extract local states from criteria CTN blocks
    let local_states = extract_local_states_from_criteria(definition, context)?;

    let total_local_states: usize = local_states.values().map(|v| v.len()).sum();
    let _ = log_consumer_debug(
        "State extraction completed",
        &[
            ("global_states", &global_states.len().to_string()),
            ("local_ctn_count", &local_states.len().to_string()),
            ("total_local_states", &total_local_states.to_string()),
            (
                "deferred_validations",
                &context.deferred_validations.len().to_string(),
            ),
        ],
    );

    Ok((global_states, local_states))
}

/// Extract global states from definition.states array
fn extract_global_states(
    definition: &serde_json::Value,
    context: &mut ResolutionContext,
) -> Result<Vec<StateDeclaration>, ResolutionError> {
    let _ = log_consumer_debug("Extracting global states from definition", &[]);

    let empty_vec = Vec::new();
    let states_array = definition
        .get("states")
        .and_then(|states| states.as_array())
        .unwrap_or(&empty_vec); // No states is valid, return empty vec

    let _ = log_consumer_debug(
        "Found global states array",
        &[("state_count", &states_array.len().to_string())],
    );

    let mut global_states = Vec::new();

    for (index, state_json) in states_array.iter().enumerate() {
        let _ = log_consumer_debug("Processing global state", &[("index", &index.to_string())]);

        match parse_state_from_json(state_json, true) {
            Ok(state) => {
                let _ = log_consumer_debug(
                    "Successfully parsed global state",
                    &[
                        ("state_id", &state.identifier),
                        ("field_count", &state.fields.len().to_string()),
                        ("record_check_count", &state.record_checks.len().to_string()),
                    ],
                );

                // NEW: Collect variable references for deferred validation
                collect_state_variable_references(&state, context);

                global_states.push(state);
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!("Failed to parse global state at index {}: {:?}", index, e),
                    &[("index", &index.to_string())],
                );
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Global state extraction completed",
        &[("total_extracted", &global_states.len().to_string())],
    );

    Ok(global_states)
}

/// Extract local states from criteria blocks, organizing by CTN node ID
fn extract_local_states_from_criteria(
    definition: &serde_json::Value,
    context: &mut ResolutionContext,
) -> Result<HashMap<usize, Vec<StateDeclaration>>, ResolutionError> {
    let _ = log_consumer_debug("Extracting local states from criteria", &[]);

    let empty_vec = Vec::new();
    let criteria_array = definition
        .get("criteria")
        .and_then(|crit| crit.as_array())
        .unwrap_or(&empty_vec); // No criteria is valid for this extraction

    let mut local_states: HashMap<usize, Vec<StateDeclaration>> = HashMap::new();
    let mut ctn_node_id_counter = 0usize;

    for (criteria_index, criteria_json) in criteria_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing criteria block",
            &[("criteria_index", &criteria_index.to_string())],
        );

        if let Some(content_array) = criteria_json.get("content").and_then(|c| c.as_array()) {
            for (content_index, content_json) in content_array.iter().enumerate() {
                if let Some(criterion_json) = content_json.get("Criterion") {
                    let ctn_type = criterion_json
                        .get("criterion_type")
                        .and_then(|ct| ct.as_str())
                        .unwrap_or("unknown");

                    let _ = log_consumer_debug(
                        "Processing criterion",
                        &[
                            ("ctn_node_id", &ctn_node_id_counter.to_string()),
                            ("ctn_type", ctn_type),
                            ("content_index", &content_index.to_string()),
                        ],
                    );

                    // Check for local_states (multiple allowed)
                    if let Some(local_states_json) = criterion_json.get("local_states") {
                        if let Some(local_states_array) = local_states_json.as_array() {
                            if !local_states_array.is_empty() {
                                let _ = log_consumer_debug(
                                    "Found local states in CTN",
                                    &[
                                        ("ctn_node_id", &ctn_node_id_counter.to_string()),
                                        ("ctn_type", ctn_type),
                                        ("state_count", &local_states_array.len().to_string()),
                                    ],
                                );

                                let mut ctn_states = Vec::new();

                                for (state_index, state_json) in
                                    local_states_array.iter().enumerate()
                                {
                                    match parse_state_from_json(state_json, false) {
                                        Ok(state) => {
                                            let _ = log_consumer_debug(
                                                "Successfully parsed local state",
                                                &[
                                                    ("state_id", &state.identifier),
                                                    (
                                                        "ctn_node_id",
                                                        &ctn_node_id_counter.to_string(),
                                                    ),
                                                    ("state_index", &state_index.to_string()),
                                                ],
                                            );

                                            // NEW: Collect variable references for deferred validation
                                            collect_state_variable_references(&state, context);

                                            ctn_states.push(state);
                                        }
                                        Err(e) => {
                                            let _ = log_consumer_error(
                                                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                                                &format!("Failed to parse local state {} in CTN {}: {:?}", state_index, ctn_node_id_counter, e),
                                                &[("ctn_node_id", &ctn_node_id_counter.to_string()),
                                                 ("ctn_type", ctn_type),
                                                 ("state_index", &state_index.to_string())]
                                            );
                                            return Err(e);
                                        }
                                    }
                                }

                                local_states.insert(ctn_node_id_counter, ctn_states);
                            }
                        }
                    }

                    ctn_node_id_counter += 1;
                }
            }
        }
    }

    let total_local_states: usize = local_states.values().map(|v| v.len()).sum();
    let _ = log_consumer_debug(
        "Local state extraction completed",
        &[
            ("ctn_count", &local_states.len().to_string()),
            ("total_local_states", &total_local_states.to_string()),
        ],
    );

    Ok(local_states)
}

/// NEW: Collect variable references from state and add to deferred validation
fn collect_state_variable_references(state: &StateDeclaration, context: &mut ResolutionContext) {
    if state.has_variable_references() {
        let var_refs = state.get_variable_references();
        for var_ref in var_refs {
            context.defer_variable_reference_validation(
                state.identifier.clone(),
                var_ref,
                format!("state field reference in '{}'", state.identifier),
            );
        }
    }
}

/// Parse a single state from JSON
pub fn parse_state_from_json(
    state_json: &serde_json::Value,
    is_global: bool,
) -> Result<StateDeclaration, ResolutionError> {
    let identifier = state_json
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "State missing 'id' field",
                &[("is_global", &is_global.to_string())],
            );
            ResolutionError::InvalidInput {
                message: "State missing id".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Parsing state",
        &[
            ("state_id", identifier),
            ("is_global", &is_global.to_string()),
        ],
    );

    // Validate state identifier format (basic identifier validation)
    if !is_valid_identifier(identifier) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!("Invalid state identifier format: '{}'", identifier),
            &[("state_id", identifier)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Invalid state identifier format: '{}'", identifier),
        });
    }

    // Parse state fields
    let fields = parse_state_fields(state_json, identifier)?;

    // Parse record checks
    let record_checks = parse_record_checks(state_json, identifier)?;

    // Validate state is not empty
    if fields.is_empty() && record_checks.is_empty() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "State '{}' has no fields or record checks (empty states not allowed)",
                identifier
            ),
            &[("state_id", identifier)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("State '{}' cannot be empty", identifier),
        });
    }

    let _ = log_consumer_debug(
        "State parsing completed",
        &[
            ("state_id", identifier),
            ("field_count", &fields.len().to_string()),
            ("record_check_count", &record_checks.len().to_string()),
            ("is_global", &is_global.to_string()),
        ],
    );

    Ok(StateDeclaration {
        identifier: identifier.to_string(),
        fields,
        record_checks,
        is_global,
    })
}

/// Parse state fields array from JSON
fn parse_state_fields(
    state_json: &serde_json::Value,
    state_id: &str,
) -> Result<Vec<StateField>, ResolutionError> {
    let _ = log_consumer_debug("Parsing state fields", &[("state_id", state_id)]);

    let empty_vec = Vec::new();
    let fields_array = state_json
        .get("fields")
        .and_then(|fields| fields.as_array())
        .unwrap_or(&empty_vec); // No fields is valid, could have record checks

    let _ = log_consumer_debug(
        "Found state fields array",
        &[
            ("state_id", state_id),
            ("field_count", &fields_array.len().to_string()),
        ],
    );

    let mut fields = Vec::new();

    for (field_index, field_json) in fields_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing state field",
            &[
                ("state_id", state_id),
                ("field_index", &field_index.to_string()),
            ],
        );

        match parse_state_field_from_json(field_json, state_id, field_index) {
            Ok(field) => {
                let _ = log_consumer_debug(
                    "Successfully parsed state field",
                    &[
                        ("state_id", state_id),
                        ("field_name", &field.name),
                        ("field_index", &field_index.to_string()),
                    ],
                );
                fields.push(field);
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Failed to parse field {} in state '{}': {:?}",
                        field_index, state_id, e
                    ),
                    &[
                        ("state_id", state_id),
                        ("field_index", &field_index.to_string()),
                    ],
                );
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "State fields parsing completed",
        &[
            ("state_id", state_id),
            ("total_fields", &fields.len().to_string()),
        ],
    );

    Ok(fields)
}

/// Parse a single state field from JSON
/// EBNF: state_field ::= field_name space data_type space operation space value_spec (space entity_check)? statement_end
fn parse_state_field_from_json(
    field_json: &serde_json::Value,
    state_id: &str,
    field_index: usize,
) -> Result<StateField, ResolutionError> {
    let name = field_json
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "State field at index {} missing 'name' in state '{}'",
                    field_index, state_id
                ),
                &[
                    ("state_id", state_id),
                    ("field_index", &field_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "State field missing name".to_string(),
            }
        })?;

    let data_type_str = field_json
        .get("data_type")
        .and_then(|dt| dt.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "State field '{}' missing 'data_type' in state '{}'",
                    name, state_id
                ),
                &[("state_id", state_id), ("field_name", name)],
            );
            ResolutionError::InvalidInput {
                message: "State field missing data_type".to_string(),
            }
        })?;

    let data_type = DataType::from_str(data_type_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid data type '{}' for field '{}' in state '{}'",
                data_type_str, name, state_id
            ),
            &[
                ("state_id", state_id),
                ("field_name", name),
                ("data_type", data_type_str),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid data type: {}", data_type_str),
        }
    })?;

    let operation_str = field_json
        .get("operation")
        .and_then(|op| op.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "State field '{}' missing 'operation' in state '{}'",
                    name, state_id
                ),
                &[("state_id", state_id), ("field_name", name)],
            );
            ResolutionError::InvalidInput {
                message: "State field missing operation".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Attempting to parse operation",
        &[("operation_str", operation_str), ("field_name", name)],
    );
    let operation = Operation::from_str(operation_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid operation '{}' for field '{}' in state '{}'",
                operation_str, name, state_id
            ),
            &[
                ("state_id", state_id),
                ("field_name", name),
                ("operation", operation_str),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid operation: {}", operation_str),
        }
    })?;

    let value_json = field_json.get("value").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!(
                "State field '{}' missing 'value' in state '{}'",
                name, state_id
            ),
            &[("state_id", state_id), ("field_name", name)],
        );
        ResolutionError::InvalidInput {
            message: "State field missing value".to_string(),
        }
    })?;

    let value = parse_value_from_json(
        value_json,
        data_type,                                         // Pass the declared type
        &format!("state '{}' field '{}'", state_id, name), // Context for error messages
    )?;

    // Parse optional entity check
    let entity_check = if let Some(entity_check_json) = field_json.get("entity_check") {
        if !entity_check_json.is_null() {
            let entity_check_str = entity_check_json.as_str().ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!(
                        "Entity check is not a string for field '{}' in state '{}'",
                        name, state_id
                    ),
                    &[("state_id", state_id), ("field_name", name)],
                );
                ResolutionError::InvalidInput {
                    message: "Entity check must be a string".to_string(),
                }
            })?;

            let entity_check = EntityCheck::from_str(entity_check_str).ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Invalid entity check '{}' for field '{}' in state '{}'",
                        entity_check_str, name, state_id
                    ),
                    &[
                        ("state_id", state_id),
                        ("field_name", name),
                        ("entity_check", entity_check_str),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: format!("Invalid entity check: {}", entity_check_str),
                }
            })?;

            Some(entity_check)
        } else {
            None
        }
    } else {
        None
    };

    let _ = log_consumer_debug(
        "Successfully parsed state field",
        &[
            ("state_id", state_id),
            ("field_name", name),
            ("data_type", data_type_str),
            ("operation", operation_str),
            ("has_entity_check", &entity_check.is_some().to_string()),
        ],
    );

    Ok(StateField {
        name: name.to_string(),
        data_type,
        operation,
        value,
        entity_check,
    })
}

/// Parse record checks array from JSON
fn parse_record_checks(
    state_json: &serde_json::Value,
    state_id: &str,
) -> Result<Vec<RecordCheck>, ResolutionError> {
    let _ = log_consumer_debug("Parsing record checks", &[("state_id", state_id)]);

    let empty_vec = Vec::new();
    let record_checks_array = state_json
        .get("record_checks")
        .and_then(|checks| checks.as_array())
        .unwrap_or(&empty_vec); // No record checks is valid

    let _ = log_consumer_debug(
        "Found record checks array",
        &[
            ("state_id", state_id),
            ("record_check_count", &record_checks_array.len().to_string()),
        ],
    );

    let mut record_checks = Vec::new();

    for (record_index, record_json) in record_checks_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing record check",
            &[
                ("state_id", state_id),
                ("record_index", &record_index.to_string()),
            ],
        );

        match parse_record_check_from_json(record_json, state_id, record_index) {
            Ok(record_check) => {
                let _ = log_consumer_debug(
                    "Successfully parsed record check",
                    &[
                        ("state_id", state_id),
                        ("record_index", &record_index.to_string()),
                    ],
                );
                record_checks.push(record_check);
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Failed to parse record check {} in state '{}': {:?}",
                        record_index, state_id, e
                    ),
                    &[
                        ("state_id", state_id),
                        ("record_index", &record_index.to_string()),
                    ],
                );
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Record checks parsing completed",
        &[
            ("state_id", state_id),
            ("total_record_checks", &record_checks.len().to_string()),
        ],
    );

    Ok(record_checks)
}

/// Parse a single record check from JSON
/// EBNF: record_check ::= "record" space data_type? statement_end record_content "record_end" statement_end
fn parse_record_check_from_json(
    record_json: &serde_json::Value,
    state_id: &str,
    record_index: usize,
) -> Result<RecordCheck, ResolutionError> {
    // Parse optional data type
    let data_type = if let Some(data_type_json) = record_json.get("data_type") {
        if !data_type_json.is_null() {
            let data_type_str = data_type_json.as_str().ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!(
                        "Record check data_type is not a string in record {} of state '{}'",
                        record_index, state_id
                    ),
                    &[
                        ("state_id", state_id),
                        ("record_index", &record_index.to_string()),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: "Record check data_type must be a string".to_string(),
                }
            })?;

            let data_type = DataType::from_str(data_type_str).ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Invalid data type '{}' for record check {} in state '{}'",
                        data_type_str, record_index, state_id
                    ),
                    &[
                        ("state_id", state_id),
                        ("record_index", &record_index.to_string()),
                        ("data_type", data_type_str),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: format!("Invalid record data type: {}", data_type_str),
                }
            })?;

            Some(data_type)
        } else {
            None
        }
    } else {
        None
    };

    // Parse record content
    let content_json = record_json.get("content").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!(
                "Record check {} missing 'content' in state '{}'",
                record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("record_index", &record_index.to_string()),
            ],
        );
        ResolutionError::InvalidInput {
            message: "Record check missing content".to_string(),
        }
    })?;

    let content = parse_record_content_from_json(content_json, state_id, record_index)?;

    let _ = log_consumer_debug(
        "Successfully parsed record check",
        &[
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
            ("has_data_type", &data_type.is_some().to_string()),
        ],
    );

    Ok(RecordCheck { data_type, content })
}

/// Parse record content from JSON
/// EBNF: record_content ::= direct_operation | nested_fields
fn parse_record_content_from_json(
    content_json: &serde_json::Value,
    state_id: &str,
    record_index: usize,
) -> Result<RecordContent, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing record content",
        &[
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
        ],
    );

    // Handle Direct content
    if let Some(direct_json) = content_json.get("Direct") {
        let _ = log_consumer_debug("Parsing Direct record content", &[("state_id", state_id)]);
        return parse_direct_record_content(direct_json, state_id, record_index);
    }

    // Handle Nested content
    if let Some(nested_json) = content_json.get("Nested") {
        let _ = log_consumer_debug("Parsing Nested record content", &[("state_id", state_id)]);
        return parse_nested_record_content(nested_json, state_id, record_index);
    }

    // Unknown content type
    let available_keys = content_json
        .as_object()
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);

    let _ = log_consumer_error(
        consumer_codes::CONSUMER_FORMAT_ERROR,
        &format!(
            "Unknown record content type in record {} of state '{}'",
            record_index, state_id
        ),
        &[
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
            ("available_keys", &available_keys.join(",")),
        ],
    );

    Err(ResolutionError::InvalidInput {
        message: format!(
            "Unknown record content type in record {} of state '{}'",
            record_index, state_id
        ),
    })
}

/// Parse direct record content (operation on entire record)
fn parse_direct_record_content(
    direct_json: &serde_json::Value,
    state_id: &str,
    record_index: usize,
) -> Result<RecordContent, ResolutionError> {
    let operation_str = direct_json
        .get("operation")
        .and_then(|op| op.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Direct record content missing 'operation' in record {} of state '{}'",
                    record_index, state_id
                ),
                &[
                    ("state_id", state_id),
                    ("record_index", &record_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "Direct record content missing operation".to_string(),
            }
        })?;

    let operation = Operation::from_str(operation_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid operation '{}' for direct record content in record {} of state '{}'",
                operation_str, record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("record_index", &record_index.to_string()),
                ("operation", operation_str),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid direct record operation: {}", operation_str),
        }
    })?;

    let value_json = direct_json.get("value").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!(
                "Direct record content missing 'value' in record {} of state '{}'",
                record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("record_index", &record_index.to_string()),
            ],
        );
        ResolutionError::InvalidInput {
            message: "Direct record content missing value".to_string(),
        }
    })?;

    // For record direct content, we don't have declared type yet, so pass generic
    let value = parse_value_from_json(
        value_json,
        DataType::String, // Or extract from record_check if available
        &format!(
            "state '{}' record {} direct content",
            state_id, record_index
        ),
    )?;

    let _ = log_consumer_debug(
        "Successfully parsed direct record content",
        &[
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
            ("operation", operation_str),
        ],
    );

    Ok(RecordContent::Direct { operation, value })
}

/// Parse nested record content (field operations)
fn parse_nested_record_content(
    nested_json: &serde_json::Value,
    state_id: &str,
    record_index: usize,
) -> Result<RecordContent, ResolutionError> {
    let fields_array = nested_json
        .get("fields")
        .and_then(|f| f.as_array())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Nested record content missing 'fields' array in record {} of state '{}'",
                    record_index, state_id
                ),
                &[
                    ("state_id", state_id),
                    ("record_index", &record_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "Nested record content missing fields".to_string(),
            }
        })?;

    // DEBUG: Show the fields array structure with proper JSON serialization
    let fields_debug = match serde_json::to_string_pretty(fields_array) {
        Ok(json_str) => json_str,
        Err(_) => format!("Failed to serialize, raw debug: {:?}", fields_array),
    };

    let _ = log_consumer_debug(
        "DEBUG_FIELDS_ARRAY_START",
        &[("json_content", &fields_debug)],
    );
    // DEBUG: Show the fields array structure - FIX THE to_string() ERROR
    let _ = log_consumer_debug(
        "DEBUG: Nested record fields array",
        &[
            ("fields_array", &format!("{:?}", fields_array)),
            ("fields_count", &fields_array.len().to_string()),
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
        ],
    );

    let mut fields = Vec::new();

    for (field_index, field_json) in fields_array.iter().enumerate() {
        match parse_record_field_from_json(field_json, state_id, record_index, field_index) {
            Ok(field) => {
                fields.push(field);
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Failed to parse record field {} in record {} of state '{}': {:?}",
                        field_index, record_index, state_id, e
                    ),
                    &[
                        ("state_id", state_id),
                        ("record_index", &record_index.to_string()),
                        ("field_index", &field_index.to_string()),
                    ],
                );
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Successfully parsed nested record content",
        &[
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
            ("field_count", &fields.len().to_string()),
        ],
    );

    Ok(RecordContent::Nested { fields })
}

/// Parse record field from JSON
/// EBNF: record_field ::= "field" space field_path space data_type space operation space value_spec (space entity_check)? statement_end
/// Parse record field from JSON
/// EBNF: record_field ::= "field" space field_path space data_type space operation
/// Parse Value enum from JSON (reused from variable module pattern)
///
///

/// Parse record field from JSON - FIXED to handle actual parser JSON structure
fn parse_record_field_from_json(
    field_json: &serde_json::Value,
    state_id: &str,
    record_index: usize,
    field_index: usize,
) -> Result<RecordField, ResolutionError> {
    // DEBUG: Log the actual JSON structure we're working with
    let field_debug = match serde_json::to_string_pretty(field_json) {
        Ok(json_str) => json_str,
        Err(_) => format!("Failed to serialize, raw debug: {:?}", field_json),
    };

    let _ = log_consumer_debug(
        "DEBUG: Parsing record field JSON structure",
        &[
            ("field_json", &field_debug),
            ("state_id", state_id),
            ("record_index", &record_index.to_string()),
            ("field_index", &field_index.to_string()),
        ],
    );

    // Parse the path field - handle the actual parser structure
    let path = if let Some(path_obj) = field_json.get("path") {
        // Check if path is an object with components array (actual parser format)
        if let Some(components_array) = path_obj.get("components").and_then(|c| c.as_array()) {
            // Extract components and create FieldPath
            let components: Result<Vec<String>, _> = components_array
                .iter()
                .map(|comp| {
                    comp.as_str()
                        .ok_or_else(|| ResolutionError::InvalidInput {
                            message: "Path component must be a string".to_string(),
                        })
                        .map(|s| s.to_string())
                })
                .collect();

            let components = components?;

            let _ = log_consumer_debug(
                "Successfully parsed path components",
                &[
                    ("state_id", state_id),
                    ("field_index", &field_index.to_string()),
                    ("components", &format!("{:?}", components)),
                ],
            );

            FieldPath::new(components)
        } else if let Some(path_str) = path_obj.as_str() {
            // Fallback: if path is directly a string
            let _ = log_consumer_debug("Path is direct string", &[("path_str", path_str)]);
            FieldPath::from_dot_notation(path_str)
        } else {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Record field {} path is not in expected format in record {} of state '{}'",
                    field_index, record_index, state_id
                ),
                &[
                    ("state_id", state_id),
                    ("record_index", &record_index.to_string()),
                    ("field_index", &field_index.to_string()),
                    ("path_structure", &format!("{:?}", path_obj)),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Record field {} missing valid path in record {} of state '{}'",
                    field_index, record_index, state_id
                ),
            });
        }
    } else {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!(
                "Record field {} missing 'path' field in record {} of state '{}'",
                field_index, record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("record_index", &record_index.to_string()),
                ("field_index", &field_index.to_string()),
                (
                    "available_keys",
                    &field_json
                        .as_object()
                        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                        .unwrap_or_else(|| "none".to_string()),
                ),
            ],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Record field {} missing path in record {} of state '{}'",
                field_index, record_index, state_id
            ),
        });
    };

    // Parse data_type
    let data_type_str = field_json
        .get("data_type")
        .and_then(|dt| dt.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Record field {} missing 'data_type' in record {} of state '{}'",
                    field_index, record_index, state_id
                ),
                &[
                    ("state_id", state_id),
                    ("record_index", &record_index.to_string()),
                    ("field_index", &field_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "Record field missing data_type".to_string(),
            }
        })?;

    let data_type = DataType::from_str(data_type_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid data type '{}' for record field {} in record {} of state '{}'",
                data_type_str, field_index, record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("data_type", data_type_str),
                ("field_index", &field_index.to_string()),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid record field data type: {}", data_type_str),
        }
    })?;

    // Parse operation
    let operation_str = field_json
        .get("operation")
        .and_then(|op| op.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Record field {} missing 'operation' in record {} of state '{}'",
                    field_index, record_index, state_id
                ),
                &[
                    ("state_id", state_id),
                    ("field_index", &field_index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: "Record field missing operation".to_string(),
            }
        })?;

    let operation = Operation::from_str(operation_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid operation '{}' for record field {} in record {} of state '{}'",
                operation_str, field_index, record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("operation", operation_str),
                ("field_index", &field_index.to_string()),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid record field operation: {}", operation_str),
        }
    })?;

    // Parse value
    let value_json = field_json.get("value").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!(
                "Record field {} missing 'value' in record {} of state '{}'",
                field_index, record_index, state_id
            ),
            &[
                ("state_id", state_id),
                ("field_index", &field_index.to_string()),
            ],
        );
        ResolutionError::InvalidInput {
            message: "Record field missing value".to_string(),
        }
    })?;

    let value = parse_value_from_json(
        value_json,
        data_type, // Use the field's declared type
        &format!(
            "state '{}' record {} field {}",
            state_id, record_index, field_index
        ),
    )?;

    // Parse optional entity check
    let entity_check = if let Some(entity_check_json) = field_json.get("entity_check") {
        if !entity_check_json.is_null() {
            let entity_check_str = entity_check_json.as_str()
                .ok_or_else(|| {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_FORMAT_ERROR,
                        &format!(
                            "Entity check is not a string for record field {} in record {} of state '{}'",
                            field_index, record_index, state_id
                        ),
                        &[
                            ("state_id", state_id),
                            ("field_index", &field_index.to_string()),
                        ],
                    );
                    ResolutionError::InvalidInput {
                        message: "Record field entity check must be a string".to_string() 
                    }
                })?;

            let entity_check = EntityCheck::from_str(entity_check_str).ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Invalid entity check '{}' for record field {} in record {} of state '{}'",
                        entity_check_str, field_index, record_index, state_id
                    ),
                    &[
                        ("state_id", state_id),
                        ("entity_check", entity_check_str),
                        ("field_index", &field_index.to_string()),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: format!("Invalid record field entity check: {}", entity_check_str),
                }
            })?;

            Some(entity_check)
        } else {
            None
        }
    } else {
        None
    };

    let _ = log_consumer_debug(
        "Successfully parsed record field",
        &[
            ("state_id", state_id),
            ("field_index", &field_index.to_string()),
            ("path_components", &path.to_dot_notation()),
            ("data_type", data_type_str),
            ("operation", operation_str),
            ("has_entity_check", &entity_check.is_some().to_string()),
        ],
    );

    Ok(RecordField {
        path,
        data_type,
        operation,
        value,
        entity_check,
    })
}

/// Parse and validate value from JSON against declared data type
fn parse_value_from_json(
    value_json: &serde_json::Value,
    declared_type: DataType,
    field_context: &str,
) -> Result<Value, ResolutionError> {
    // CHECK FOR VARIABLE REFERENCE FIRST - before type validation
    if let Some(var_name) = value_json.get("Variable").and_then(|v| v.as_str()) {
        return Ok(Value::Variable(var_name.to_string()));
    }

    // Now do type-specific validation for literals
    match declared_type {
        DataType::Boolean => {
            if let Some(bool_val) = value_json.get("Boolean").and_then(|b| b.as_bool()) {
                Ok(Value::Boolean(bool_val))
            } else if value_json.get("String").is_some() {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: boolean requires literal true/false (no backticks)",
                        field_context
                    ),
                })
            } else {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: expected boolean, got {:?}",
                        field_context, value_json
                    ),
                })
            }
        }

        DataType::String => {
            if let Some(str_val) = value_json.get("String").and_then(|s| s.as_str()) {
                Ok(Value::String(str_val.to_string()))
            } else if value_json.get("Boolean").is_some() || value_json.get("Integer").is_some() {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: string requires backticks (e.g., `value`)",
                        field_context
                    ),
                })
            } else {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: expected string, got {:?}",
                        field_context, value_json
                    ),
                })
            }
        }

        DataType::Int => {
            if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
                Ok(Value::Integer(int_val))
            } else if value_json.get("String").is_some() {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: integer requires number without backticks",
                        field_context
                    ),
                })
            } else {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: expected integer, got {:?}",
                        field_context, value_json
                    ),
                })
            }
        }

        DataType::Float => {
            if let Some(float_val) = value_json.get("Float").and_then(|f| f.as_f64()) {
                Ok(Value::Float(float_val))
            } else if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
                Ok(Value::Float(int_val as f64))
            } else if value_json.get("String").is_some() {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: float requires number without backticks",
                        field_context
                    ),
                })
            } else {
                Err(ResolutionError::InvalidInput {
                    message: format!(
                        "Type mismatch in {}: expected float, got {:?}",
                        field_context, value_json
                    ),
                })
            }
        }

        _ => {
            // Generic parsing for other types
            if let Some(string_val) = value_json.get("String").and_then(|s| s.as_str()) {
                Ok(Value::String(string_val.to_string()))
            } else if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
                Ok(Value::Integer(int_val))
            } else if let Some(float_val) = value_json.get("Float").and_then(|f| f.as_f64()) {
                Ok(Value::Float(float_val))
            } else if let Some(bool_val) = value_json.get("Boolean").and_then(|b| b.as_bool()) {
                Ok(Value::Boolean(bool_val))
            } else {
                Err(ResolutionError::InvalidInput {
                    message: format!("Unknown value type in {}: {:?}", field_context, value_json),
                })
            }
        }
    }
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

/// Extract state references from JSON (used by other parsers)
pub fn extract_state_references_from_json(
    refs_json: &serde_json::Value,
    context: &str,
) -> Result<Vec<StateRef>, ResolutionError> {
    let _ = log_consumer_debug("Extracting state references", &[("context", context)]);

    let refs_array = refs_json.as_array().ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("State references is not an array in context '{}'", context),
            &[("context", context)],
        );
        ResolutionError::InvalidInput {
            message: format!("State references must be an array in {}", context),
        }
    })?;

    let mut state_refs = Vec::new();

    for (index, ref_json) in refs_array.iter().enumerate() {
        let state_id = ref_json
            .get("state_id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!(
                        "State reference at index {} missing 'state_id' in context '{}'",
                        index, context
                    ),
                    &[("context", context), ("index", &index.to_string())],
                );
                ResolutionError::InvalidInput {
                    message: format!("State reference missing state_id in {}", context),
                }
            })?;

        if !is_valid_identifier(state_id) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid state ID format '{}' at index {} in context '{}'",
                    state_id, index, context
                ),
                &[
                    ("context", context),
                    ("state_id", state_id),
                    ("index", &index.to_string()),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid state ID format '{}' in {}", state_id, context),
            });
        }

        state_refs.push(StateRef::new(state_id));
    }

    let _ = log_consumer_debug(
        "State references extraction completed",
        &[
            ("context", context),
            ("reference_count", &state_refs.len().to_string()),
        ],
    );

    Ok(state_refs)
}

/// Check if a state has variable references in any of its fields or record checks
pub fn has_variable_references_in_state(state: &StateDeclaration) -> bool {
    state.has_variable_references()
}

/// Get all variable references from a state (for dependency analysis)
pub fn get_variable_references_from_state(state: &StateDeclaration) -> Vec<String> {
    state.get_variable_references()
}
