// parser/set/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::parser::filter;
use crate::resolution::ResolutionError;
use crate::types::filter::FilterSpec;
use crate::types::object::ObjectDeclaration;
use crate::types::resolution_context::ResolutionContext;
use crate::types::set::{SetOperand, SetOperation, SetOperationType};

/// Extract set operations from AST JSON with enhanced ResolutionContext integration
/// All sets are global scope only per ICS EBNF rules
pub fn extract_set_operations_from_json(
    ast_json: &serde_json::Value,
    context: &mut ResolutionContext, // NEW: Pass mutable context for deferred validation
) -> Result<Vec<SetOperation>, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting set operation extraction from AST JSON",
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

    let empty_vec = Vec::new();
    let set_operations_array = definition
        .get("set_operations")
        .and_then(|sets| sets.as_array())
        .unwrap_or(&empty_vec); // No set operations is valid, return empty vec

    let _ = log_consumer_debug(
        "Found set operations array",
        &[(
            "set_operation_count",
            &set_operations_array.len().to_string(),
        )],
    );

    let mut set_operations = Vec::new();

    for (index, set_json) in set_operations_array.iter().enumerate() {
        let _ = log_consumer_debug("Processing set operation", &[("index", &index.to_string())]);

        match parse_set_operation_from_json(set_json, context) {
            Ok(set_operation) => {
                let _ = log_consumer_debug(
                    "Successfully parsed set operation",
                    &[
                        ("set_id", &set_operation.set_id),
                        ("operation_type", set_operation.operation.as_str()),
                        ("operand_count", &set_operation.operands.len().to_string()),
                        ("has_filter", &set_operation.filter.is_some().to_string()),
                    ],
                );
                set_operations.push(set_operation);
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!("Failed to parse set operation at index {}: {:?}", index, e),
                    &[("index", &index.to_string())],
                );
                // NEW: Add non-fatal parsing error instead of returning immediately
                context.add_parsing_error(format!(
                    "Failed to parse set operation at index {}: {:?}",
                    index, e
                ));
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Set operation extraction completed",
        &[("total_extracted", &set_operations.len().to_string())],
    );

    Ok(set_operations)
}

/// Parse a single set operation from JSON with DAG-aware validation deferral
/// EBNF: set_block ::= "SET" space set_identifier space set_operation statement_end set_content "SET_END" statement_end
fn parse_set_operation_from_json(
    set_json: &serde_json::Value,
    context: &mut ResolutionContext, // NEW: Context for deferred validation
) -> Result<SetOperation, ResolutionError> {
    let set_id = set_json
        .get("set_id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Set operation missing 'set_id' field",
                &[],
            );
            ResolutionError::InvalidInput {
                message: "Set operation missing set_id".to_string(),
            }
        })?;

    let _ = log_consumer_debug("Parsing set operation", &[("set_id", set_id)]);

    // Validate set identifier format (basic structural validation - OK to keep)
    if !is_valid_identifier(set_id) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!("Invalid set identifier format: '{}'", set_id),
            &[("set_id", set_id)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Invalid set identifier format: '{}'", set_id),
        });
    }

    let operation_str = set_json
        .get("operation")
        .and_then(|op| op.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Set '{}' missing 'operation' field", set_id),
                &[("set_id", set_id)],
            );
            ResolutionError::InvalidInput {
                message: "Set operation missing operation type".to_string(),
            }
        })?;

    let operation = SetOperationType::from_str(operation_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid set operation type '{}' for set '{}'",
                operation_str, set_id
            ),
            &[("set_id", set_id), ("operation", operation_str)],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid set operation type: {}", operation_str),
        }
    })?;

    // Parse operands with deferred validation
    let operands = parse_set_operands(set_json, set_id, context)?;

    // Validate operand count for operation type (structural validation - OK to keep)
    if let Err(validation_error) = operation.validate_operand_count(operands.len()) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Set operation '{}' operand count validation failed: {}",
                set_id, validation_error
            ),
            &[
                ("set_id", set_id),
                ("operation", operation_str),
                ("operand_count", &operands.len().to_string()),
            ],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Set '{}': {}", set_id, validation_error),
        });
    }

    // Parse optional filter with deferred validation
    let filter = parse_set_filter(set_json, set_id, context)?;

    let _ = log_consumer_debug(
        "Set operation parsing completed",
        &[
            ("set_id", set_id),
            ("operation", operation_str),
            ("operand_count", &operands.len().to_string()),
            ("has_filter", &filter.is_some().to_string()),
        ],
    );

    Ok(SetOperation {
        set_id: set_id.to_string(),
        operation,
        operands,
        filter,
    })
}

/// Parse set operands from JSON with DAG-aware dependency collection
/// EBNF: set_operands ::= set_operand+
fn parse_set_operands(
    set_json: &serde_json::Value,
    set_id: &str,
    context: &mut ResolutionContext, // NEW: Context for deferred validation
) -> Result<Vec<SetOperand>, ResolutionError> {
    let _ = log_consumer_debug("Parsing set operands", &[("set_id", set_id)]);

    let operands_array = set_json
        .get("operands")
        .and_then(|operands| operands.as_array())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Set '{}' missing 'operands' array", set_id),
                &[("set_id", set_id)],
            );
            ResolutionError::InvalidInput {
                message: "Set missing operands".to_string(),
            }
        })?;

    if operands_array.is_empty() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!("Set '{}' has no operands (empty sets not allowed)", set_id),
            &[("set_id", set_id)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Set '{}' cannot have zero operands", set_id),
        });
    }

    let _ = log_consumer_debug(
        "Found set operands array",
        &[
            ("set_id", set_id),
            ("operand_count", &operands_array.len().to_string()),
        ],
    );

    let mut operands = Vec::new();

    for (operand_index, operand_json) in operands_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing set operand",
            &[
                ("set_id", set_id),
                ("operand_index", &operand_index.to_string()),
            ],
        );

        match parse_set_operand_from_json(operand_json, set_id, operand_index, context) {
            Ok(operand) => {
                let _ = log_consumer_debug(
                    "Successfully parsed set operand",
                    &[
                        ("set_id", set_id),
                        ("operand_index", &operand_index.to_string()),
                        ("operand_type", operand.operand_type_name()),
                    ],
                );
                operands.push(operand);
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Failed to parse operand {} in set '{}': {:?}",
                        operand_index, set_id, e
                    ),
                    &[
                        ("set_id", set_id),
                        ("operand_index", &operand_index.to_string()),
                    ],
                );
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Set operands parsing completed",
        &[
            ("set_id", set_id),
            ("total_operands", &operands.len().to_string()),
        ],
    );

    Ok(operands)
}

/// Parse a single set operand from JSON using tagged union format with deferred validation
/// EBNF: operand_type ::= object_spec | object_reference | set_reference
fn parse_set_operand_from_json(
    operand_json: &serde_json::Value,
    set_id: &str,
    operand_index: usize,
    context: &mut ResolutionContext, // NEW: Context for deferred validation
) -> Result<SetOperand, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing set operand",
        &[
            ("set_id", set_id),
            ("operand_index", &operand_index.to_string()),
            (
                "operand_keys",
                &operand_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            ),
        ],
    );

    // Handle ObjectRef operand (OBJECT_REF obj_id)
    if let Some(obj_ref_str) = operand_json
        .get("ObjectRef")
        .and_then(|obj_ref| obj_ref.as_str())
    {
        let _ = log_consumer_debug(
            "Parsing ObjectRef operand",
            &[("set_id", set_id), ("object_id", obj_ref_str)],
        );

        // Structural validation (identifier format) - OK to keep
        if !is_valid_identifier(obj_ref_str) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid object identifier '{}' in operand {} of set '{}'",
                    obj_ref_str, operand_index, set_id
                ),
                &[("set_id", set_id), ("object_id", obj_ref_str)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid object identifier format: '{}'", obj_ref_str),
            });
        }

        // NEW: Defer object reference validation to DAG phase
        context.defer_object_reference_validation(
            format!("SET:{}", set_id),
            obj_ref_str.to_string(),
            format!(
                "Set '{}' operand {} references object '{}'",
                set_id, operand_index, obj_ref_str
            ),
        );

        return Ok(SetOperand::ObjectRef(obj_ref_str.to_string()));
    }

    // Handle SetRef operand (SET_REF set_id)
    if let Some(set_ref_str) = operand_json
        .get("SetRef")
        .and_then(|set_ref| set_ref.as_str())
    {
        let _ = log_consumer_debug(
            "Parsing SetRef operand",
            &[("set_id", set_id), ("referenced_set_id", set_ref_str)],
        );

        // Structural validation (identifier format) - OK to keep
        if !is_valid_identifier(set_ref_str) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid set identifier '{}' in operand {} of set '{}'",
                    set_ref_str, operand_index, set_id
                ),
                &[("set_id", set_id), ("referenced_set_id", set_ref_str)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid set identifier format: '{}'", set_ref_str),
            });
        }

        // REMOVED: Immediate circular dependency check - defer to DAG phase
        // The DAG resolver will detect all circular dependencies comprehensively

        // NEW: Defer set reference validation to DAG phase
        context.add_deferred_validation(crate::types::resolution_context::DeferredValidation {
            validation_type: crate::types::resolution_context::ValidationType::SetReference,
            source_symbol: format!("SET:{}", set_id),
            target_symbol: set_ref_str.to_string(),
            context: format!(
                "Set '{}' operand {} references set '{}'",
                set_id, operand_index, set_ref_str
            ),
        });

        return Ok(SetOperand::SetRef(set_ref_str.to_string()));
    }

    // Handle InlineObject operand
    if let Some(inline_object_json) = operand_json.get("InlineObject") {
        let _ = log_consumer_debug("Parsing InlineObject operand", &[("set_id", set_id)]);

        // For now, create a simple object declaration manually since the function is private
        // TODO: Refactor object parser to expose public interface for inline objects
        let object_id = inline_object_json
            .get("id")
            .and_then(|id| id.as_str())
            .unwrap_or("inline_object");

        let object_declaration = ObjectDeclaration {
            identifier: object_id.to_string(),
            elements: Vec::new(), // Simplified for now - full parsing would require exposing more functions
            is_global: false,
        };

        let _ = log_consumer_debug(
            "Created simplified inline object (full parsing requires refactoring)",
            &[
                ("set_id", set_id),
                ("object_id", &object_declaration.identifier),
                ("operand_index", &operand_index.to_string()),
            ],
        );

        // NEW: If inline object has variable references, collect them for DAG
        if object_declaration.has_variable_references() {
            for var_ref in object_declaration.get_variable_references() {
                context.defer_variable_reference_validation(
                    format!("SET:{}:InlineObject:{}", set_id, object_id),
                    var_ref,
                    format!(
                        "Inline object '{}' in set '{}' references variable",
                        object_id, set_id
                    ),
                );
            }
        }

        return Ok(SetOperand::InlineObject(object_declaration));
    }

    // Unknown operand type
    let available_keys = operand_json
        .as_object()
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);

    let _ = log_consumer_error(
        consumer_codes::CONSUMER_FORMAT_ERROR,
        &format!(
            "Unknown set operand type at index {} in set '{}'",
            operand_index, set_id
        ),
        &[
            ("set_id", set_id),
            ("operand_index", &operand_index.to_string()),
            ("available_keys", &available_keys.join(",")),
        ],
    );

    Err(ResolutionError::InvalidInput {
        message: format!(
            "Unknown set operand type in set '{}' at index {}",
            set_id, operand_index
        ),
    })
}

/// Parse optional set filter from JSON with deferred state reference validation
/// EBNF: set_filter ::= "FILTER" space filter_action? statement_end filter_references "FILTER_END"
fn parse_set_filter(
    set_json: &serde_json::Value,
    set_id: &str,
    context: &mut ResolutionContext, // NEW: Context for deferred validation
) -> Result<Option<FilterSpec>, ResolutionError> {
    let _ = log_consumer_debug("Parsing set filter", &[("set_id", set_id)]);

    if let Some(filter_json) = set_json.get("filter") {
        if !filter_json.is_null() {
            let _ = log_consumer_debug("Found filter in set", &[("set_id", set_id)]);

            match filter::parse_filter_from_set_context(filter_json, set_id) {
                Ok(filter_spec) => {
                    let _ = log_consumer_debug(
                        "Successfully parsed set filter",
                        &[
                            ("set_id", set_id),
                            ("filter_action", filter_spec.action.as_str()),
                            ("state_ref_count", &filter_spec.state_refs.len().to_string()),
                        ],
                    );

                    // NEW: Defer filter state reference validation to DAG phase
                    for state_ref in &filter_spec.state_refs {
                        context.add_deferred_validation(crate::types::resolution_context::DeferredValidation {
                            validation_type: crate::types::resolution_context::ValidationType::FilterStateReference,
                            source_symbol: format!("SET:{}:FILTER", set_id),
                            target_symbol: state_ref.clone(),
                            context: format!("Set '{}' filter references state '{}'", set_id, state_ref),
                        });
                    }

                    return Ok(Some(filter_spec));
                }
                Err(e) => {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                        &format!("Failed to parse filter for set '{}': {:?}", set_id, e),
                        &[("set_id", set_id)],
                    );
                    return Err(ResolutionError::InvalidInput {
                        message: format!("Failed to parse filter for set '{}'", set_id),
                    });
                }
            }
        }
    }

    let _ = log_consumer_debug("No filter found for set", &[("set_id", set_id)]);

    Ok(None)
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

/// Extract set references from JSON (used by other parsers for dependency analysis)
pub fn extract_set_references_from_json(
    refs_json: &serde_json::Value,
    context_name: &str,
) -> Result<Vec<String>, ResolutionError> {
    let _ = log_consumer_debug("Extracting set references", &[("context", context_name)]);

    let refs_array = refs_json.as_array().ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!(
                "Set references is not an array in context '{}'",
                context_name
            ),
            &[("context", context_name)],
        );
        ResolutionError::InvalidInput {
            message: format!("Set references must be an array in {}", context_name),
        }
    })?;

    let mut set_refs = Vec::new();

    for (index, ref_json) in refs_array.iter().enumerate() {
        let set_id = if let Some(set_str) = ref_json.as_str() {
            // Direct string reference
            set_str.to_string()
        } else if let Some(set_ref_obj) = ref_json.as_object() {
            // SET_REF object format
            set_ref_obj
                .get("set_id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_FORMAT_ERROR,
                        &format!(
                            "Set reference object at index {} missing 'set_id' in context '{}'",
                            index, context_name
                        ),
                        &[("context", context_name), ("index", &index.to_string())],
                    );
                    ResolutionError::InvalidInput {
                        message: format!("Set reference object missing set_id in {}", context_name),
                    }
                })?
                .to_string()
        } else {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Invalid set reference format at index {} in context '{}'",
                    index, context_name
                ),
                &[("context", context_name), ("index", &index.to_string())],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Invalid set reference format at index {} in {}",
                    index, context_name
                ),
            });
        };

        // Structural validation only - defer existence validation
        if !is_valid_identifier(&set_id) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid set ID format '{}' at index {} in context '{}'",
                    set_id, index, context_name
                ),
                &[
                    ("context", context_name),
                    ("set_id", &set_id),
                    ("index", &index.to_string()),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid set ID format '{}' in {}", set_id, context_name),
            });
        }

        set_refs.push(set_id);
    }

    let _ = log_consumer_debug(
        "Set references extraction completed",
        &[
            ("context", context_name),
            ("reference_count", &set_refs.len().to_string()),
        ],
    );

    Ok(set_refs)
}

// =============================================================================
// NEW: DAG-aware dependency collection methods (no validation)
// =============================================================================

/// Check if a set operation has variable references in any of its operands (collection only)
pub fn has_variable_references_in_set(set_operation: &SetOperation) -> bool {
    set_operation.has_variable_references()
}

/// Get all variable references from a set operation (for dependency analysis)
pub fn get_variable_references_from_set(set_operation: &SetOperation) -> Vec<String> {
    set_operation.get_variable_references()
}

/// Get all object references from a set operation (for dependency analysis)
pub fn get_object_references_from_set(set_operation: &SetOperation) -> Vec<String> {
    set_operation.get_object_references()
}

/// Get all set references from a set operation (for dependency analysis)
pub fn get_set_references_from_set(set_operation: &SetOperation) -> Vec<String> {
    set_operation.get_set_references()
}

/// Get all filter state dependencies from a set operation (for dependency analysis)
pub fn get_filter_dependencies_from_set(set_operation: &SetOperation) -> Vec<String> {
    set_operation.get_filter_state_dependencies()
}

/// MODIFIED: Validate set operation structure only (no cross-reference validation)
pub fn validate_set_operation_structure(
    set_operation: &SetOperation,
) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating set operation structure",
        &[
            ("set_id", &set_operation.set_id),
            ("operation", set_operation.operation.as_str()),
        ],
    );

    // Use the built-in validation method (structural validation only)
    if let Err(validation_error) = set_operation.validate() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Set operation structure validation failed for '{}': {}",
                set_operation.set_id, validation_error
            ),
            &[("set_id", &set_operation.set_id)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Set '{}' structure validation failed: {}",
                set_operation.set_id, validation_error
            ),
        });
    }

    let _ = log_consumer_debug(
        "Set operation structure validation completed successfully",
        &[("set_id", &set_operation.set_id)],
    );

    Ok(())
}

/// NEW: Collect all dependencies from a set operation for DAG construction
pub fn collect_set_dependencies(set_operation: &SetOperation, context: &mut ResolutionContext) {
    let set_id = &set_operation.set_id;

    // Collect object references
    for obj_ref in set_operation.get_object_references() {
        context.defer_object_reference_validation(
            format!("SET:{}", set_id),
            obj_ref,
            format!("Set '{}' depends on object", set_id),
        );
    }

    // Collect set references
    for set_ref in set_operation.get_set_references() {
        context.add_deferred_validation(crate::types::resolution_context::DeferredValidation {
            validation_type: crate::types::resolution_context::ValidationType::SetReference,
            source_symbol: format!("SET:{}", set_id),
            target_symbol: set_ref,
            context: format!("Set '{}' depends on set", set_id),
        });
    }

    // Collect filter state dependencies
    for state_ref in set_operation.get_filter_state_dependencies() {
        context.add_deferred_validation(crate::types::resolution_context::DeferredValidation {
            validation_type: crate::types::resolution_context::ValidationType::FilterStateReference,
            source_symbol: format!("SET:{}:FILTER", set_id),
            target_symbol: state_ref,
            context: format!("Set '{}' filter depends on state", set_id),
        });
    }

    // Collect variable references from inline objects
    for var_ref in set_operation.get_variable_references() {
        context.defer_variable_reference_validation(
            format!("SET:{}", set_id),
            var_ref,
            format!("Set '{}' contains variable reference", set_id),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::set::SetOperationType;
    use serde_json::json;

    #[test]
    fn test_parse_union_set_operation() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_union",
            "operation": "union",
            "operands": [
                {"ObjectRef": "object1"},
                {"ObjectRef": "object2"}
            ],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_ok());

        let set_operation = result.unwrap();
        assert_eq!(set_operation.set_id, "test_union");
        assert_eq!(set_operation.operation, SetOperationType::Union);
        assert_eq!(set_operation.operands.len(), 2);
        assert!(set_operation.filter.is_none());

        // Should have deferred validations for object references
        assert!(!context.deferred_validations.is_empty());
    }

    #[test]
    fn test_parse_intersection_set_operation() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_intersection",
            "operation": "intersection",
            "operands": [
                {"ObjectRef": "object1"},
                {"SetRef": "other_set"}
            ],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_ok());

        let set_operation = result.unwrap();
        assert_eq!(set_operation.operation, SetOperationType::Intersection);
        assert_eq!(set_operation.operands.len(), 2);

        // Should have deferred validations for both object and set references
        assert!(!context.deferred_validations.is_empty());
    }

    #[test]
    fn test_parse_complement_set_operation() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_complement",
            "operation": "complement",
            "operands": [
                {"ObjectRef": "object1"},
                {"ObjectRef": "object2"}
            ],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_ok());

        let set_operation = result.unwrap();
        assert_eq!(set_operation.operation, SetOperationType::Complement);
        assert_eq!(set_operation.operands.len(), 2);
    }

    #[test]
    fn test_parse_set_operation_with_filter() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_filtered",
            "operation": "union",
            "operands": [
                {"ObjectRef": "object1"}
            ],
            "filter": {
                "action": "include",
                "state_refs": ["state1", "state2"]
            }
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_ok());

        let set_operation = result.unwrap();
        assert!(set_operation.filter.is_some());

        // Should have deferred validations for filter state references
        let filter_validations: Vec<_> = context
            .deferred_validations
            .iter()
            .filter(|v| {
                matches!(
                    v.validation_type,
                    crate::types::resolution_context::ValidationType::FilterStateReference
                )
            })
            .collect();
        assert!(!filter_validations.is_empty());
    }

    #[test]
    fn test_parse_set_operation_invalid_operand_count() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_invalid",
            "operation": "intersection",
            "operands": [
                {"ObjectRef": "object1"}
            ],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_err()); // Intersection requires at least 2 operands
    }

    #[test]
    fn test_parse_set_operation_self_reference_allowed_in_dag() {
        // Self-references are now allowed during parsing - DAG will detect cycles
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "self_referencing",
            "operation": "union",
            "operands": [
                {"SetRef": "self_referencing"}
            ],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        // Should succeed during parsing - DAG resolution will detect the cycle
        assert!(result.is_ok());

        // Should have deferred validation for the set reference
        assert!(!context.deferred_validations.is_empty());
    }

    #[test]
    fn test_parse_set_operation_empty_operands() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_empty",
            "operation": "union",
            "operands": [],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_err()); // Empty operands not allowed (structural error)
    }

    #[test]
    fn test_parse_set_operation_invalid_operation_type() {
        let mut context = ResolutionContext::new();
        let set_json = json!({
            "set_id": "test_invalid_op",
            "operation": "invalid_operation",
            "operands": [
                {"ObjectRef": "object1"}
            ],
            "filter": null
        });

        let result = parse_set_operation_from_json(&set_json, &mut context);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("valid_id"));
        assert!(is_valid_identifier("_underscore"));
        assert!(is_valid_identifier("mixed123"));
        assert!(!is_valid_identifier("123invalid"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("invalid-dash"));
    }

    #[test]
    fn test_parse_object_ref_operand() {
        let mut context = ResolutionContext::new();
        let operand_json = json!({"ObjectRef": "test_object"});
        let result = parse_set_operand_from_json(&operand_json, "test_set", 0, &mut context);

        assert!(result.is_ok());
        if let SetOperand::ObjectRef(obj_id) = result.unwrap() {
            assert_eq!(obj_id, "test_object");
        } else {
            panic!("Expected ObjectRef operand");
        }

        // Should have deferred validation for object reference
        assert!(!context.deferred_validations.is_empty());
    }

    #[test]
    fn test_parse_set_ref_operand() {
        let mut context = ResolutionContext::new();
        let operand_json = json!({"SetRef": "other_set"});
        let result = parse_set_operand_from_json(&operand_json, "test_set", 0, &mut context);

        assert!(result.is_ok());
        if let SetOperand::SetRef(set_id) = result.unwrap() {
            assert_eq!(set_id, "other_set");
        } else {
            panic!("Expected SetRef operand");
        }

        // Should have deferred validation for set reference
        assert!(!context.deferred_validations.is_empty());
    }

    #[test]
    fn test_collect_set_dependencies() {
        let mut context = ResolutionContext::new();
        let set_operation = SetOperation {
            set_id: "test_set".to_string(),
            operation: SetOperationType::Union,
            operands: vec![
                SetOperand::ObjectRef("obj1".to_string()),
                SetOperand::SetRef("set2".to_string()),
            ],
            filter: None,
        };

        collect_set_dependencies(&set_operation, &mut context);

        // Should have collected dependencies without validating existence
        assert!(!context.deferred_validations.is_empty());

        let has_object_validation = context.deferred_validations.iter().any(|v| {
            matches!(
                v.validation_type,
                crate::types::resolution_context::ValidationType::ObjectReference
            )
        });
        let has_set_validation = context.deferred_validations.iter().any(|v| {
            matches!(
                v.validation_type,
                crate::types::resolution_context::ValidationType::SetReference
            )
        });

        assert!(has_object_validation);
        assert!(has_set_validation);
    }
}
