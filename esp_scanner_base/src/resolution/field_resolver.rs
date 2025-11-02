use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::types::common::{ResolvedValue, Value};
use crate::types::error::FieldResolutionError;
use crate::types::variable::ResolvedVariable;
use std::collections::HashMap;

pub struct FieldResolver;

impl FieldResolver {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_value(
        &self,
        value: &Value,
        context: &str,
        resolved_variables: &HashMap<String, ResolvedVariable>,
    ) -> Result<ResolvedValue, FieldResolutionError> {
        let _ = log_consumer_debug(
            "Resolving field value with variable support",
            &[
                ("context", context),
                (
                    "value_type",
                    match value {
                        Value::String(_) => "string",
                        Value::Integer(_) => "integer",
                        Value::Float(_) => "float",
                        Value::Boolean(_) => "boolean",
                        Value::Variable(_) => "variable",
                    },
                ),
                ("available_variables", &resolved_variables.len().to_string()),
            ],
        );

        let result = match value {
            Value::String(s) => {
                let _ = log_consumer_debug(
                    "Resolved string value",
                    &[("context", context), ("length", &s.len().to_string())],
                );
                Ok(ResolvedValue::String(s.clone()))
            }
            Value::Integer(i) => {
                let _ = log_consumer_debug(
                    "Resolved integer value",
                    &[("context", context), ("value", &i.to_string())],
                );
                Ok(ResolvedValue::Integer(*i))
            }
            Value::Float(f) => {
                let _ = log_consumer_debug(
                    "Resolved float value",
                    &[("context", context), ("value", &f.to_string())],
                );
                Ok(ResolvedValue::Float(*f))
            }
            Value::Boolean(b) => {
                let _ = log_consumer_debug(
                    "Resolved boolean value",
                    &[("context", context), ("value", &b.to_string())],
                );
                Ok(ResolvedValue::Boolean(*b))
            }
            Value::Variable(var_name) => {
                let _ = log_consumer_debug(
                    "Resolving variable reference",
                    &[
                        ("variable_name", var_name),
                        ("context", context),
                        (
                            "available_variables",
                            &resolved_variables
                                .keys()
                                .map(|k| k.as_str())
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                    ],
                );

                if let Some(resolved_var) = resolved_variables.get(var_name) {
                    let _ = log_consumer_debug(
                        "Variable resolved successfully",
                        &[
                            ("variable_name", var_name),
                            ("resolved_type", &format!("{:?}", resolved_var.data_type)),
                            ("context", context),
                        ],
                    );
                    Ok(resolved_var.value.clone())
                } else {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_VALIDATION_ERROR,
                        &format!(
                            "Variable reference '{}' could not be resolved in {}",
                            var_name, context
                        ),
                        &[
                            ("variable_name", var_name),
                            ("context", context),
                            (
                                "available_variables",
                                &resolved_variables
                                    .keys()
                                    .map(|k| k.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                        ],
                    );

                    // Provide helpful error message based on what variables are available
                    let error_context = if resolved_variables.is_empty() {
                        format!("{} - no variables have been resolved yet", context)
                    } else {
                        format!(
                            "{} - variable '{}' not found (available: {})",
                            context,
                            var_name,
                            resolved_variables
                                .keys()
                                .map(|k| k.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };

                    Err(FieldResolutionError::VariableReferenceNotAllowed {
                        variable_name: var_name.clone(),
                        context: error_context,
                    })
                }
            }
        };

        if result.is_err() {
            let _ = log_consumer_debug("Field resolution failed", &[("context", context)]);
        } else {
            let _ = log_consumer_debug(
                "Field resolution completed successfully",
                &[("context", context)],
            );
        }

        result
    }

    /// Convenience method for resolving values without variable support (legacy compatibility)
    pub fn resolve_value_direct(
        &self,
        value: &Value,
        context: &str,
    ) -> Result<ResolvedValue, FieldResolutionError> {
        let empty_variables = HashMap::new();
        self.resolve_value(value, context, &empty_variables)
    }
}

impl Default for FieldResolver {
    fn default() -> Self {
        Self::new()
    }
}
