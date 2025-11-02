// parser/variable/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::resolution::ResolutionError;
use crate::types::common::DataType;
use crate::types::common::Value;
use crate::types::resolution_context::ResolutionContext;
use crate::types::variable::VariableDeclaration;

/// Extract variables from AST JSON with DAG-compatible computed variable support
pub fn extract_variables_from_json(
    ast_json: &serde_json::Value,
) -> Result<Vec<VariableDeclaration>, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting variable extraction from AST JSON",
        &[("ast_is_object", &ast_json.is_object().to_string())],
    );

    let definition = ast_json.get("definition").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "No 'definition' key found in AST JSON",
            &[(
                "available_keys",
                &ast_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            )],
        );
        ResolutionError::InvalidInput {
            message: "No definition found in AST".to_string(),
        }
    })?;

    // Handle missing variables gracefully - empty definitions are valid
    let variables_array = match definition.get("variables").and_then(|vars| vars.as_array()) {
        Some(array) => array,
        None => {
            let _ = log_consumer_debug(
                "No variables section found in definition - using empty variable collection",
                &[(
                    "definition_keys",
                    &definition
                        .as_object()
                        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                        .unwrap_or_else(|| "none".to_string()),
                )],
            );
            return Ok(Vec::new()); // Return empty vector - valid for definitions with no variables
        }
    };

    let _ = log_consumer_debug(
        "Found variables array in AST",
        &[("variable_count", &variables_array.len().to_string())],
    );

    let mut variables = Vec::new();

    for (index, var_json) in variables_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing variable",
            &[
                ("index", &index.to_string()),
                (
                    "variable_keys",
                    &var_json
                        .as_object()
                        .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                        .unwrap_or_else(|| "none".to_string()),
                ),
            ],
        );

        // Parse variable name (required)
        let name = var_json
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!("Variable at index {} missing name", index),
                    &[("index", &index.to_string())],
                );
                ResolutionError::InvalidInput {
                    message: "Variable missing name".to_string(),
                }
            })?;

        // Validate variable name is not empty (structural validation)
        if name.trim().is_empty() {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!("Variable at index {} has empty name", index),
                &[("index", &index.to_string())],
            );
            return Err(ResolutionError::InvalidInput {
                message: "Variable name cannot be empty".to_string(),
            });
        }

        // Parse data type (required)
        let data_type_str = var_json
            .get("data_type")
            .and_then(|dt| dt.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    &format!("Variable '{}' missing data_type", name),
                    &[("variable", name), ("index", &index.to_string())],
                );
                ResolutionError::InvalidInput {
                    message: "Variable missing data_type".to_string(),
                }
            })?;

        let data_type = DataType::from_str(data_type_str).ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid data type '{}' for variable '{}'",
                    data_type_str, name
                ),
                &[
                    ("variable", name),
                    ("data_type", data_type_str),
                    ("index", &index.to_string()),
                ],
            );
            ResolutionError::InvalidInput {
                message: format!("Invalid data type: {}", data_type_str),
            }
        })?;

        // Parse initial value - SUPPORTS COMPUTED VARIABLES
        let initial_value = if let Some(init_val) = var_json.get("initial_value") {
            if init_val.is_null() {
                // This is a computed variable (no initial value) - VALID for DAG resolution
                let _ = log_consumer_debug(
                    "Variable is computed (no initial value) - will be populated by RUN operations",
                    &[("variable", name)],
                );
                None
            } else {
                // Parse the initial value
                let _ = log_consumer_debug(
                    "Variable has initial value",
                    &[
                        ("variable", name),
                        (
                            "value_type",
                            match init_val {
                                serde_json::Value::Object(_) => "object",
                                serde_json::Value::String(_) => "string",
                                serde_json::Value::Number(_) => "number",
                                serde_json::Value::Bool(_) => "bool",
                                serde_json::Value::Array(_) => "array",
                                serde_json::Value::Null => "null",
                            },
                        ),
                    ],
                );

                match parse_value_from_json(init_val) {
                    Ok(value) => Some(value),
                    Err(e) => {
                        let _ = log_consumer_error(
                            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                            &format!(
                                "Failed to parse initial value for variable '{}': {:?}",
                                name, e
                            ),
                            &[("variable", name)],
                        );
                        return Err(e);
                    }
                }
            }
        } else {
            // Missing initial_value field means computed variable - VALID for DAG resolution
            let _ = log_consumer_debug(
                "Variable has no initial_value field - computed variable for RUN operations",
                &[("variable", name)],
            );
            None
        };

        // Log variable categorization for debugging
        let variable_category = if initial_value.is_none() {
            "computed"
        } else if matches!(initial_value, Some(Value::Variable(_))) {
            "reference"
        } else {
            "literal"
        };

        let _ = log_consumer_debug(
            "Successfully parsed variable",
            &[
                ("variable", name),
                ("data_type", data_type_str),
                ("category", variable_category),
                ("has_initial_value", &initial_value.is_some().to_string()),
            ],
        );

        variables.push(VariableDeclaration {
            name: name.to_string(),
            data_type,
            initial_value,
        });
    }

    let _ = log_consumer_debug(
        "Variable extraction completed",
        &[("total_extracted", &variables.len().to_string())],
    );

    Ok(variables)
}

/// Extract and categorize variables with ResolutionContext integration
pub fn extract_and_categorize_variables_from_json(
    ast_json: &serde_json::Value,
    context: &mut ResolutionContext,
) -> Result<Vec<VariableDeclaration>, ResolutionError> {
    let variables = extract_variables_from_json(ast_json)?;

    let _ = log_consumer_debug(
        "Categorizing variables for DAG resolution",
        &[("total_variables", &variables.len().to_string())],
    );

    let mut computed_count = 0;
    let mut literal_count = 0;
    let mut reference_count = 0;

    // Categorize variables and populate ResolutionContext tracking
    for variable in &variables {
        if variable.is_computed() {
            context.add_computed_variable(variable.name.clone());
            computed_count += 1;

            let _ = log_consumer_debug(
                "Categorized computed variable",
                &[("variable", &variable.name)],
            );
        } else if variable.has_literal_initial_value() {
            context.add_literal_variable(variable.name.clone());
            literal_count += 1;

            let _ = log_consumer_debug(
                "Categorized literal variable",
                &[("variable", &variable.name)],
            );
        } else if let Some(ref_name) = variable.get_initialization_dependency() {
            context.add_reference_variable(variable.name.clone(), ref_name.to_string());
            reference_count += 1;

            // Add deferred validation for variable reference (don't validate now)
            context.defer_variable_reference_validation(
                variable.name.clone(),
                ref_name.to_string(),
                format!(
                    "Variable initialization: {} = VAR {}",
                    variable.name, ref_name
                ),
            );

            let _ = log_consumer_debug(
                "Categorized reference variable with deferred validation",
                &[("variable", &variable.name), ("references", ref_name)],
            );
        }
    }

    let _ = log_consumer_debug(
        "Variable categorization completed",
        &[
            ("computed_variables", &computed_count.to_string()),
            ("literal_variables", &literal_count.to_string()),
            ("reference_variables", &reference_count.to_string()),
        ],
    );

    Ok(variables)
}

/// Collect variable dependencies for DAG construction (no validation)
pub fn collect_variable_dependencies(variables: &[VariableDeclaration]) -> Vec<VariableDependency> {
    let mut dependencies = Vec::new();

    for variable in variables {
        if let Some(ref_name) = variable.get_initialization_dependency() {
            dependencies.push(VariableDependency {
                source_variable: variable.name.clone(),
                target_variable: ref_name.to_string(),
                dependency_type: VariableDependencyType::Initialization,
            });
        }
    }

    dependencies
}

/// Analyze variable distribution for debugging and optimization
pub fn analyze_variable_distribution(variables: &[VariableDeclaration]) -> VariableDistribution {
    let mut computed = Vec::new();
    let mut literal = Vec::new();
    let mut reference = Vec::new();

    for variable in variables {
        if variable.is_computed() {
            computed.push(variable.name.clone());
        } else if variable.has_literal_initial_value() {
            literal.push(variable.name.clone());
        } else if let Some(ref_name) = variable.get_initialization_dependency() {
            reference.push((variable.name.clone(), ref_name.to_string()));
        }
    }

    VariableDistribution {
        computed,
        literal,
        reference,
    }
}

/// Parse Value enum from JSON with comprehensive error handling
fn parse_value_from_json(value_json: &serde_json::Value) -> Result<Value, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing value from JSON",
        &[(
            "json_type",
            match value_json {
                serde_json::Value::Object(_) => "object",
                serde_json::Value::String(_) => "string",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Bool(_) => "bool",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Null => "null",
            },
        )],
    );

    // Parse based on JSON structure from AST
    if let Some(string_val) = value_json.get("String").and_then(|s| s.as_str()) {
        let _ = log_consumer_debug("Parsed as String value", &[("value", string_val)]);
        Ok(Value::String(string_val.to_string()))
    } else if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
        let _ = log_consumer_debug(
            "Parsed as Integer value",
            &[("value", &int_val.to_string())],
        );
        Ok(Value::Integer(int_val))
    } else if let Some(float_val) = value_json.get("Float").and_then(|f| f.as_f64()) {
        let _ = log_consumer_debug(
            "Parsed as Float value",
            &[("value", &float_val.to_string())],
        );
        Ok(Value::Float(float_val))
    } else if let Some(bool_val) = value_json.get("Boolean").and_then(|b| b.as_bool()) {
        let _ = log_consumer_debug(
            "Parsed as Boolean value",
            &[("value", &bool_val.to_string())],
        );
        Ok(Value::Boolean(bool_val))
    } else if let Some(var_name) = value_json.get("Variable").and_then(|v| v.as_str()) {
        let _ = log_consumer_debug("Parsed as Variable reference", &[("variable", var_name)]);
        Ok(Value::Variable(var_name.to_string()))
    } else {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "Unknown value type in JSON - expected String, Integer, Float, Boolean, or Variable",
            &[(
                "available_keys",
                &value_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            )],
        );
        Err(ResolutionError::InvalidInput {
            message: "Unknown value type in JSON".to_string(),
        })
    }
}

/// Check if a value contains variable references (utility function)
pub fn has_variable_references(value: &Value) -> bool {
    matches!(value, Value::Variable(_))
}

/// Validate variable names for conflicts (structural validation only)
pub fn validate_variable_name_uniqueness(
    variables: &[VariableDeclaration],
) -> Result<(), ResolutionError> {
    use std::collections::HashSet;

    let mut seen_names = HashSet::new();

    for variable in variables {
        if !seen_names.insert(&variable.name) {
            return Err(ResolutionError::InvalidInput {
                message: format!("Duplicate variable name: '{}'", variable.name),
            });
        }
    }

    Ok(())
}

/// Helper structures for variable analysis
#[derive(Debug, Clone)]
pub struct VariableDependency {
    pub source_variable: String,
    pub target_variable: String,
    pub dependency_type: VariableDependencyType,
}

#[derive(Debug, Clone)]
pub enum VariableDependencyType {
    Initialization, // VAR x = VAR y
}

#[derive(Debug, Clone)]
pub struct VariableDistribution {
    pub computed: Vec<String>,            // Variables with no initial value
    pub literal: Vec<String>,             // Variables with literal initial values
    pub reference: Vec<(String, String)>, // (source_var, target_var) pairs
}

impl VariableDistribution {
    /// Get total variable count
    pub fn total_count(&self) -> usize {
        self.computed.len() + self.literal.len() + self.reference.len()
    }

    /// Check if any computed variables exist
    pub fn has_computed_variables(&self) -> bool {
        !self.computed.is_empty()
    }

    /// Check if any reference variables exist
    pub fn has_reference_variables(&self) -> bool {
        !self.reference.is_empty()
    }

    /// Get summary string for logging
    pub fn summary(&self) -> String {
        format!(
            "Variables: {} computed, {} literal, {} reference",
            self.computed.len(),
            self.literal.len(),
            self.reference.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_computed_variable_parsing() {
        let ast_json = json!({
            "definition": {
                "variables": [
                    {
                        "name": "computed_var",
                        "data_type": "String",
                        "initial_value": null
                    },
                    {
                        "name": "literal_var",
                        "data_type": "Int",
                        "initial_value": {"Integer": 42}
                    }
                ]
            }
        });

        let variables = extract_variables_from_json(&ast_json).unwrap();
        assert_eq!(variables.len(), 2);

        let computed = variables.iter().find(|v| v.name == "computed_var").unwrap();
        assert!(computed.is_computed());
        assert!(!computed.has_initial_value());

        let literal = variables.iter().find(|v| v.name == "literal_var").unwrap();
        assert!(!literal.is_computed());
        assert!(literal.has_initial_value());
    }

    #[test]
    fn test_variable_reference_parsing() {
        let ast_json = json!({
            "definition": {
                "variables": [
                    {
                        "name": "ref_var",
                        "data_type": "String",
                        "initial_value": {"Variable": "source_var"}
                    }
                ]
            }
        });

        let variables = extract_variables_from_json(&ast_json).unwrap();
        assert_eq!(variables.len(), 1);

        let ref_var = &variables[0];
        assert!(!ref_var.is_computed());
        assert!(ref_var.has_variable_reference_initialization());
        assert_eq!(ref_var.get_initialization_dependency(), Some("source_var"));
    }

    #[test]
    fn test_empty_variables_section() {
        let ast_json = json!({
            "definition": {}
        });

        let variables = extract_variables_from_json(&ast_json).unwrap();
        assert_eq!(variables.len(), 0);
    }
}
