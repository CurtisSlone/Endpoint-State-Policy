// parser/criterion/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::parser::object;
use crate::resolution::ResolutionError;
use crate::types::common::LogicalOp;
use crate::types::criterion::{CriterionDeclaration, CtnNodeId};
use crate::types::object::{ObjectDeclaration, ObjectRef};
use crate::types::resolution_context::ResolutionContext;
use crate::types::state::{StateDeclaration, StateRef};
use crate::types::test::{ExistenceCheck, ItemCheck, StateJoinOp, TestSpecification};
use crate::types::{CriteriaRoot, CriteriaTree};

/// Extract criteria from AST JSON, building tree structure instead of flattening
/// Returns CriteriaRoot with nested CriteriaTree structures
pub fn extract_criteria_from_json(
    ast_json: &serde_json::Value,
    context: &mut ResolutionContext,
) -> Result<CriteriaRoot, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting criteria extraction from AST JSON (tree structure)",
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
    let criteria_array = definition
        .get("criteria")
        .and_then(|crit| crit.as_array())
        .unwrap_or(&empty_vec);

    let _ = log_consumer_debug(
        "Found criteria array",
        &[("criteria_count", &criteria_array.len().to_string())],
    );

    // Track CTN node IDs globally
    let mut ctn_node_id_counter = 0usize;
    let mut trees = Vec::new();

    // Build tree for each top-level CRI block
    for (criteria_index, criteria_json) in criteria_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing top-level criteria block",
            &[("criteria_index", &criteria_index.to_string())],
        );

        match build_criteria_tree(criteria_json, &mut ctn_node_id_counter, 0, context) {
            Ok(tree) => {
                let ctn_count = tree.count_criteria();
                let max_depth = tree.max_depth();

                let _ = log_consumer_debug(
                    "Successfully built criteria tree",
                    &[
                        ("criteria_index", &criteria_index.to_string()),
                        ("ctn_count", &ctn_count.to_string()),
                        ("max_depth", &max_depth.to_string()),
                    ],
                );

                trees.push(tree);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to build criteria tree at index {}: {:?}",
                    criteria_index, e
                );
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[("criteria_index", &criteria_index.to_string())],
                );
                context.add_parsing_error(error_msg);
                return Err(e);
            }
        }
    }

    let root = CriteriaRoot {
        trees,
        root_logical_op: LogicalOp::And, // Default: top-level CRI blocks are ANDed
    };

    let total_criteria = root.total_criteria_count();

    let _ = log_consumer_debug(
        "Criteria extraction completed",
        &[
            ("total_trees", &root.trees.len().to_string()),
            ("total_criteria", &total_criteria.to_string()),
            ("total_ctn_nodes", &ctn_node_id_counter.to_string()),
            (
                "deferred_validations",
                &context.deferred_validations.len().to_string(),
            ),
        ],
    );

    Ok(root)
}

/// Build criteria tree from JSON (recursive)
/// EBNF: criteria ::= "CRI" space logical_operator space? negate_flag? statement_end criteria_content "CRI_END" statement_end
fn build_criteria_tree(
    criteria_json: &serde_json::Value,
    ctn_node_id_counter: &mut CtnNodeId,
    depth: usize,
    context: &mut ResolutionContext,
) -> Result<CriteriaTree, ResolutionError> {
    const MAX_NESTING_DEPTH: usize = 10;

    if depth > MAX_NESTING_DEPTH {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Criteria nesting depth {} exceeds maximum {}",
                depth, MAX_NESTING_DEPTH
            ),
            &[
                ("depth", &depth.to_string()),
                ("max_depth", &MAX_NESTING_DEPTH.to_string()),
            ],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Criteria nesting too deep: {} > {}",
                depth, MAX_NESTING_DEPTH
            ),
        });
    }

    let _ = log_consumer_debug("Building criteria tree", &[("depth", &depth.to_string())]);

    // Parse logical operator and negate flag
    let logical_op_str = criteria_json
        .get("logical_op")
        .and_then(|op| op.as_str())
        .unwrap_or("AND");

    let logical_op = LogicalOp::from_str(logical_op_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!("Invalid logical operator '{}' in criteria", logical_op_str),
            &[("logical_op", logical_op_str)],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid logical operator: {}", logical_op_str),
        }
    })?;

    let negate = criteria_json
        .get("negate")
        .and_then(|n| n.as_bool())
        .unwrap_or(false);

    let _ = log_consumer_debug(
        "Parsed criteria metadata",
        &[
            ("logical_op", logical_op_str),
            ("negate", &negate.to_string()),
            ("depth", &depth.to_string()),
        ],
    );

    // Process criteria content
    let content_array = criteria_json
        .get("content")
        .and_then(|content| content.as_array())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Criteria missing 'content' array",
                &[("depth", &depth.to_string())],
            );
            ResolutionError::InvalidInput {
                message: "Criteria missing content".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Processing criteria content",
        &[
            ("content_count", &content_array.len().to_string()),
            ("depth", &depth.to_string()),
        ],
    );

    let mut children = Vec::new();

    for (content_index, content_json) in content_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing criteria content item",
            &[
                ("content_index", &content_index.to_string()),
                ("depth", &depth.to_string()),
            ],
        );

        // Handle nested Criteria (recursive)
        if let Some(nested_criteria) = content_json.get("Criteria") {
            let _ = log_consumer_debug(
                "Processing nested Criteria block",
                &[("depth", &depth.to_string())],
            );

            match build_criteria_tree(nested_criteria, ctn_node_id_counter, depth + 1, context) {
                Ok(nested_tree) => {
                    let _ = log_consumer_debug(
                        "Successfully built nested criteria tree",
                        &[
                            ("nested_count", &nested_tree.count_criteria().to_string()),
                            ("depth", &depth.to_string()),
                        ],
                    );
                    children.push(nested_tree);
                }
                Err(e) => {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                        &format!(
                            "Failed to build nested criteria at content index {}: {:?}",
                            content_index, e
                        ),
                        &[
                            ("content_index", &content_index.to_string()),
                            ("depth", &depth.to_string()),
                        ],
                    );
                    return Err(e);
                }
            }
        }
        // Handle Criterion (CTN block)
        else if let Some(criterion_json) = content_json.get("Criterion") {
            let _ = log_consumer_debug(
                "Processing Criterion (CTN)",
                &[
                    ("ctn_node_id", &ctn_node_id_counter.to_string()),
                    ("depth", &depth.to_string()),
                ],
            );

            match parse_criterion_from_json(criterion_json, *ctn_node_id_counter, context) {
                Ok(criterion_declaration) => {
                    let _ = log_consumer_debug(
                        "Successfully parsed criterion",
                        &[
                            ("ctn_type", &criterion_declaration.criterion_type),
                            ("ctn_node_id", &ctn_node_id_counter.to_string()),
                            ("depth", &depth.to_string()),
                        ],
                    );

                    *ctn_node_id_counter += 1;

                    let tree_node = CriteriaTree::Criterion {
                        declaration: criterion_declaration,
                        node_id: *ctn_node_id_counter,
                    };

                    children.push(tree_node);
                }
                Err(e) => {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                        &format!(
                            "Failed to parse criterion at content index {}: {:?}",
                            content_index, e
                        ),
                        &[
                            ("content_index", &content_index.to_string()),
                            ("depth", &depth.to_string()),
                        ],
                    );
                    return Err(e);
                }
            }
        } else {
            let available_keys = content_json
                .as_object()
                .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_else(Vec::new);

            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Unknown criteria content type at index {}", content_index),
                &[
                    ("content_index", &content_index.to_string()),
                    ("available_keys", &available_keys.join(",")),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Unknown criteria content type at index {}", content_index),
            });
        }
    }

    // Handle edge case: single CTN without CRI block wrapping
    if children.len() == 1 && depth == 0 {
        if let CriteriaTree::Criterion { .. } = &children[0] {
            let _ = log_consumer_debug(
                "Single CTN at root level, returning directly",
                &[("depth", &depth.to_string())],
            );
            return Ok(children.into_iter().next().unwrap());
        }
    }

    let tree = CriteriaTree::Block {
        logical_op,
        negate,
        children,
    };

    let _ = log_consumer_debug(
        "Criteria tree building completed",
        &[
            ("depth", &depth.to_string()),
            ("logical_op", logical_op_str),
            ("negate", &negate.to_string()),
            ("children_count", &tree.count_criteria().to_string()),
        ],
    );

    Ok(tree)
}

/// Parse a single criterion (CTN block) from JSON with DAG-aware validation deferral
/// EBNF: criterion ::= "CTN" space criterion_type statement_end ctn_content "CTN_END" statement_end
fn parse_criterion_from_json(
    criterion_json: &serde_json::Value,
    ctn_node_id: CtnNodeId,
    context: &mut ResolutionContext,
) -> Result<CriterionDeclaration, ResolutionError> {
    let criterion_type = criterion_json
        .get("criterion_type")
        .and_then(|ct| ct.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Criterion missing 'criterion_type' field",
                &[("ctn_node_id", &ctn_node_id.to_string())],
            );
            ResolutionError::InvalidInput {
                message: "Criterion missing criterion_type".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Parsing criterion",
        &[
            ("criterion_type", criterion_type),
            ("ctn_node_id", &ctn_node_id.to_string()),
        ],
    );

    // Validate criterion type identifier (structural validation - OK to keep)
    if !is_valid_identifier(criterion_type) {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid criterion type identifier format: '{}'",
                criterion_type
            ),
            &[("criterion_type", criterion_type)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Invalid criterion type identifier format: '{}'",
                criterion_type
            ),
        });
    }

    // Parse test specification (required)
    let test = parse_test_specification(criterion_json, criterion_type)?;

    // Parse state references (optional) with deferred validation
    let state_refs = parse_state_refs(criterion_json, criterion_type, context)?;

    // Parse object references (optional) with deferred validation
    let object_refs = parse_object_refs(criterion_json, criterion_type, context)?;

    // Parse local states (optional) with variable reference tracking
    let local_states = parse_local_states(criterion_json, criterion_type, context)?;

    // Parse local object (optional, max 1) with variable reference tracking
    let local_object = parse_local_object(criterion_json, criterion_type, context)?;

    // Validate CTN has some content - structural validation only
    if state_refs.is_empty()
        && object_refs.is_empty()
        && local_states.is_empty()
        && local_object.is_none()
    {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "CTN '{}' has no content (state refs, object refs, or local elements required)",
                criterion_type
            ),
            &[("criterion_type", criterion_type)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("CTN '{}' must have at least one state reference, object reference, or local element", criterion_type) 
        });
    }

    // REMOVED: Premature test specification vs state count validation
    // This validation is deferred to DAG resolution phase when:
    // 1. All computed variables are resolved
    // 2. All RUN operations have been executed
    // 3. Final state count is known

    let _ = log_consumer_debug(
        "Criterion parsing completed - test/state count validation deferred to DAG resolution",
        &[
            ("criterion_type", criterion_type),
            ("ctn_node_id", &ctn_node_id.to_string()),
            ("state_refs", &state_refs.len().to_string()),
            ("object_refs", &object_refs.len().to_string()),
            ("local_states", &local_states.len().to_string()),
            ("has_local_object", &local_object.is_some().to_string()),
        ],
    );

    Ok(CriterionDeclaration {
        criterion_type: criterion_type.to_string(),
        test,
        state_refs,
        object_refs,
        local_states,
        local_object,
        ctn_node_id: Some(ctn_node_id),
    })
}

/// Parse test specification from criterion JSON
/// EBNF: test_specification ::= "TEST" space existence_check space item_check (space state_operator)? statement_end
fn parse_test_specification(
    criterion_json: &serde_json::Value,
    criterion_type: &str,
) -> Result<TestSpecification, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing test specification",
        &[("criterion_type", criterion_type)],
    );

    let test_json = criterion_json.get("test").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("CTN '{}' missing 'test' specification", criterion_type),
            &[("criterion_type", criterion_type)],
        );
        ResolutionError::InvalidInput {
            message: "CTN missing test specification".to_string(),
        }
    })?;

    let existence_check_str = test_json
        .get("existence_check")
        .and_then(|ec| ec.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Test specification missing 'existence_check' in CTN '{}'",
                    criterion_type
                ),
                &[("criterion_type", criterion_type)],
            );
            ResolutionError::InvalidInput {
                message: "Test specification missing existence_check".to_string(),
            }
        })?;

    let existence_check = ExistenceCheck::from_str(existence_check_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid existence check '{}' in CTN '{}'",
                existence_check_str, criterion_type
            ),
            &[
                ("criterion_type", criterion_type),
                ("existence_check", existence_check_str),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid existence check: {}", existence_check_str),
        }
    })?;

    let item_check_str = test_json
        .get("item_check")
        .and_then(|ic| ic.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Test specification missing 'item_check' in CTN '{}'",
                    criterion_type
                ),
                &[("criterion_type", criterion_type)],
            );
            ResolutionError::InvalidInput {
                message: "Test specification missing item_check".to_string(),
            }
        })?;

    let item_check = ItemCheck::from_str(item_check_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid item check '{}' in CTN '{}'",
                item_check_str, criterion_type
            ),
            &[
                ("criterion_type", criterion_type),
                ("item_check", item_check_str),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid item check: {}", item_check_str),
        }
    })?;

    // Parse optional state operator
    let state_operator = if let Some(state_op_json) = test_json.get("state_operator") {
        if !state_op_json.is_null() {
            let state_op_str = state_op_json.as_str().ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!("State operator is not a string in CTN '{}'", criterion_type),
                    &[("criterion_type", criterion_type)],
                );
                ResolutionError::InvalidInput {
                    message: "State operator must be a string".to_string(),
                }
            })?;

            let state_op = StateJoinOp::from_str(state_op_str).ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!(
                        "Invalid state operator '{}' in CTN '{}'",
                        state_op_str, criterion_type
                    ),
                    &[
                        ("criterion_type", criterion_type),
                        ("state_operator", state_op_str),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: format!("Invalid state operator: {}", state_op_str),
                }
            })?;

            Some(state_op)
        } else {
            None
        }
    } else {
        None
    };

    let _ = log_consumer_debug(
        "Successfully parsed test specification",
        &[
            ("criterion_type", criterion_type),
            ("existence_check", existence_check_str),
            ("item_check", item_check_str),
            ("has_state_operator", &state_operator.is_some().to_string()),
        ],
    );

    Ok(TestSpecification::new(
        existence_check,
        item_check,
        state_operator,
    ))
}

/// Parse state references from criterion JSON with deferred validation
fn parse_state_refs(
    criterion_json: &serde_json::Value,
    criterion_type: &str,
    context: &mut ResolutionContext,
) -> Result<Vec<StateRef>, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing state references",
        &[("criterion_type", criterion_type)],
    );

    let empty_vec = Vec::new();
    let state_refs_array = criterion_json
        .get("state_refs")
        .and_then(|refs| refs.as_array())
        .unwrap_or(&empty_vec); // No state refs is valid

    let _ = log_consumer_debug(
        "Found state references array",
        &[
            ("criterion_type", criterion_type),
            ("state_ref_count", &state_refs_array.len().to_string()),
        ],
    );

    let mut state_refs = Vec::new();

    for (ref_index, state_ref_json) in state_refs_array.iter().enumerate() {
        let state_id = state_ref_json
            .get("state_id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!(
                        "State reference {} missing 'state_id' in CTN '{}'",
                        ref_index, criterion_type
                    ),
                    &[
                        ("criterion_type", criterion_type),
                        ("ref_index", &ref_index.to_string()),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: "State reference missing state_id".to_string(),
                }
            })?;

        // Structural validation (identifier format) - OK to keep
        if !is_valid_identifier(state_id) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid state ID '{}' in reference {} of CTN '{}'",
                    state_id, ref_index, criterion_type
                ),
                &[("criterion_type", criterion_type), ("state_id", state_id)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid state ID format: '{}'", state_id),
            });
        }

        // Defer state reference validation to DAG phase
        context.defer_state_reference_validation(
            format!("CTN:{}", criterion_type),
            state_id.to_string(),
            format!(
                "CTN '{}' references global state '{}'",
                criterion_type, state_id
            ),
        );

        state_refs.push(StateRef::new(state_id));
    }

    let _ = log_consumer_debug(
        "State references parsing completed",
        &[
            ("criterion_type", criterion_type),
            ("total_state_refs", &state_refs.len().to_string()),
        ],
    );

    Ok(state_refs)
}

/// Parse object references from criterion JSON with deferred validation
fn parse_object_refs(
    criterion_json: &serde_json::Value,
    criterion_type: &str,
    context: &mut ResolutionContext,
) -> Result<Vec<ObjectRef>, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing object references",
        &[("criterion_type", criterion_type)],
    );

    let empty_vec = Vec::new();
    let object_refs_array = criterion_json
        .get("object_refs")
        .and_then(|refs| refs.as_array())
        .unwrap_or(&empty_vec); // No object refs is valid

    let _ = log_consumer_debug(
        "Found object references array",
        &[
            ("criterion_type", criterion_type),
            ("object_ref_count", &object_refs_array.len().to_string()),
        ],
    );

    let mut object_refs = Vec::new();

    for (ref_index, object_ref_json) in object_refs_array.iter().enumerate() {
        let object_id = object_ref_json
            .get("object_id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!(
                        "Object reference {} missing 'object_id' in CTN '{}'",
                        ref_index, criterion_type
                    ),
                    &[
                        ("criterion_type", criterion_type),
                        ("ref_index", &ref_index.to_string()),
                    ],
                );
                ResolutionError::InvalidInput {
                    message: "Object reference missing object_id".to_string(),
                }
            })?;

        // Structural validation (identifier format) - OK to keep
        if !is_valid_identifier(object_id) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid object ID '{}' in reference {} of CTN '{}'",
                    object_id, ref_index, criterion_type
                ),
                &[("criterion_type", criterion_type), ("object_id", object_id)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid object ID format: '{}'", object_id),
            });
        }

        // Defer object reference validation to DAG phase
        context.defer_object_reference_validation(
            format!("CTN:{}", criterion_type),
            object_id.to_string(),
            format!(
                "CTN '{}' references global object '{}'",
                criterion_type, object_id
            ),
        );

        object_refs.push(ObjectRef::new(object_id));
    }

    let _ = log_consumer_debug(
        "Object references parsing completed",
        &[
            ("criterion_type", criterion_type),
            ("total_object_refs", &object_refs.len().to_string()),
        ],
    );

    Ok(object_refs)
}

/// Parse a single state from JSON for local state parsing (temporary function until state module is updated)
fn parse_local_state_from_json(
    state_json: &serde_json::Value,
    _is_global: bool, // false for local states
    context: &mut ResolutionContext,
) -> Result<StateDeclaration, ResolutionError> {
    // This is a temporary implementation - in practice, this should call the state module
    // For now, we'll create a basic parser that follows the expected JSON structure

    let identifier = state_json
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "State missing id field".to_string(),
        })?;

    let _ = log_consumer_debug("Parsing local state", &[("state_id", identifier)]);

    // Parse fields array
    let empty_vec = Vec::new();
    let fields_array = state_json
        .get("fields")
        .and_then(|f| f.as_array())
        .unwrap_or(&empty_vec);

    let mut fields = Vec::new();
    for field_json in fields_array {
        if let Ok(field) = parse_state_field_from_json(field_json, identifier, context) {
            fields.push(field);
        }
    }

    // Parse record_checks array (if present)
    let record_checks_array = state_json
        .get("record_checks")
        .and_then(|rc| rc.as_array())
        .unwrap_or(&empty_vec);

    let mut record_checks = Vec::new();
    for record_json in record_checks_array {
        if let Ok(record_check) = parse_record_check_from_json(record_json, identifier, context) {
            record_checks.push(record_check);
        }
    }

    Ok(StateDeclaration {
        identifier: identifier.to_string(),
        fields,
        record_checks,
        is_global: false, // Local states are never global
    })
}

/// Parse a state field from JSON (helper function)
fn parse_state_field_from_json(
    field_json: &serde_json::Value,
    state_id: &str,
    context: &mut ResolutionContext,
) -> Result<crate::types::state::StateField, ResolutionError> {
    use crate::types::common::{DataType, Operation};
    use crate::types::state::{EntityCheck, StateField};

    let name = field_json
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "State field missing name".to_string(),
        })?;

    let data_type_str = field_json
        .get("data_type")
        .and_then(|dt| dt.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "State field missing data_type".to_string(),
        })?;

    let data_type =
        DataType::from_str(data_type_str).ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Invalid data type: {}", data_type_str),
        })?;

    let operation_str = field_json
        .get("operation")
        .and_then(|op| op.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "State field missing operation".to_string(),
        })?;

    let operation =
        Operation::from_str(operation_str).ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Invalid operation: {}", operation_str),
        })?;

    let value_json = field_json
        .get("value")
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "State field missing value".to_string(),
        })?;

    let value = parse_value_from_json_simple(value_json, state_id, name, context)?;

    let entity_check = field_json
        .get("entity_check")
        .and_then(|ec| ec.as_str())
        .and_then(EntityCheck::from_str);

    Ok(StateField {
        name: name.to_string(),
        data_type,
        operation,
        value,
        entity_check,
    })
}

/// Parse a record check from JSON (helper function)
fn parse_record_check_from_json(
    _record_json: &serde_json::Value,
    _state_id: &str,
    _context: &mut ResolutionContext,
) -> Result<crate::types::state::RecordCheck, ResolutionError> {
    // Simplified implementation for now - would need full record parsing
    // This is a placeholder until the state module provides proper record parsing
    Err(ResolutionError::InvalidInput {
        message: "Record checks not yet implemented in local parser".to_string(),
    })
}

/// Simple value parser (helper function)
fn parse_value_from_json_simple(
    value_json: &serde_json::Value,
    state_id: &str,
    field_name: &str,
    context: &mut ResolutionContext,
) -> Result<crate::types::common::Value, ResolutionError> {
    use crate::types::common::Value;

    if let Some(string_val) = value_json.get("String").and_then(|s| s.as_str()) {
        Ok(Value::String(string_val.to_string()))
    } else if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
        Ok(Value::Integer(int_val))
    } else if let Some(float_val) = value_json.get("Float").and_then(|f| f.as_f64()) {
        Ok(Value::Float(float_val))
    } else if let Some(bool_val) = value_json.get("Boolean").and_then(|b| b.as_bool()) {
        Ok(Value::Boolean(bool_val))
    } else if let Some(var_name) = value_json.get("Variable").and_then(|v| v.as_str()) {
        // Defer variable reference validation
        context.defer_variable_reference_validation(
            format!("State:{}:Field:{}", state_id, field_name),
            var_name.to_string(),
            format!(
                "State field '{}' in state '{}' references variable",
                field_name, state_id
            ),
        );
        Ok(Value::Variable(var_name.to_string()))
    } else {
        Err(ResolutionError::InvalidInput {
            message: "Unknown value type in state field".to_string(),
        })
    }
}

/// Parse local states from criterion JSON (CTN-level, non-referenceable) with variable tracking
fn parse_local_states(
    criterion_json: &serde_json::Value,
    criterion_type: &str,
    context: &mut ResolutionContext,
) -> Result<Vec<StateDeclaration>, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing local states",
        &[("criterion_type", criterion_type)],
    );

    let empty_vec = Vec::new();
    let local_states_array = criterion_json
        .get("local_states")
        .and_then(|states| states.as_array())
        .unwrap_or(&empty_vec); // No local states is valid

    let _ = log_consumer_debug(
        "Found local states array",
        &[
            ("criterion_type", criterion_type),
            ("local_state_count", &local_states_array.len().to_string()),
        ],
    );

    let mut local_states = Vec::new();

    for (state_index, state_json) in local_states_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing local state",
            &[
                ("criterion_type", criterion_type),
                ("state_index", &state_index.to_string()),
            ],
        );

        // Call state parsing function - need to check what function actually exists
        match parse_local_state_from_json(state_json, false, context) {
            Ok(state_declaration) => {
                let _ = log_consumer_debug(
                    "Successfully parsed local state",
                    &[
                        ("criterion_type", criterion_type),
                        ("state_id", &state_declaration.identifier),
                        ("state_index", &state_index.to_string()),
                    ],
                );

                // Track variable references from local state for DAG resolution
                if state_declaration.has_variable_references() {
                    for var_ref in state_declaration.get_variable_references() {
                        context.defer_variable_reference_validation(
                            format!(
                                "CTN:{}:LocalState:{}",
                                criterion_type, state_declaration.identifier
                            ),
                            var_ref,
                            format!(
                                "Local state '{}' in CTN '{}' references variable",
                                state_declaration.identifier, criterion_type
                            ),
                        );
                    }
                }

                local_states.push(state_declaration);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to parse local state {} in CTN '{}': {:?}",
                    state_index, criterion_type, e
                );
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[
                        ("criterion_type", criterion_type),
                        ("state_index", &state_index.to_string()),
                    ],
                );

                // For DAG resolution, we could collect this as a non-fatal error
                context.add_parsing_error(error_msg.clone());
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Local states parsing completed",
        &[
            ("criterion_type", criterion_type),
            ("total_local_states", &local_states.len().to_string()),
        ],
    );

    Ok(local_states)
}

/// Parse local object from criterion JSON (CTN-level, non-referenceable, max 1) with variable tracking
fn parse_local_object(
    criterion_json: &serde_json::Value,
    criterion_type: &str,
    context: &mut ResolutionContext,
) -> Result<Option<ObjectDeclaration>, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing local object",
        &[("criterion_type", criterion_type)],
    );

    if let Some(local_object_json) = criterion_json.get("local_object") {
        if !local_object_json.is_null() {
            let _ = log_consumer_debug(
                "Found local object in CTN",
                &[("criterion_type", criterion_type)],
            );

            // Create ObjectParsingContext for this local object
            let mut obj_parsing_context = object::ObjectParsingContext::new();

            // FIXED: Call the correct function with proper parameters
            match object::parse_object_from_json(local_object_json, false, &mut obj_parsing_context)
            {
                Ok(object_declaration) => {
                    let _ = log_consumer_debug(
                        "Successfully parsed local object",
                        &[
                            ("criterion_type", criterion_type),
                            ("object_id", &object_declaration.identifier),
                        ],
                    );

                    // Apply parsing results to resolution context
                    obj_parsing_context.apply_to_resolution_context(context);

                    // Track variable references from local object for DAG resolution
                    if object_declaration.has_variable_references() {
                        for var_ref in object_declaration.get_variable_references() {
                            context.defer_variable_reference_validation(
                                format!(
                                    "CTN:{}:LocalObject:{}",
                                    criterion_type, object_declaration.identifier
                                ),
                                var_ref,
                                format!(
                                    "Local object '{}' in CTN '{}' references variable",
                                    object_declaration.identifier, criterion_type
                                ),
                            );
                        }
                    }

                    // Track filter dependencies from local object for DAG resolution
                    if object_declaration.has_filters() {
                        for state_dep in object_declaration.get_filter_state_dependencies() {
                            context.add_deferred_validation(crate::types::resolution_context::DeferredValidation {
                                validation_type: crate::types::resolution_context::ValidationType::FilterStateReference,
                                source_symbol: format!("CTN:{}:LocalObject:{}:FILTER", criterion_type, object_declaration.identifier),
                                target_symbol: state_dep,
                                context: format!("Local object '{}' filter in CTN '{}' references state", 
                                               object_declaration.identifier, criterion_type),
                            });
                        }
                    }

                    return Ok(Some(object_declaration));
                }
                Err(e) => {
                    let error_msg = format!(
                        "Failed to parse local object in CTN '{}': {:?}",
                        criterion_type, e
                    );
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                        &error_msg,
                        &[("criterion_type", criterion_type)],
                    );

                    // For DAG resolution, collect as non-fatal error
                    context.add_parsing_error(error_msg);
                    return Err(e);
                }
            }
        }
    }

    let _ = log_consumer_debug(
        "No local object found in CTN",
        &[("criterion_type", criterion_type)],
    );

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

// =============================================================================
// DAG-aware dependency collection methods (no validation)
// =============================================================================

/// Check if a criterion has variable references in its local elements
pub fn has_variable_references_in_criterion(criterion: &CriterionDeclaration) -> bool {
    !criterion.get_variable_references().is_empty()
}

/// Get all variable references from a criterion (for dependency analysis)
pub fn get_variable_references_from_criterion(criterion: &CriterionDeclaration) -> Vec<String> {
    criterion.get_variable_references()
}

/// Get all global state references from a criterion (for dependency analysis)
pub fn get_global_state_references_from_criterion(criterion: &CriterionDeclaration) -> Vec<String> {
    criterion.get_global_state_refs()
}

/// Get all global object references from a criterion (for dependency analysis)
pub fn get_global_object_references_from_criterion(
    criterion: &CriterionDeclaration,
) -> Vec<String> {
    criterion.get_global_object_refs()
}

/// Get all filter dependencies from a criterion (for dependency analysis)
pub fn get_filter_dependencies_from_criterion(criterion: &CriterionDeclaration) -> Vec<String> {
    criterion.get_filter_dependencies()
}

/// Validate criterion structure only (no cross-reference validation)
pub fn validate_criterion_structure(
    criterion: &CriterionDeclaration,
) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating criterion structure only",
        &[("criterion_type", &criterion.criterion_type)],
    );

    // Use the built-in validation method (structural validation only)
    if let Err(validation_error) = criterion.validate() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Criterion structure validation failed for '{}': {}",
                criterion.criterion_type, validation_error
            ),
            &[("criterion_type", &criterion.criterion_type)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Criterion '{}' structure validation failed: {}",
                criterion.criterion_type, validation_error
            ),
        });
    }

    let _ = log_consumer_debug(
        "Criterion structure validation completed successfully",
        &[("criterion_type", &criterion.criterion_type)],
    );

    Ok(())
}

/// Collect all dependencies from a criterion for DAG construction
pub fn collect_criterion_dependencies(
    criterion: &CriterionDeclaration,
    context: &mut ResolutionContext,
) {
    let criterion_type = &criterion.criterion_type;

    // Collect global state references
    for state_ref in &criterion.state_refs {
        context.defer_state_reference_validation(
            format!("CTN:{}", criterion_type),
            state_ref.state_id.clone(),
            format!("CTN '{}' depends on global state", criterion_type),
        );
    }

    // Collect global object references
    for object_ref in &criterion.object_refs {
        context.defer_object_reference_validation(
            format!("CTN:{}", criterion_type),
            object_ref.object_id.clone(),
            format!("CTN '{}' depends on global object", criterion_type),
        );
    }

    // Collect variable references from local states
    for local_state in &criterion.local_states {
        if local_state.has_variable_references() {
            for var_ref in local_state.get_variable_references() {
                context.defer_variable_reference_validation(
                    format!(
                        "CTN:{}:LocalState:{}",
                        criterion_type, local_state.identifier
                    ),
                    var_ref,
                    format!(
                        "Local state in CTN '{}' depends on variable",
                        criterion_type
                    ),
                );
            }
        }
    }

    // Collect variable references from local object
    if let Some(local_object) = &criterion.local_object {
        if local_object.has_variable_references() {
            for var_ref in local_object.get_variable_references() {
                context.defer_variable_reference_validation(
                    format!(
                        "CTN:{}:LocalObject:{}",
                        criterion_type, local_object.identifier
                    ),
                    var_ref,
                    format!(
                        "Local object in CTN '{}' depends on variable",
                        criterion_type
                    ),
                );
            }
        }
    }

    // Collect filter dependencies
    for filter_state in criterion.get_filter_dependencies() {
        context.add_deferred_validation(crate::types::resolution_context::DeferredValidation {
            validation_type: crate::types::resolution_context::ValidationType::FilterStateReference,
            source_symbol: format!("CTN:{}:FILTER", criterion_type),
            target_symbol: filter_state,
            context: format!("CTN '{}' filter depends on state", criterion_type),
        });
    }
}
