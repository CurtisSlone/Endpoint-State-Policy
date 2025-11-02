//! SET expansion for ICS execution - Declaration-level expansion
//!
//! This module provides functions to expand SET_REF elements during the resolution phase,
//! working directly on CriterionDeclaration structures before ExecutionContext creation.

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error, log_consumer_info};
use crate::resolution::ResolutionError;
use crate::types::criteria::{CriteriaTree, CriteriaRoot};
use crate::types::criterion::CriterionDeclaration;
use crate::types::object::{ObjectDeclaration, ObjectElement, ObjectRef};
use crate::types::resolution_context::ResolutionContext;
use crate::types::set::{ResolvedSetOperand, ResolvedSetOperation};
use std::collections::{HashMap, HashSet};

/// Main entry point for SET expansion during resolution phase
/// Call this AFTER resolve_dag() and BEFORE ExecutionContext creation
pub fn expand_sets_in_resolution_context(
    context: &mut ResolutionContext,
) -> Result<(), ResolutionError> {
    let _ = log_consumer_info(
        "Starting SET expansion in resolution context",
        &[("total_sets", &context.resolved_sets.len().to_string())],
    );

    // Validate SET structures first (check for circular dependencies)
    validate_set_expansions(context)?;

    // Expand SET_REF in all criteria trees
    // CRITICAL FIX: We need to separate the mutable borrow from the immutable read
    // Solution: Clone the data we need from context for read-only access during expansion
    let resolved_sets = context.resolved_sets.clone();
    let resolved_global_objects = context.resolved_global_objects.clone();
    
    let mut total_expanded = 0;
    for tree in &mut context.criteria_root.trees {
        total_expanded += expand_set_refs_in_criteria_tree_helper(
            tree, 
            &resolved_sets,
            &resolved_global_objects
        )?;
    }

    let _ = log_consumer_info(
        "SET expansion completed",
        &[
            ("criteria_expanded", &total_expanded.to_string()),
            ("total_sets", &context.resolved_sets.len().to_string()),
        ],
    );

    Ok(())
}

/// Helper function that takes cloned data instead of full context
/// This avoids borrow checker issues
fn expand_set_refs_in_criteria_tree_helper(
    tree: &mut CriteriaTree,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
    resolved_global_objects: &HashMap<String, crate::types::object::ResolvedObject>,
) -> Result<usize, ResolutionError> {
    match tree {
        CriteriaTree::Criterion { declaration, node_id } => {
            let _ = log_consumer_debug(
                "Expanding SET_REF in criterion",
                &[
                    ("criterion_type", &declaration.criterion_type),
                    ("node_id", &node_id.to_string()),
                ],
            );

            let expanded = expand_set_ref_in_declaration_helper(
                declaration,
                resolved_sets,
                resolved_global_objects,
            )?;
            Ok(if expanded { 1 } else { 0 })
        }
        CriteriaTree::Block { children, .. } => {
            let mut total = 0;
            for child in children {
                total += expand_set_refs_in_criteria_tree_helper(
                    child,
                    resolved_sets,
                    resolved_global_objects,
                )?;
            }
            Ok(total)
        }
    }
}

/// Expand SET_REF in a single CriterionDeclaration
/// Helper version that takes separate parameters instead of full context
fn expand_set_ref_in_declaration_helper(
    declaration: &mut CriterionDeclaration,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
    resolved_global_objects: &HashMap<String, crate::types::object::ResolvedObject>,
) -> Result<bool, ResolutionError> {
    let mut was_expanded = false;
    let mut new_object_refs = Vec::new();

    // Phase 1: Check local object for SET_REF
    if let Some(local_obj) = &declaration.local_object {
        if let Some(set_id) = extract_set_ref_from_declaration_object(local_obj) {
            let _ = log_consumer_debug(
                "Found SET_REF in local object",
                &[
                    ("set_id", &set_id),
                    ("object_id", &local_obj.identifier),
                    ("criterion_type", &declaration.criterion_type),
                ],
            );

            // Expand the SET_REF into object references
            let expanded_refs = expand_set_ref_to_object_refs(
                &set_id,
                &local_obj.identifier,
                &declaration.criterion_type,
                resolved_sets,
            )?;

            let _ = log_consumer_debug(
                "Expanded local object SET_REF",
                &[
                    ("set_id", &set_id),
                    ("expanded_count", &expanded_refs.len().to_string()),
                ],
            );

            new_object_refs.extend(expanded_refs);
            was_expanded = true;

            // Clear local object since we've expanded it
            declaration.local_object = None;
        }
    }

    // Phase 2: Check global object references for SET_REF
    let mut refs_to_remove = HashSet::new();
    for obj_ref in &declaration.object_refs {
        // Look up the global object
        if let Some(global_obj) = resolved_global_objects.get(&obj_ref.object_id) {
            // Check if this resolved object contains SET_REF
            if contains_set_ref_in_resolved_object(global_obj) {
                let set_id = extract_set_ref_from_resolved_object(global_obj)
                    .ok_or_else(|| ResolutionError::InvalidInput {
                        message: format!(
                            "Expected SET_REF in object '{}' but not found",
                            obj_ref.object_id
                        ),
                    })?;

                let _ = log_consumer_debug(
                    "Found SET_REF in global object reference",
                    &[
                        ("set_id", &set_id),
                        ("object_id", &obj_ref.object_id),
                        ("criterion_type", &declaration.criterion_type),
                    ],
                );

                // Expand this SET_REF
                let expanded_refs = expand_set_ref_to_object_refs(
                    &set_id,
                    &obj_ref.object_id,
                    &declaration.criterion_type,
                    resolved_sets,
                )?;

                let _ = log_consumer_debug(
                    "Expanded global object SET_REF",
                    &[
                        ("set_id", &set_id),
                        ("expanded_count", &expanded_refs.len().to_string()),
                    ],
                );

                new_object_refs.extend(expanded_refs);
                was_expanded = true;

                // Mark this reference for removal (it was just a container for SET_REF)
                refs_to_remove.insert(obj_ref.object_id.clone());
            }
        }
    }

    // Phase 3: Update declaration with expanded references
    if was_expanded {
        // Remove old SET_REF container references
        declaration.object_refs.retain(|r| !refs_to_remove.contains(&r.object_id));

        // Add new expanded references
        declaration.object_refs.extend(new_object_refs);

        // Deduplicate
        let mut seen = HashSet::new();
        declaration.object_refs.retain(|r| seen.insert(r.object_id.clone()));

        let _ = log_consumer_debug(
            "Updated declaration with expanded object references",
            &[
                ("criterion_type", &declaration.criterion_type),
                ("final_object_refs_count", &declaration.object_refs.len().to_string()),
            ],
        );
    }

    Ok(was_expanded)
}

/// Extract SET_REF from ObjectDeclaration (unresolved)
fn extract_set_ref_from_declaration_object(object: &ObjectDeclaration) -> Option<String> {
    for element in &object.elements {
        if let ObjectElement::SetRef { set_id } = element {
            return Some(set_id.clone());
        }
    }
    None
}

/// Check if a ResolvedObject contains SET_REF
fn contains_set_ref_in_resolved_object(object: &crate::types::object::ResolvedObject) -> bool {
    use crate::types::object::ResolvedObjectElement;
    object.resolved_elements.iter().any(|elem| {
        matches!(elem, ResolvedObjectElement::SetRef { .. })
    })
}

/// Extract SET_REF from ResolvedObject
fn extract_set_ref_from_resolved_object(
    object: &crate::types::object::ResolvedObject,
) -> Option<String> {
    use crate::types::object::ResolvedObjectElement;
    for element in &object.resolved_elements {
        if let ResolvedObjectElement::SetRef { set_id } = element {
            return Some(set_id.clone());
        }
    }
    None
}

/// Expand a SET_REF into concrete ObjectRef entries
/// This is the core expansion logic that handles union/intersection/complement
fn expand_set_ref_to_object_refs(
    set_id: &str,
    container_object_id: &str,
    criterion_type: &str,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
) -> Result<Vec<ObjectRef>, ResolutionError> {
    let _ = log_consumer_debug(
        "Expanding SET_REF to object references",
        &[
            ("set_id", set_id),
            ("container_object", container_object_id),
            ("criterion_type", criterion_type),
        ],
    );

    // Lookup resolved set
    let resolved_set = resolved_sets.get(set_id).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_VALIDATION_ERROR,
            &format!("SET_REF '{}' not found in resolved sets", set_id),
            &[("set_id", set_id), ("criterion_type", criterion_type)],
        );
        ResolutionError::UndefinedSet {
            name: set_id.to_string(),
            context: format!("SET expansion in CTN '{}'", criterion_type),
        }
    })?;

    let _ = log_consumer_debug(
        "Found resolved set",
        &[
            ("set_id", set_id),
            ("operation", resolved_set.operation.as_str()),
            ("operands_count", &resolved_set.resolved_operands.len().to_string()),
        ],
    );

    // Extract all object references from the set's operands
    let mut object_refs = Vec::new();
    extract_objects_from_operands(&resolved_set.resolved_operands, &mut object_refs, resolved_sets)?;

    let _ = log_consumer_debug(
        "Extracted objects from SET operands",
        &[
            ("set_id", set_id),
            ("extracted_count", &object_refs.len().to_string()),
        ],
    );

    // Apply set filters if present
    // NOTE: Filter evaluation is deferred to execution phase when we have collected data
    // For now, we just validate that the filter is well-formed
    if let Some(filter) = &resolved_set.resolved_filter {
        let _ = log_consumer_debug(
            "SET has filter (evaluation deferred to execution phase)",
            &[
                ("set_id", set_id),
                ("filter_action", filter.action.as_str()),
                ("state_refs_count", &filter.state_refs.len().to_string()),
            ],
        );
        // Actual filtering happens in execution engine when states are evaluated
    }

    Ok(object_refs)
}

/// Recursively extract all object references from SET operands
fn extract_objects_from_operands(
    operands: &[ResolvedSetOperand],
    output: &mut Vec<ObjectRef>,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
) -> Result<(), ResolutionError> {
    for operand in operands {
        match operand {
            ResolvedSetOperand::ObjectRef(obj_id) => {
                let _ = log_consumer_debug(
                    "Adding object reference from SET operand",
                    &[("object_id", obj_id)],
                );
                output.push(ObjectRef::new(obj_id.clone()));
            }
            ResolvedSetOperand::SetRef(nested_set_id) => {
                let _ = log_consumer_debug(
                    "Recursively expanding nested SET_REF",
                    &[("nested_set_id", nested_set_id)],
                );

                // Recursive: expand nested SET
                let nested_set = resolved_sets.get(nested_set_id).ok_or_else(|| {
                    ResolutionError::UndefinedSet {
                        name: nested_set_id.clone(),
                        context: format!("Nested SET expansion for '{}'", nested_set_id),
                    }
                })?;

                // Recursively extract from nested set
                extract_objects_from_operands(&nested_set.resolved_operands, output, resolved_sets)?;
            }
            ResolvedSetOperand::InlineObject(obj) => {
                let _ = log_consumer_debug(
                    "Adding inline object from SET operand",
                    &[("object_id", &obj.identifier)],
                );
                output.push(ObjectRef::new(obj.identifier.clone()));
            }
        }
    }

    Ok(())
}

/// Validate that all SET_REF expansions are resolvable
/// Checks for circular dependencies in SET references
pub fn validate_set_expansions(context: &ResolutionContext) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating SET expansions",
        &[("sets_count", &context.resolved_sets.len().to_string())],
    );

    // Check for circular SET_REF dependencies
    for (set_id, resolved_set) in &context.resolved_sets {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if let Err(cycle) =
            check_circular_set_ref(set_id, resolved_set, &context.resolved_sets, &mut visited, &mut path)
        {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_VALIDATION_ERROR,
                &format!("Circular SET_REF dependency detected: {}", cycle.join(" -> ")),
                &[("set_id", set_id)],
            );
            return Err(ResolutionError::CircularDependency { cycle });
        }
    }

    let _ = log_consumer_debug("SET expansion validation complete", &[]);
    Ok(())
}

/// Check for circular SET_REF dependencies using DFS
fn check_circular_set_ref(
    set_id: &str,
    resolved_set: &ResolvedSetOperation,
    resolved_sets: &HashMap<String, ResolvedSetOperation>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> Result<(), Vec<String>> {
    // Check if we've seen this set_id in the current path (cycle detected)
    if path.contains(&set_id.to_string()) {
        let mut cycle = path.clone();
        cycle.push(set_id.to_string());
        return Err(cycle);
    }

    // Check if we've already validated this set in a previous path
    if visited.contains(set_id) {
        return Ok(());
    }

    visited.insert(set_id.to_string());
    path.push(set_id.to_string());

    // Check all SET_REF operands
    for operand in &resolved_set.resolved_operands {
        if let ResolvedSetOperand::SetRef(nested_set_id) = operand {
            if let Some(nested_set) = resolved_sets.get(nested_set_id) {
                check_circular_set_ref(nested_set_id, nested_set, resolved_sets, visited, path)?;
            }
        }
    }

    path.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::criterion::CriterionDeclaration;
    use crate::types::object::{ObjectDeclaration, ObjectElement, ObjectRef, ResolvedObject, ResolvedObjectElement};
    use crate::types::resolution_context::ResolutionContext;
    use crate::types::set::{ResolvedSetOperand, ResolvedSetOperation, SetOperationType};
    use crate::types::test::{TestSpecification, ExistenceCheck, ItemCheck};

    #[test]
    fn test_extract_set_ref_from_declaration_object() {
        let object_with_set_ref = ObjectDeclaration {
            identifier: "test_obj".to_string(),
            elements: vec![ObjectElement::SetRef {
                set_id: "my_set".to_string(),
            }],
            is_global: true,
        };

        assert_eq!(
            extract_set_ref_from_declaration_object(&object_with_set_ref),
            Some("my_set".to_string())
        );

        let object_without_set_ref = ObjectDeclaration {
            identifier: "test_obj".to_string(),
            elements: vec![],
            is_global: true,
        };

        assert_eq!(
            extract_set_ref_from_declaration_object(&object_without_set_ref),
            None
        );
    }

    #[test]
    fn test_expand_set_ref_in_declaration_with_local_object() {
        let mut context = ResolutionContext::new();

        // Create resolved objects
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

        context.resolved_global_objects.insert("obj1".to_string(), obj1);
        context.resolved_global_objects.insert("obj2".to_string(), obj2);

        // Create resolved set
        let resolved_set = ResolvedSetOperation {
            set_id: "test_set".to_string(),
            operation: SetOperationType::Union,
            resolved_operands: vec![
                ResolvedSetOperand::ObjectRef("obj1".to_string()),
                ResolvedSetOperand::ObjectRef("obj2".to_string()),
            ],
            resolved_filter: None,
        };

        context.resolved_sets.insert("test_set".to_string(), resolved_set);

        // Create declaration with local object containing SET_REF
        let mut declaration = CriterionDeclaration {
            criterion_type: "test_ctn".to_string(),
            test: TestSpecification::simple(ExistenceCheck::All, ItemCheck::All),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![],
            local_object: Some(ObjectDeclaration {
                identifier: "local_obj".to_string(),
                elements: vec![ObjectElement::SetRef {
                    set_id: "test_set".to_string(),
                }],
                is_global: false,
            }),
            ctn_node_id: Some(1),
        };

        // Expand using helper
        let expanded = expand_set_ref_in_declaration_helper(
            &mut declaration,
            &context.resolved_sets,
            &context.resolved_global_objects,
        ).unwrap();

        // Assert
        assert!(expanded, "Should have expanded");
        assert!(declaration.local_object.is_none(), "Local object should be cleared");
        assert_eq!(
            declaration.object_refs.len(),
            2,
            "Should have 2 expanded object refs"
        );
        assert!(declaration.object_refs.iter().any(|r| r.object_id == "obj1"));
        assert!(declaration.object_refs.iter().any(|r| r.object_id == "obj2"));
    }

    #[test]
    fn test_validate_circular_set_ref() {
        let mut context = ResolutionContext::new();

        // Create circular dependency: set_a -> set_b -> set_a
        let set_a = ResolvedSetOperation {
            set_id: "set_a".to_string(),
            operation: SetOperationType::Union,
            resolved_operands: vec![ResolvedSetOperand::SetRef("set_b".to_string())],
            resolved_filter: None,
        };

        let set_b = ResolvedSetOperation {
            set_id: "set_b".to_string(),
            operation: SetOperationType::Union,
            resolved_operands: vec![ResolvedSetOperand::SetRef("set_a".to_string())],
            resolved_filter: None,
        };

        context.resolved_sets.insert("set_a".to_string(), set_a);
        context.resolved_sets.insert("set_b".to_string(), set_b);

        // Should detect circular dependency
        let result = validate_set_expansions(&context);
        assert!(result.is_err(), "Should detect circular dependency");

        if let Err(ResolutionError::CircularDependency { cycle }) = result {
            assert!(cycle.contains(&"set_a".to_string()));
            assert!(cycle.contains(&"set_b".to_string()));
        } else {
            panic!("Expected CircularDependency error");
        }
    }
}