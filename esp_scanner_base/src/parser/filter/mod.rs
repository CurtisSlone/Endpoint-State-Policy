// parser/filter/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::resolution::ResolutionError;
use crate::types::filter::{FilterAction, FilterSpec};
use crate::types::resolution_context::{DeferredValidation, ValidationType};

/// Enhanced context for filter parsing with DAG support
#[derive(Debug, Clone)]
pub struct FilterParsingContext {
    /// Parsing errors that are non-fatal
    pub parsing_errors: Vec<String>,
    /// Deferred validations to be processed later
    pub deferred_validations: Vec<DeferredValidation>,
    /// State references found during parsing (source, target, context)
    pub state_references: Vec<(String, String, String)>,
}

impl FilterParsingContext {
    pub fn new() -> Self {
        Self {
            parsing_errors: Vec::new(),
            deferred_validations: Vec::new(),
            state_references: Vec::new(),
        }
    }

    /// Add a parsing error (non-fatal)
    pub fn add_parsing_error(&mut self, error: String) {
        self.parsing_errors.push(error);
    }

    /// Add deferred filter state reference validation
    pub fn defer_filter_state_reference(
        &mut self,
        source: String,
        target: String,
        context: String,
    ) {
        self.state_references
            .push((source.clone(), target.clone(), context.clone()));
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::FilterStateReference,
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
    }
}

/// Result of filter parsing with collected context
#[derive(Debug)]
pub struct FilterParsingResult {
    pub filter: FilterSpec,
    pub context: FilterParsingContext,
}

impl FilterParsingResult {
    pub fn new(filter: FilterSpec, context: FilterParsingContext) -> Self {
        Self { filter, context }
    }
}

/// Extract filter specification from JSON with DAG-aware context collection
/// Used by object parser and set parser when they encounter Filter elements
pub fn extract_filter_from_json_with_context(
    filter_json: &serde_json::Value,
    source_id: &str,
    context: &str,
) -> Result<FilterParsingResult, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting filter extraction from JSON with DAG context",
        &[
            ("source_id", source_id),
            ("context", context),
            ("filter_is_object", &filter_json.is_object().to_string()),
        ],
    );

    let mut parsing_context = FilterParsingContext::new();

    if filter_json.is_null() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("Filter specification is null in context '{}'", context),
            &[("context", context), ("source_id", source_id)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Filter specification cannot be null in {}", context),
        });
    }

    // Parse filter action (required field)
    let action = parse_filter_action(filter_json, context, &mut parsing_context)?;

    // Parse state references (required field) - DEFER validation of existence
    let state_refs = parse_state_references_with_deferred_validation(
        filter_json,
        source_id,
        context,
        &mut parsing_context,
    )?;

    // Validate filter has at least one state reference (structural validation)
    if state_refs.is_empty() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Filter in '{}' has no state references (at least one required)",
                context
            ),
            &[("context", context), ("source_id", source_id)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Filter in '{}' must reference at least one state", context),
        });
    }

    let filter = FilterSpec::new(action, state_refs);

    let _ = log_consumer_debug(
        "Successfully parsed filter specification with DAG context",
        &[
            ("source_id", source_id),
            ("context", context),
            ("action", action.as_str()),
            ("state_ref_count", &filter.state_refs.len().to_string()),
            (
                "deferred_validations",
                &parsing_context.deferred_validations.len().to_string(),
            ),
        ],
    );

    Ok(FilterParsingResult::new(filter, parsing_context))
}

/// Backward compatibility wrapper - extract filter without context collection
pub fn extract_filter_from_json(
    filter_json: &serde_json::Value,
    context: &str,
) -> Result<FilterSpec, ResolutionError> {
    let result = extract_filter_from_json_with_context(filter_json, "unknown", context)?;
    // Note: This loses the parsing context information, but maintains backward compatibility
    Ok(result.filter)
}

/// Parse filter action from JSON (include/exclude)
fn parse_filter_action(
    filter_json: &serde_json::Value,
    context: &str,
    parsing_context: &mut FilterParsingContext,
) -> Result<FilterAction, ResolutionError> {
    let _ = log_consumer_debug("Parsing filter action", &[("context", context)]);

    let action_value = filter_json.get("action").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("Filter missing 'action' field in context '{}'", context),
            &[
                ("context", context),
                (
                    "available_keys",
                    &filter_json
                        .as_object()
                        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                        .unwrap_or_else(|| "none".to_string()),
                ),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Filter missing action in {}", context),
        }
    })?;

    let action_str = action_value.as_str().ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("Filter action is not a string in context '{}'", context),
            &[
                ("context", context),
                (
                    "action_type",
                    match action_value {
                        serde_json::Value::Object(_) => "object",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Bool(_) => "bool",
                        serde_json::Value::Null => "null",
                        serde_json::Value::String(_) => "string",
                    },
                ),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Filter action must be a string in {}", context),
        }
    })?;

    let action = FilterAction::from_str(action_str).unwrap_or_else(|| {
        let error_msg = format!(
            "Invalid filter action '{}' in context '{}', defaulting to 'include'",
            action_str, context
        );
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &error_msg,
            &[
                ("context", context),
                ("action", action_str),
                ("valid_actions", "include,exclude"),
                ("default_action", "include"),
            ],
        );

        // For DAG resolution, continue parsing with default and collect error
        parsing_context.add_parsing_error(error_msg);
        FilterAction::Include // Default fallback
    });

    let _ = log_consumer_debug(
        "Successfully parsed filter action",
        &[("context", context), ("action", action.as_str())],
    );

    Ok(action)
}

/// Parse state references array from JSON with deferred validation
fn parse_state_references_with_deferred_validation(
    filter_json: &serde_json::Value,
    source_id: &str,
    context: &str,
    parsing_context: &mut FilterParsingContext,
) -> Result<Vec<String>, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing filter state references with deferred validation",
        &[("source_id", source_id), ("context", context)],
    );

    let state_refs_value = filter_json.get("state_refs").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("Filter missing 'state_refs' field in context '{}'", context),
            &[
                ("source_id", source_id),
                ("context", context),
                (
                    "available_keys",
                    &filter_json
                        .as_object()
                        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                        .unwrap_or_else(|| "none".to_string()),
                ),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Filter missing state_refs in {}", context),
        }
    })?;

    let state_refs_array = state_refs_value.as_array().ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("Filter state_refs is not an array in context '{}'", context),
            &[
                ("source_id", source_id),
                ("context", context),
                (
                    "state_refs_type",
                    match state_refs_value {
                        serde_json::Value::Object(_) => "object",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Bool(_) => "bool",
                        serde_json::Value::Null => "null",
                    },
                ),
            ],
        );
        ResolutionError::InvalidInput {
            message: format!("Filter state_refs must be an array in {}", context),
        }
    })?;

    let _ = log_consumer_debug(
        "Found state references array",
        &[
            ("source_id", source_id),
            ("context", context),
            ("state_ref_count", &state_refs_array.len().to_string()),
        ],
    );

    let mut state_refs = Vec::new();

    for (index, state_ref_value) in state_refs_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing state reference",
            &[
                ("source_id", source_id),
                ("context", context),
                ("index", &index.to_string()),
            ],
        );

        // Handle both direct string references and STATE_REF objects
        let state_id = if let Some(state_str) = state_ref_value.as_str() {
            // Direct string reference
            let _ = log_consumer_debug(
                "Found direct string state reference",
                &[
                    ("source_id", source_id),
                    ("context", context),
                    ("state_id", state_str),
                    ("index", &index.to_string()),
                ],
            );
            state_str.to_string()
        } else if let Some(state_ref_obj) = state_ref_value.as_object() {
            // STATE_REF object format
            let state_id_str = state_ref_obj
                .get("state_id")
                .and_then(|id| id.as_str())
                .ok_or_else(|| {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_FORMAT_ERROR,
                        &format!(
                            "State reference object at index {} missing 'state_id' in context '{}'",
                            index, context
                        ),
                        &[
                            ("source_id", source_id),
                            ("context", context),
                            ("index", &index.to_string()),
                            (
                                "available_keys",
                                &state_ref_obj.keys().cloned().collect::<Vec<_>>().join(","),
                            ),
                        ],
                    );
                    ResolutionError::InvalidInput {
                        message: format!("State reference object missing state_id in {}", context),
                    }
                })?;

            let _ = log_consumer_debug(
                "Found STATE_REF object",
                &[
                    ("source_id", source_id),
                    ("context", context),
                    ("state_id", state_id_str),
                    ("index", &index.to_string()),
                ],
            );

            state_id_str.to_string()
        } else {
            let error_msg = format!(
                "Invalid state reference format at index {} in context '{}'",
                index, context
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &error_msg,
                &[
                    ("source_id", source_id),
                    ("context", context),
                    ("index", &index.to_string()),
                    (
                        "reference_type",
                        match state_ref_value {
                            serde_json::Value::Object(_) => "object",
                            serde_json::Value::Array(_) => "array",
                            serde_json::Value::String(_) => "string",
                            serde_json::Value::Number(_) => "number",
                            serde_json::Value::Bool(_) => "bool",
                            serde_json::Value::Null => "null",
                        },
                    ),
                ],
            );

            // For DAG resolution, continue parsing and collect error
            parsing_context.add_parsing_error(error_msg);
            continue; // Skip this invalid reference
        };

        // Validate state ID is not empty (structural validation)
        if state_id.trim().is_empty() {
            let error_msg = format!("Empty state ID at index {} in context '{}'", index, context);
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &error_msg,
                &[
                    ("source_id", source_id),
                    ("context", context),
                    ("index", &index.to_string()),
                ],
            );

            // For DAG resolution, continue parsing and collect error
            parsing_context.add_parsing_error(error_msg);
            continue; // Skip empty state ID
        }

        // Validate state ID format (basic identifier validation - structural)
        if !is_valid_identifier(&state_id) {
            let error_msg = format!(
                "Invalid state ID format '{}' at index {} in context '{}'",
                state_id, index, context
            );
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &error_msg,
                &[
                    ("source_id", source_id),
                    ("context", context),
                    ("state_id", &state_id),
                    ("index", &index.to_string()),
                ],
            );

            // For DAG resolution, continue parsing and collect error
            parsing_context.add_parsing_error(error_msg);
            continue; // Skip invalid state ID
        }

        // DEFER state existence validation - don't check if state exists now
        let _ = log_consumer_debug(
            "Deferring state reference validation",
            &[
                ("source_id", source_id),
                ("context", context),
                ("state_id", &state_id),
                ("index", &index.to_string()),
            ],
        );

        parsing_context.defer_filter_state_reference(
            source_id.to_string(),
            state_id.clone(),
            format!("filter state reference in {}", context),
        );

        let _ = log_consumer_debug(
            "Successfully parsed state reference",
            &[
                ("source_id", source_id),
                ("context", context),
                ("state_id", &state_id),
                ("index", &index.to_string()),
            ],
        );

        state_refs.push(state_id);
    }

    // Remove duplicates while preserving order
    let mut unique_state_refs = Vec::new();
    for state_ref in state_refs {
        if !unique_state_refs.contains(&state_ref) {
            unique_state_refs.push(state_ref);
        }
    }

    if unique_state_refs.len() != state_refs_array.len() {
        let _ = log_consumer_debug(
            "Removed duplicate and invalid state references",
            &[
                ("source_id", source_id),
                ("context", context),
                ("original_count", &state_refs_array.len().to_string()),
                ("unique_count", &unique_state_refs.len().to_string()),
            ],
        );
    }

    let _ = log_consumer_debug(
        "State references parsing completed with deferred validation",
        &[
            ("source_id", source_id),
            ("context", context),
            ("total_state_refs", &unique_state_refs.len().to_string()),
            (
                "deferred_validations",
                &parsing_context.state_references.len().to_string(),
            ),
        ],
    );

    Ok(unique_state_refs)
}

/// Validate identifier format (basic validation matching ICS EBNF identifier rules)
/// This is STRUCTURAL validation, not existence validation
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

/// Parse filter from object element context with DAG support (used by object parser)
pub fn parse_filter_from_object_element_with_context(
    filter_json: &serde_json::Value,
    object_id: &str,
) -> Result<FilterParsingResult, ResolutionError> {
    let context = format!("object '{}'", object_id);
    extract_filter_from_json_with_context(filter_json, object_id, &context)
}

/// Parse filter from set context with DAG support (used by set parser)
pub fn parse_filter_from_set_context_with_context(
    filter_json: &serde_json::Value,
    set_id: &str,
) -> Result<FilterParsingResult, ResolutionError> {
    let context = format!("set '{}'", set_id);
    extract_filter_from_json_with_context(filter_json, set_id, &context)
}

/// Backward compatibility: Parse filter from object element context (used by object parser)
pub fn parse_filter_from_object_element(
    filter_json: &serde_json::Value,
    object_id: &str,
) -> Result<FilterSpec, ResolutionError> {
    let result = parse_filter_from_object_element_with_context(filter_json, object_id)?;
    Ok(result.filter)
}

/// Backward compatibility: Parse filter from set context (used by set parser)
pub fn parse_filter_from_set_context(
    filter_json: &serde_json::Value,
    set_id: &str,
) -> Result<FilterSpec, ResolutionError> {
    let result = parse_filter_from_set_context_with_context(filter_json, set_id)?;
    Ok(result.filter)
}

/// Check if a filter specification has variable references (for dependency analysis)
pub fn has_variable_references_in_filter(_filter: &FilterSpec) -> bool {
    // Filters only reference states, not variables directly
    // Variable dependencies come through the referenced states
    false
}

/// Get all state dependencies from a filter (for dependency analysis)
pub fn get_state_dependencies_from_filter(filter: &FilterSpec) -> Vec<String> {
    filter.get_state_dependencies()
}

/// Apply filter parsing results to ResolutionContext
pub fn apply_filter_parsing_to_context(
    parsing_results: Vec<FilterParsingResult>,
    resolution_context: &mut crate::types::resolution_context::ResolutionContext,
) {
    for result in parsing_results {
        result
            .context
            .apply_to_resolution_context(resolution_context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_filter_with_include_action_dag_context() {
        let filter_json = json!({
            "action": "include",
            "state_refs": ["state1", "state2"]
        });

        let result =
            extract_filter_from_json_with_context(&filter_json, "test_source", "test_context");
        assert!(result.is_ok());

        let filter_result = result.unwrap();
        assert_eq!(filter_result.filter.action, FilterAction::Include);
        assert_eq!(filter_result.filter.state_refs.len(), 2);
        assert!(filter_result
            .filter
            .state_refs
            .contains(&"state1".to_string()));
        assert!(filter_result
            .filter
            .state_refs
            .contains(&"state2".to_string()));

        // Check that deferred validations were collected
        assert_eq!(filter_result.context.deferred_validations.len(), 2);
        assert_eq!(filter_result.context.state_references.len(), 2);
    }

    #[test]
    fn test_extract_filter_with_exclude_action() {
        let filter_json = json!({
            "action": "exclude",
            "state_refs": ["state3"]
        });

        let result = extract_filter_from_json(&filter_json, "test_context");
        assert!(result.is_ok());

        let filter = result.unwrap();
        assert_eq!(filter.action, FilterAction::Exclude);
        assert_eq!(filter.state_refs.len(), 1);
        assert_eq!(filter.state_refs[0], "state3");
    }

    #[test]
    fn test_extract_filter_with_state_ref_objects() {
        let filter_json = json!({
            "action": "include",
            "state_refs": [
                {"state_id": "state1"},
                {"state_id": "state2"}
            ]
        });

        let result =
            extract_filter_from_json_with_context(&filter_json, "test_source", "test_context");
        assert!(result.is_ok());

        let filter_result = result.unwrap();
        assert_eq!(filter_result.filter.state_refs.len(), 2);
        assert_eq!(filter_result.context.deferred_validations.len(), 2);
    }

    #[test]
    fn test_extract_filter_missing_action_with_fallback() {
        let filter_json = json!({
            "state_refs": ["state1"]
        });

        let result = extract_filter_from_json(&filter_json, "test_context");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_filter_empty_state_refs() {
        let filter_json = json!({
            "action": "include",
            "state_refs": []
        });

        let result = extract_filter_from_json(&filter_json, "test_context");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_filter_invalid_action_with_fallback() {
        let filter_json = json!({
            "action": "invalid_action",
            "state_refs": ["state1"]
        });

        let result =
            extract_filter_from_json_with_context(&filter_json, "test_source", "test_context");
        assert!(result.is_ok());

        let filter_result = result.unwrap();
        // Should fallback to Include and collect parsing error
        assert_eq!(filter_result.filter.action, FilterAction::Include);
        assert_eq!(filter_result.context.parsing_errors.len(), 1);
        assert_eq!(filter_result.context.deferred_validations.len(), 1);
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
    fn test_parse_filter_from_object_element_with_context() {
        let filter_json = json!({
            "action": "include",
            "state_refs": ["state1"]
        });

        let result = parse_filter_from_object_element_with_context(&filter_json, "test_object");
        assert!(result.is_ok());

        let filter_result = result.unwrap();
        assert_eq!(filter_result.context.deferred_validations.len(), 1);
        assert_eq!(filter_result.context.state_references.len(), 1);
    }

    #[test]
    fn test_parse_filter_with_invalid_state_refs_continues_parsing() {
        let filter_json = json!({
            "action": "include",
            "state_refs": ["valid_state", "", "123invalid", "another_valid"]
        });

        let result =
            extract_filter_from_json_with_context(&filter_json, "test_source", "test_context");
        assert!(result.is_ok());

        let filter_result = result.unwrap();
        // Should only include valid state references
        assert_eq!(filter_result.filter.state_refs.len(), 2);
        assert!(filter_result
            .filter
            .state_refs
            .contains(&"valid_state".to_string()));
        assert!(filter_result
            .filter
            .state_refs
            .contains(&"another_valid".to_string()));

        // Should collect parsing errors for invalid references
        assert!(filter_result.context.parsing_errors.len() > 0);
        // Should only have deferred validations for valid references
        assert_eq!(filter_result.context.deferred_validations.len(), 2);
    }
}
