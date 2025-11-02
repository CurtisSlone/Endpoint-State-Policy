//! SET operation execution for ICS SET blocks
//! Handles union, intersection, and complement operations in definition order

use super::error::ResolutionError;
use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::types::object::{ResolvedObject, ResolvedObjectElement};
use crate::types::resolution_context::ResolutionContext;
use crate::types::set::{
    ResolvedSetOperand, ResolvedSetOperation, SetOperand, SetOperation, SetOperationType,
};
use std::collections::HashSet;

/// Execute a SET operation and store result in resolution context
pub fn execute_set_operation(
    set_operation: &SetOperation,
    context: &mut ResolutionContext,
) -> Result<ResolvedSetOperation, ResolutionError> {
    let _ = log_consumer_debug(
        "Executing SET operation",
        &[
            ("set_id", &set_operation.set_id),
            ("operation_type", set_operation.operation.as_str()),
            ("operand_count", &set_operation.operands.len().to_string()),
            ("has_filter", &set_operation.has_filter().to_string()),
        ],
    );

    // Validate operand count (already done by parser, but double-check)
    if let Err(validation_error) = set_operation
        .operation
        .validate_operand_count(set_operation.operands.len())
    {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_VALIDATION_ERROR,
            &format!(
                "SET operation '{}' validation failed: {}",
                set_operation.set_id, validation_error
            ),
            &[("set_id", &set_operation.set_id)],
        );
        return Err(ResolutionError::SetOperationFailed {
            set_id: set_operation.set_id.clone(),
            reason: validation_error,
        });
    }

    // Resolve all operands to concrete objects
    let mut resolved_operands = Vec::new();
    let mut operand_object_lists = Vec::new();

    for (operand_index, operand) in set_operation.operands.iter().enumerate() {
        let _ = log_consumer_debug(
            "Resolving SET operand",
            &[
                ("set_id", &set_operation.set_id),
                ("operand_index", &operand_index.to_string()),
                ("operand_type", operand.operand_type_name()),
            ],
        );

        let (resolved_operand, objects) =
            resolve_set_operand(operand, &set_operation.set_id, operand_index, context)?;

        resolved_operands.push(resolved_operand);
        operand_object_lists.push(objects);
    }

    // Execute the specific SET operation to validate the logic
    match set_operation.operation {
        SetOperationType::Union => execute_union(&operand_object_lists, &set_operation.set_id)?,
        SetOperationType::Intersection => {
            execute_intersection(&operand_object_lists, &set_operation.set_id)?
        }
        SetOperationType::Complement => {
            execute_complement(&operand_object_lists, &set_operation.set_id)?
        }
    };

    // Resolve filter if present (using existing filter resolution logic)
    let resolved_filter = if let Some(filter) = &set_operation.filter {
        let _ = log_consumer_debug(
            "Resolving SET filter",
            &[
                ("set_id", &set_operation.set_id),
                ("filter_action", filter.action.as_str()),
                ("state_refs_count", &filter.state_refs.len().to_string()),
            ],
        );

        // Validate that all state references exist in global states
        for state_ref in &filter.state_refs {
            if !context
                .global_states
                .iter()
                .any(|s| s.identifier == *state_ref)
            {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_VALIDATION_ERROR,
                    &format!(
                        "SET filter references undefined global state '{}' in set '{}'",
                        state_ref, set_operation.set_id
                    ),
                    &[("set_id", &set_operation.set_id), ("state_ref", state_ref)],
                );
                return Err(ResolutionError::UndefinedGlobalState {
                    name: state_ref.clone(),
                    context: format!("SET filter in set '{}'", set_operation.set_id),
                });
            }
        }

        Some(crate::types::filter::ResolvedFilterSpec::new(
            filter.action,
            filter.state_refs.clone(),
        ))
    } else {
        None
    };

    let resolved_set = ResolvedSetOperation {
        set_id: set_operation.set_id.clone(),
        operation: set_operation.operation,
        resolved_operands,
        resolved_filter,
    };

    let _ = log_consumer_debug(
        "SET operation execution completed",
        &[
            ("set_id", &set_operation.set_id),
            ("operation_type", set_operation.operation.as_str()),
            (
                "resolved_operands_count",
                &resolved_set.resolved_operands.len().to_string(),
            ),
            (
                "has_resolved_filter",
                &resolved_set.resolved_filter.is_some().to_string(),
            ),
        ],
    );

    Ok(resolved_set)
}

/// Resolve a single SET operand to concrete objects
fn resolve_set_operand(
    operand: &SetOperand,
    set_id: &str,
    operand_index: usize,
    context: &ResolutionContext,
) -> Result<(ResolvedSetOperand, Vec<ResolvedObject>), ResolutionError> {
    let _ = log_consumer_debug(
        "Resolving SET operand",
        &[
            ("set_id", set_id),
            ("operand_index", &operand_index.to_string()),
            ("operand_type", operand.operand_type_name()),
        ],
    );

    match operand {
        SetOperand::ObjectRef(object_id) => {
            let _ = log_consumer_debug(
                "Resolving OBJECT_REF operand",
                &[("object_id", object_id), ("set_id", set_id)],
            );

            // Look for object in resolved global objects
            if let Some(resolved_object) = context.resolved_global_objects.get(object_id) {
                let _ = log_consumer_debug(
                    "OBJECT_REF resolved successfully",
                    &[("object_id", object_id), ("set_id", set_id)],
                );

                Ok((
                    ResolvedSetOperand::ObjectRef(object_id.clone()),
                    vec![resolved_object.clone()],
                ))
            } else {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_VALIDATION_ERROR,
                    &format!(
                        "OBJECT_REF '{}' in SET '{}' not found in resolved global objects",
                        object_id, set_id
                    ),
                    &[
                        ("object_id", object_id),
                        ("set_id", set_id),
                        (
                            "available_objects",
                            &context
                                .resolved_global_objects
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                    ],
                );

                Err(ResolutionError::UndefinedGlobalObject {
                    name: object_id.clone(),
                    context: format!("SET operand in set '{}'", set_id),
                })
            }
        }

        SetOperand::SetRef(referenced_set_id) => {
            let _ = log_consumer_debug(
                "Resolving SET_REF operand",
                &[("referenced_set_id", referenced_set_id), ("set_id", set_id)],
            );

            // Look for set in resolved sets (must be resolved before this set)
            if let Some(resolved_set) = context.resolved_sets.get(referenced_set_id) {
                // Extract objects from the resolved set's operands
                let mut objects = Vec::new();
                for resolved_operand in &resolved_set.resolved_operands {
                    match resolved_operand {
                        ResolvedSetOperand::ObjectRef(obj_id) => {
                            if let Some(resolved_obj) = context.resolved_global_objects.get(obj_id)
                            {
                                objects.push(resolved_obj.clone());
                            }
                        }
                        ResolvedSetOperand::InlineObject(resolved_obj) => {
                            objects.push(resolved_obj.clone());
                        }
                        ResolvedSetOperand::SetRef(_) => {
                            // Nested set reference - this would require recursive resolution
                            // For now, log a warning and skip
                            let _ = log_consumer_debug(
                                "Nested SET_REF detected - recursive resolution not fully implemented",
                                &[("nested_set_ref", referenced_set_id)],
                            );
                        }
                    }
                }

                let _ = log_consumer_debug(
                    "SET_REF resolved successfully",
                    &[
                        ("referenced_set_id", referenced_set_id),
                        ("resolved_objects", &objects.len().to_string()),
                    ],
                );

                Ok((
                    ResolvedSetOperand::SetRef(referenced_set_id.clone()),
                    objects,
                ))
            } else {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_VALIDATION_ERROR,
                    &format!(
                        "SET_REF '{}' in SET '{}' not found in resolved sets",
                        referenced_set_id, set_id
                    ),
                    &[
                        ("referenced_set_id", referenced_set_id),
                        ("set_id", set_id),
                        (
                            "available_sets",
                            &context
                                .resolved_sets
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                    ],
                );

                Err(ResolutionError::UndefinedSet {
                    name: referenced_set_id.clone(),
                    context: format!("SET operand in set '{}'", set_id),
                })
            }
        }

        SetOperand::InlineObject(object_decl) => {
            let _ = log_consumer_debug(
                "Resolving InlineObject operand",
                &[
                    ("object_id", &object_decl.identifier),
                    ("set_id", set_id),
                    ("element_count", &object_decl.elements.len().to_string()),
                ],
            );

            // Resolve inline object using existing object resolution logic
            let resolved_object = resolve_inline_object(object_decl, context)?;

            let _ = log_consumer_debug(
                "InlineObject resolved successfully",
                &[
                    ("object_id", &object_decl.identifier),
                    (
                        "resolved_elements",
                        &resolved_object.resolved_elements.len().to_string(),
                    ),
                ],
            );

            Ok((
                ResolvedSetOperand::InlineObject(resolved_object.clone()),
                vec![resolved_object],
            ))
        }
    }
}

/// Resolve inline object using existing field resolution patterns
fn resolve_inline_object(
    object_decl: &crate::types::object::ObjectDeclaration,
    context: &ResolutionContext,
) -> Result<ResolvedObject, ResolutionError> {
    let _ = log_consumer_debug(
        "Resolving inline object fields",
        &[
            ("object_id", &object_decl.identifier),
            ("element_count", &object_decl.elements.len().to_string()),
        ],
    );

    let mut resolved_elements = Vec::new();

    // For now, only handle Field elements (follow existing patterns)
    for element in &object_decl.elements {
        match element {
            crate::types::object::ObjectElement::Field { name, value } => {
                // Resolve the field value using existing field resolver logic
                let resolved_value = match value {
                    crate::types::common::Value::String(s) => {
                        crate::types::common::ResolvedValue::String(s.clone())
                    }
                    crate::types::common::Value::Integer(i) => {
                        crate::types::common::ResolvedValue::Integer(*i)
                    }
                    crate::types::common::Value::Float(f) => {
                        crate::types::common::ResolvedValue::Float(*f)
                    }
                    crate::types::common::Value::Boolean(b) => {
                        crate::types::common::ResolvedValue::Boolean(*b)
                    }
                    crate::types::common::Value::Variable(var_name) => {
                        // Resolve variable reference
                        if let Some(resolved_var) = context.resolved_variables.get(var_name) {
                            resolved_var.value.clone()
                        } else {
                            return Err(ResolutionError::UndefinedVariable {
                                name: var_name.clone(),
                                context: format!(
                                    "Inline object '{}' field '{}'",
                                    object_decl.identifier, name
                                ),
                            });
                        }
                    }
                };

                resolved_elements.push(ResolvedObjectElement::Field {
                    name: name.clone(),
                    value: resolved_value,
                });
            }
            _ => {
                // For other element types, log and skip for now
                let _ = log_consumer_debug(
                    "Skipping unsupported inline object element type",
                    &[
                        ("element_type", element.element_type_name()),
                        ("object_id", &object_decl.identifier),
                    ],
                );
            }
        }
    }

    Ok(ResolvedObject {
        identifier: object_decl.identifier.clone(),
        resolved_elements,
        is_global: false, // Inline objects are never global
    })
}

/// Execute UNION operation - combine all unique objects
fn execute_union(
    object_lists: &[Vec<ResolvedObject>],
    set_id: &str,
) -> Result<Vec<ResolvedObject>, ResolutionError> {
    let _ = log_consumer_debug(
        "Executing UNION operation",
        &[
            ("set_id", set_id),
            ("operand_lists_count", &object_lists.len().to_string()),
        ],
    );

    let mut result_objects = Vec::new();
    let mut seen_identifiers = HashSet::new();

    // Combine all objects, maintaining uniqueness by identifier
    for object_list in object_lists {
        for object in object_list {
            if !seen_identifiers.contains(&object.identifier) {
                seen_identifiers.insert(object.identifier.clone());
                result_objects.push(object.clone());

                let _ = log_consumer_debug(
                    "Added object to UNION result",
                    &[("object_id", &object.identifier), ("set_id", set_id)],
                );
            }
        }
    }

    let _ = log_consumer_debug(
        "UNION operation completed",
        &[
            ("set_id", set_id),
            ("result_count", &result_objects.len().to_string()),
            ("unique_objects", &seen_identifiers.len().to_string()),
        ],
    );

    Ok(result_objects)
}

/// Execute INTERSECTION operation - objects present in ALL operands
fn execute_intersection(
    object_lists: &[Vec<ResolvedObject>],
    set_id: &str,
) -> Result<Vec<ResolvedObject>, ResolutionError> {
    let _ = log_consumer_debug(
        "Executing INTERSECTION operation",
        &[
            ("set_id", set_id),
            ("operand_lists_count", &object_lists.len().to_string()),
        ],
    );

    if object_lists.is_empty() {
        return Ok(Vec::new());
    }

    if object_lists.len() == 1 {
        return Ok(object_lists[0].clone());
    }

    // Start with first operand's objects
    let mut result_objects = Vec::new();
    let first_list = &object_lists[0];

    for object in first_list {
        // Check if this object exists in ALL other operands
        let exists_in_all = object_lists[1..].iter().all(|other_list| {
            other_list
                .iter()
                .any(|other_obj| other_obj.identifier == object.identifier)
        });

        if exists_in_all {
            result_objects.push(object.clone());

            let _ = log_consumer_debug(
                "Object found in all operands for INTERSECTION",
                &[("object_id", &object.identifier), ("set_id", set_id)],
            );
        }
    }

    let _ = log_consumer_debug(
        "INTERSECTION operation completed",
        &[
            ("set_id", set_id),
            ("result_count", &result_objects.len().to_string()),
        ],
    );

    Ok(result_objects)
}

/// Execute COMPLEMENT operation - objects in first but not in second (A - B)
fn execute_complement(
    object_lists: &[Vec<ResolvedObject>],
    set_id: &str,
) -> Result<Vec<ResolvedObject>, ResolutionError> {
    let _ = log_consumer_debug(
        "Executing COMPLEMENT operation",
        &[
            ("set_id", set_id),
            ("operand_lists_count", &object_lists.len().to_string()),
        ],
    );

    if object_lists.len() != 2 {
        return Err(ResolutionError::SetOperationFailed {
            set_id: set_id.to_string(),
            reason: format!(
                "COMPLEMENT requires exactly 2 operands, got {}",
                object_lists.len()
            ),
        });
    }

    let first_list = &object_lists[0];
    let second_list = &object_lists[1];

    // Build set of identifiers from second operand
    let second_identifiers: HashSet<String> = second_list
        .iter()
        .map(|obj| obj.identifier.clone())
        .collect();

    // Keep objects from first operand that are NOT in second operand
    let mut result_objects = Vec::new();

    for object in first_list {
        if !second_identifiers.contains(&object.identifier) {
            result_objects.push(object.clone());

            let _ = log_consumer_debug(
                "Object included in COMPLEMENT result (not in second operand)",
                &[("object_id", &object.identifier), ("set_id", set_id)],
            );
        }
    }

    let _ = log_consumer_debug(
        "COMPLEMENT operation completed",
        &[
            ("set_id", set_id),
            ("first_operand_count", &first_list.len().to_string()),
            ("second_operand_count", &second_list.len().to_string()),
            ("result_count", &result_objects.len().to_string()),
        ],
    );

    Ok(result_objects)
}

/// Execute all SET operations in definition order
pub fn resolve_set_operations(context: &mut ResolutionContext) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Starting SET operations resolution",
        &[(
            "set_operations_count",
            &context.set_operations.len().to_string(),
        )],
    );

    // Clone set operations to avoid borrow checker issues
    let set_operations = context.set_operations.clone();

    for (set_index, set_operation) in set_operations.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing SET operation",
            &[
                ("set_index", &set_index.to_string()),
                ("set_id", &set_operation.set_id),
                ("operation_type", set_operation.operation.as_str()),
            ],
        );

        // Execute the SET operation
        let resolved_set = execute_set_operation(set_operation, context)?;

        // Store resolved set in context for subsequent SET_REF resolution
        context
            .resolved_sets
            .insert(set_operation.set_id.clone(), resolved_set);

        let _ = log_consumer_debug(
            "SET operation resolved and stored",
            &[
                ("set_id", &set_operation.set_id),
                ("stored_in_context", "true"),
            ],
        );
    }

    let _ = log_consumer_debug(
        "SET operations resolution completed",
        &[
            ("total_processed", &set_operations.len().to_string()),
            ("total_resolved", &context.resolved_sets.len().to_string()),
        ],
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::common::{ResolvedValue, Value};
    use crate::types::object::{ObjectDeclaration, ObjectElement};
    use crate::types::set::{SetOperand, SetOperation, SetOperationType};
    use crate::types::variable::ResolvedVariable;
    use crate::types::DataType;

    #[test]
    fn test_union_operation() {
        // Create test objects
        let obj1 = ResolvedObject {
            identifier: "obj1".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };
        let obj2 = ResolvedObject {
            identifier: "obj2".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };
        let obj3 = ResolvedObject {
            identifier: "obj1".to_string(), // Duplicate ID
            resolved_elements: vec![],
            is_global: true,
        };

        let object_lists = vec![
            vec![obj1.clone()],
            vec![obj2.clone(), obj3], // obj3 should be deduplicated
        ];

        let result = execute_union(&object_lists, "test_union").unwrap();

        assert_eq!(result.len(), 2); // Should have obj1 and obj2, no duplicates
        assert!(result.iter().any(|obj| obj.identifier == "obj1"));
        assert!(result.iter().any(|obj| obj.identifier == "obj2"));
    }

    #[test]
    fn test_intersection_operation() {
        // Create test objects
        let obj1 = ResolvedObject {
            identifier: "obj1".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };
        let obj2 = ResolvedObject {
            identifier: "obj2".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };
        let obj1_copy = ResolvedObject {
            identifier: "obj1".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };

        let object_lists = vec![
            vec![obj1.clone(), obj2.clone()],
            vec![
                obj1_copy,
                ResolvedObject {
                    identifier: "obj3".to_string(),
                    resolved_elements: vec![],
                    is_global: true,
                },
            ],
        ];

        let result = execute_intersection(&object_lists, "test_intersection").unwrap();

        assert_eq!(result.len(), 1); // Only obj1 is in both lists
        assert_eq!(result[0].identifier, "obj1");
    }

    #[test]
    fn test_complement_operation() {
        // Create test objects
        let obj1 = ResolvedObject {
            identifier: "obj1".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };
        let obj2 = ResolvedObject {
            identifier: "obj2".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };
        let obj1_copy = ResolvedObject {
            identifier: "obj1".to_string(),
            resolved_elements: vec![],
            is_global: true,
        };

        let object_lists = vec![
            vec![obj1.clone(), obj2.clone()], // First operand: obj1, obj2
            vec![obj1_copy],                  // Second operand: obj1
        ];

        let result = execute_complement(&object_lists, "test_complement").unwrap();

        assert_eq!(result.len(), 1); // obj2 should remain (obj1 is removed)
        assert_eq!(result[0].identifier, "obj2");
    }

    #[test]
    fn test_complement_wrong_operand_count() {
        let object_lists = vec![
            vec![ResolvedObject {
                identifier: "obj1".to_string(),
                resolved_elements: vec![],
                is_global: true,
            }],
            // Missing second operand for complement
        ];

        let result = execute_complement(&object_lists, "test_bad_complement");
        assert!(result.is_err());
    }
}
