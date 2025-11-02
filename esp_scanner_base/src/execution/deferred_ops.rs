//! # Deferred Operation Execution
//!
//! Handles scan-time operations that require collected data.

use crate::execution::engine::ExecutionError;
use crate::strategies::{CollectedData, CollectionError, CtnStrategyRegistry};
use crate::types::common::{DataType, ResolvedValue};
use crate::types::execution_context::{ExecutableObject, ExecutionContext};
use crate::types::resolution_context::DeferredOperation;
use crate::types::RuntimeOperationType;
use regex::Regex;
use std::collections::HashMap;

/// Execute all deferred operations in sequence
pub fn execute_all_deferred_operations(
    context: &mut ExecutionContext,
    registry: &CtnStrategyRegistry,
    collected_data: &HashMap<String, CollectedData>,
) -> Result<(), ExecutionError> {
    let operations = context.deferred_operations.clone();

    for operation in operations.iter() {
        execute_single_operation(operation, context, registry, collected_data)?;
    }

    Ok(())
}

/// Execute a single deferred operation
fn execute_single_operation(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
    registry: &CtnStrategyRegistry,
    collected_data: &HashMap<String, CollectedData>,
) -> Result<(), ExecutionError> {
    match operation.operation_type {
        RuntimeOperationType::Extract => execute_extract(operation, context, collected_data),
        RuntimeOperationType::Unique => execute_unique(operation, context),
        RuntimeOperationType::Merge => execute_merge(operation, context),
        RuntimeOperationType::End => execute_end(operation, context),
        RuntimeOperationType::Split => execute_split(operation, context),
        RuntimeOperationType::Substring => execute_substring(operation, context),
        RuntimeOperationType::RegexCapture => execute_regex_capture(operation, context),
        RuntimeOperationType::Count => execute_count(operation, context), 
        _ => Err(ExecutionError::DeferredOperationFailed {
            operation: format!("{:?}", operation.operation_type),
            reason: "Not a scan-time operation".to_string(),
        }),
    }
}

fn execute_extract(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
    collected_data: &HashMap<String, CollectedData>,
) -> Result<(), ExecutionError> {
    let object_id = operation.source_object_id.as_ref().ok_or_else(|| {
        ExecutionError::DeferredOperationFailed {
            operation: "EXTRACT".to_string(),
            reason: "Missing source_object_id".to_string(),
        }
    })?;

    let data =
        collected_data
            .get(object_id)
            .ok_or_else(|| ExecutionError::DataCollectionFailed {
                object_id: object_id.clone(),
                reason: "Object not in collected data".to_string(),
            })?;

    let field_name = extract_field_name_from_parameters(&operation.parameters)?;

    let field_value = data.get_field(&field_name).cloned().ok_or_else(|| {
        ExecutionError::DeferredOperationFailed {
            operation: "EXTRACT".to_string(),
            reason: format!("Field '{}' not found in collected data", field_name),
        }
    })?;

    update_target_variable(context, &operation.target_variable, field_value)?;
    Ok(())
}

fn execute_unique(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    let source_var_name = get_source_variable_name(operation)?;

    let source_value = context
        .global_variables
        .get(&source_var_name)
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "UNIQUE".to_string(),
            reason: format!("Source variable '{}' not found", source_var_name),
        })?;

    let collection = match &source_value.value {
        ResolvedValue::Collection(items) => items.clone(),
        _ => {
            return Err(ExecutionError::DeferredOperationFailed {
                operation: "UNIQUE".to_string(),
                reason: format!("Variable '{}' is not a collection", source_var_name),
            })
        }
    };

    let unique_items = deduplicate_collection(collection);
    update_target_variable(
        context,
        &operation.target_variable,
        ResolvedValue::Collection(unique_items),
    )?;
    Ok(())
}

fn execute_merge(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    let source_var_names = get_all_source_variable_names(operation)?;

    if source_var_names.is_empty() {
        return Err(ExecutionError::DeferredOperationFailed {
            operation: "MERGE".to_string(),
            reason: "No source variables specified".to_string(),
        });
    }

    let mut all_items = Vec::new();

    for var_name in source_var_names {
        let var = context.global_variables.get(&var_name).ok_or_else(|| {
            ExecutionError::DeferredOperationFailed {
                operation: "MERGE".to_string(),
                reason: format!("Source variable '{}' not found", var_name),
            }
        })?;

        match &var.value {
            ResolvedValue::Collection(items) => all_items.extend(items.clone()),
            _ => {
                return Err(ExecutionError::DeferredOperationFailed {
                    operation: "MERGE".to_string(),
                    reason: format!("Variable '{}' is not a collection", var_name),
                })
            }
        }
    }

    update_target_variable(
        context,
        &operation.target_variable,
        ResolvedValue::Collection(all_items),
    )?;
    Ok(())
}

fn execute_end(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    let source_var_name = get_source_variable_name(operation)?;

    let source_value = context
        .global_variables
        .get(&source_var_name)
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "END".to_string(),
            reason: format!("Source variable '{}' not found", source_var_name),
        })?;

    update_target_variable(
        context,
        &operation.target_variable,
        source_value.value.clone(),
    )?;
    Ok(())
}

/// Execute SPLIT operation - split string by delimiter into collection
fn execute_split(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    // 1. Get source variable name (the string to split)
    let source_var_name = get_source_variable_name(operation)?;
    
    // 2. Get delimiter from parameters
    let delimiter = extract_delimiter_from_parameters(&operation.parameters)?;
    
    // 3. Get source variable value
    let source_value = context
        .global_variables
        .get(&source_var_name)
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "SPLIT".to_string(),
            reason: format!("Source variable '{}' not found", source_var_name),
        })?;

    // 4. Extract string value
    let source_string = match &source_value.value {
        ResolvedValue::String(s) => s.clone(),
        _ => {
            return Err(ExecutionError::DeferredOperationFailed {
                operation: "SPLIT".to_string(),
                reason: format!(
                    "Variable '{}' is not a string (type: {:?})",
                    source_var_name,
                    source_value.data_type
                ),
            })
        }
    };

    // 5. Perform split operation
    let parts: Vec<ResolvedValue> = source_string
        .split(&delimiter)
        .map(|s| ResolvedValue::String(s.to_string()))
        .collect();

    // 6. Update target variable as Collection
    update_target_variable(
        context,
        &operation.target_variable,
        ResolvedValue::Collection(parts),
    )?;
    
    Ok(())
}

/// Execute SUBSTRING operation - extract substring by position and length
fn execute_substring(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    // 1. Get source variable name (the string to extract from)
    let source_var_name = get_source_variable_name(operation)?;
    
    // 2. Get start and length from parameters
    let (start, length) = extract_substring_parameters(&operation.parameters)?;
    
    // 3. Get source variable value
    let source_value = context
        .global_variables
        .get(&source_var_name)
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "SUBSTRING".to_string(),
            reason: format!("Source variable '{}' not found", source_var_name),
        })?;

    // 4. Extract string value
    let source_string = match &source_value.value {
        ResolvedValue::String(s) => s.clone(),
        _ => {
            return Err(ExecutionError::DeferredOperationFailed {
                operation: "SUBSTRING".to_string(),
                reason: format!(
                    "Variable '{}' is not a string (type: {:?})",
                    source_var_name,
                    source_value.data_type
                ),
            })
        }
    };

    // 5. Validate and perform substring extraction with bounds checking
    // Handle negative start position (not allowed)
    if start < 0 {
        return Err(ExecutionError::DeferredOperationFailed {
            operation: "SUBSTRING".to_string(),
            reason: format!("Start position cannot be negative: {}", start),
        });
    }
    
    // Handle negative length (not allowed)
    if length < 0 {
        return Err(ExecutionError::DeferredOperationFailed {
            operation: "SUBSTRING".to_string(),
            reason: format!("Length cannot be negative: {}", length),
        });
    }

    let start_usize = start as usize;
    let length_usize = length as usize;
    
    // Bounds checking: if start is beyond string length, return empty string
    let substring = if start_usize >= source_string.len() {
        String::new()
    } else {
        // Calculate end position with bounds checking
        let end_usize = start_usize.saturating_add(length_usize).min(source_string.len());
        source_string[start_usize..end_usize].to_string()
    };

    // 6. Update target variable
    update_target_variable(
        context,
        &operation.target_variable,
        ResolvedValue::String(substring),
    )?;
    
    Ok(())
}


/// Execute REGEX_CAPTURE operation - extract matched groups from regex pattern
fn execute_regex_capture(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    // 1. Get source variable name (the string to match against)
    let source_var_name = get_source_variable_name(operation)?;
    
    // 2. Get regex pattern from parameters
    let pattern = extract_regex_pattern(&operation.parameters)?;
    
    // 3. Compile regex pattern
    let regex = Regex::new(&pattern).map_err(|e| ExecutionError::DeferredOperationFailed {
        operation: "REGEX_CAPTURE".to_string(),
        reason: format!("Invalid regex pattern '{}': {}", pattern, e),
    })?;
    
    // 4. Get source variable value
    let source_value = context
        .global_variables
        .get(&source_var_name)
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "REGEX_CAPTURE".to_string(),
            reason: format!("Source variable '{}' not found", source_var_name),
        })?;

    // 5. Extract string value
    let source_string = match &source_value.value {
        ResolvedValue::String(s) => s.clone(),
        _ => {
            return Err(ExecutionError::DeferredOperationFailed {
                operation: "REGEX_CAPTURE".to_string(),
                reason: format!(
                    "Variable '{}' is not a string (type: {:?})",
                    source_var_name,
                    source_value.data_type
                ),
            })
        }
    };

    // 6. Perform regex capture
    let result = if let Some(captures) = regex.captures(&source_string) {
        // Strategy: Return first capture group if it exists, otherwise full match
        // Capture group 0 is always the full match
        // Capture group 1+ are the parenthesized groups
        if captures.len() > 1 {
            // Has capture groups - return first capture group (index 1)
            captures.get(1)
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            // No capture groups - return full match (index 0)
            captures.get(0)
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string()
        }
    } else {
        // No match - return empty string
        String::new()
    };

    // 7. Update target variable
    update_target_variable(
        context,
        &operation.target_variable,
        ResolvedValue::String(result),
    )?;
    
    Ok(())
}

/// Execute COUNT operation - count items in a collection
fn execute_count(
    operation: &DeferredOperation,
    context: &mut ExecutionContext,
) -> Result<(), ExecutionError> {
    // 1. Get source variable name (the collection to count)
    let source_var_name = get_source_variable_name(operation)?;
    
    // 2. Get source variable value
    let source_value = context
        .global_variables
        .get(&source_var_name)
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "COUNT".to_string(),
            reason: format!("Source variable '{}' not found", source_var_name),
        })?;

    // 3. Count items based on type
    let count = match &source_value.value {
        ResolvedValue::Collection(items) => items.len() as i64,
        // For non-collection types, count as 1 if value exists
        ResolvedValue::String(s) if !s.is_empty() => 1,
        ResolvedValue::String(_) => 0, // Empty string counts as 0
        ResolvedValue::Integer(_) | ResolvedValue::Float(_) | ResolvedValue::Boolean(_) => 1,
        ResolvedValue::Binary(b) if !b.is_empty() => 1,
        ResolvedValue::Binary(_) => 0,
        _ => {
            return Err(ExecutionError::DeferredOperationFailed {
                operation: "COUNT".to_string(),
                reason: format!(
                    "Variable '{}' cannot be counted (unsupported type: {:?})",
                    source_var_name,
                    source_value.data_type
                ),
            })
        }
    };

    // 4. Update target variable with count as integer
    update_target_variable(
        context,
        &operation.target_variable,
        ResolvedValue::Integer(count),
    )?;
    
    Ok(())
}

/// Helper: Extract regex pattern from parameters
fn extract_regex_pattern(
    params: &[crate::types::runtime_operation::RunParameter],
) -> Result<String, ExecutionError> {
    params
        .iter()
        .find_map(|param| {
            if let crate::types::runtime_operation::RunParameter::Pattern(p) = param {
                Some(p.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "REGEX_CAPTURE".to_string(),
            reason: "Missing pattern parameter".to_string(),
        })
}

/// Helper: Extract start and length parameters for SUBSTRING
fn extract_substring_parameters(
    params: &[crate::types::runtime_operation::RunParameter],
) -> Result<(i64, i64), ExecutionError> {
    let mut start = None;
    let mut length = None;
    
    for param in params {
        match param {
            crate::types::runtime_operation::RunParameter::StartPosition(s) => {
                start = Some(*s);
            }
            crate::types::runtime_operation::RunParameter::Length(l) => {
                length = Some(*l);
            }
            _ => {} // Ignore other parameter types
        }
    }
    
    let start_val = start.ok_or_else(|| ExecutionError::DeferredOperationFailed {
        operation: "SUBSTRING".to_string(),
        reason: "Missing 'start' parameter".to_string(),
    })?;
    
    let length_val = length.ok_or_else(|| ExecutionError::DeferredOperationFailed {
        operation: "SUBSTRING".to_string(),
        reason: "Missing 'length' parameter".to_string(),
    })?;
    
    Ok((start_val, length_val))
}



/// Helper: Extract delimiter from parameters
fn extract_delimiter_from_parameters(
    params: &[crate::types::runtime_operation::RunParameter],
) -> Result<String, ExecutionError> {
    params
        .iter()
        .find_map(|param| {
            if let crate::types::runtime_operation::RunParameter::Delimiter(d) = param {
                Some(d.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "SPLIT".to_string(),
            reason: "Missing delimiter parameter".to_string(),
        })
}

// Helper functions

fn extract_field_name_from_parameters(
    params: &[crate::types::runtime_operation::RunParameter],
) -> Result<String, ExecutionError> {
    params
        .iter()
        .find_map(|param| {
            if let crate::types::runtime_operation::RunParameter::ObjectExtraction {
                field, ..
            } = param
            {
                Some(field.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: "EXTRACT".to_string(),
            reason: "Missing field name in parameters".to_string(),
        })
}

fn get_source_variable_name(operation: &DeferredOperation) -> Result<String, ExecutionError> {
    operation
        .parameters
        .iter()
        .find_map(|param| {
            if let crate::types::runtime_operation::RunParameter::Variable(name) = param {
                Some(name.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| ExecutionError::DeferredOperationFailed {
            operation: format!("{:?}", operation.operation_type),
            reason: "No source variable found in parameters".to_string(),
        })
}

fn get_all_source_variable_names(
    operation: &DeferredOperation,
) -> Result<Vec<String>, ExecutionError> {
    let names: Vec<String> = operation
        .parameters
        .iter()
        .filter_map(|param| {
            if let crate::types::runtime_operation::RunParameter::Variable(name) = param {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    if names.is_empty() {
        Err(ExecutionError::DeferredOperationFailed {
            operation: format!("{:?}", operation.operation_type),
            reason: "No source variables found in parameters".to_string(),
        })
    } else {
        Ok(names)
    }
}

fn update_target_variable(
    context: &mut ExecutionContext,
    var_name: &str,
    value: ResolvedValue,
) -> Result<(), ExecutionError> {
    use crate::types::variable::ResolvedVariable;

    let data_type = infer_data_type_from_value(&value);
    let resolved_var = ResolvedVariable::new(var_name.to_string(), data_type, value);

    context
        .global_variables
        .insert(var_name.to_string(), resolved_var);
    Ok(())
}

fn infer_data_type_from_value(value: &ResolvedValue) -> DataType {
    match value {
        ResolvedValue::String(_) => DataType::String,
        ResolvedValue::Integer(_) => DataType::Int,
        ResolvedValue::Float(_) => DataType::Float,
        ResolvedValue::Boolean(_) => DataType::Boolean,
        ResolvedValue::Binary(_) => DataType::Binary,
        ResolvedValue::Version(_) => DataType::Version,
        ResolvedValue::EvrString(_) => DataType::EvrString,
        ResolvedValue::RecordData(_) => DataType::RecordData,
        ResolvedValue::Collection(items) => items
            .first()
            .map(|first| infer_data_type_from_value(first))
            .unwrap_or(DataType::String),
    }
}

fn deduplicate_collection(items: Vec<ResolvedValue>) -> Vec<ResolvedValue> {
    let mut seen = HashMap::new();
    let mut result = Vec::new();

    for (index, item) in items.into_iter().enumerate() {
        let key = format!("{:?}", item);
        if !seen.contains_key(&key) {
            seen.insert(key, index);
            result.push(item);
        }
    }

    result
}

#[cfg(test)]
mod split_tests {
    use super::*;
    use crate::types::common::{DataType, ResolvedValue};
    use crate::types::execution_context::ExecutionContext;
    use crate::types::resolution_context::{DeferredOperation, ResolutionContext};
    use crate::types::runtime_operation::RunParameter;
    use crate::types::variable::ResolvedVariable;
    use crate::types::RuntimeOperationType;
    use std::collections::HashMap;

    /// Helper to create test context with a string variable
    fn create_test_context_with_string(var_name: &str, value: &str) -> ExecutionContext {
        let mut res_context = ResolutionContext::new();
        
        // Add resolved variable
        res_context.resolved_variables.insert(
            var_name.to_string(),
            ResolvedVariable::new(
                var_name.to_string(),
                DataType::String,
                ResolvedValue::String(value.to_string()),
            ),
        );
        
        // Convert to ExecutionContext
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }
    
    /// Helper to create empty test context
    fn create_empty_test_context() -> ExecutionContext {
        let res_context = ResolutionContext::new();
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }

    #[test]
    fn test_split_comma_separated_values() {
        let mut context = create_test_context_with_string("csv_data", "apple,banana,cherry");
        
        let operation = DeferredOperation {
            target_variable: "fruits".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("csv_data".to_string()),
                RunParameter::Delimiter(",".to_string()),
            ],
        };
        
        execute_split(&operation, &mut context).expect("Split should succeed");
        
        let result = context.global_variables.get("fruits").expect("Target variable should exist");
        
        match &result.value {
            ResolvedValue::Collection(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ResolvedValue::String("apple".to_string()));
                assert_eq!(items[1], ResolvedValue::String("banana".to_string()));
                assert_eq!(items[2], ResolvedValue::String("cherry".to_string()));
            }
            _ => panic!("Expected Collection, got {:?}", result.value),
        }
    }

    #[test]
    fn test_split_pipe_delimiter() {
        let mut context = create_test_context_with_string("log_line", "ERROR|2024-11-01|System failure");
        
        let operation = DeferredOperation {
            target_variable: "log_parts".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("log_line".to_string()),
                RunParameter::Delimiter("|".to_string()),
            ],
        };
        
        execute_split(&operation, &mut context).expect("Split should succeed");
        
        let result = context.global_variables.get("log_parts").unwrap();
        
        match &result.value {
            ResolvedValue::Collection(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ResolvedValue::String("ERROR".to_string()));
                assert_eq!(items[1], ResolvedValue::String("2024-11-01".to_string()));
                assert_eq!(items[2], ResolvedValue::String("System failure".to_string()));
            }
            _ => panic!("Expected Collection"),
        }
    }

    #[test]
    fn test_split_empty_string() {
        let mut context = create_test_context_with_string("empty", "");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("empty".to_string()),
                RunParameter::Delimiter(",".to_string()),
            ],
        };
        
        execute_split(&operation, &mut context).expect("Split should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        
        match &result.value {
            ResolvedValue::Collection(items) => {
                // Empty string split produces one empty string
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], ResolvedValue::String("".to_string()));
            }
            _ => panic!("Expected Collection"),
        }
    }

    #[test]
    fn test_split_no_delimiter_found() {
        let mut context = create_test_context_with_string("text", "no commas here");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Delimiter(",".to_string()),
            ],
        };
        
        execute_split(&operation, &mut context).expect("Split should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        
        match &result.value {
            ResolvedValue::Collection(items) => {
                // No delimiter found = original string as single item
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], ResolvedValue::String("no commas here".to_string()));
            }
            _ => panic!("Expected Collection"),
        }
    }

    #[test]
    fn test_split_missing_source_variable() {
        let mut context = create_empty_test_context();
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("nonexistent".to_string()),
                RunParameter::Delimiter(",".to_string()),
            ],
        };
        
        let result = execute_split(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SPLIT");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_split_missing_delimiter() {
        let mut context = create_test_context_with_string("data", "test");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("data".to_string()),
                // Missing delimiter parameter
            ],
        };
        
        let result = execute_split(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SPLIT");
            assert!(reason.contains("delimiter"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_split_wrong_type() {
        let mut res_context = ResolutionContext::new();
        
        // Create integer variable instead of string
        res_context.resolved_variables.insert(
            "num".to_string(),
            ResolvedVariable::new(
                "num".to_string(),
                DataType::Int,
                ResolvedValue::Integer(42),
            ),
        );
        
        let mut context = ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create context");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("num".to_string()),
                RunParameter::Delimiter(",".to_string()),
            ],
        };
        
        let result = execute_split(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SPLIT");
            assert!(reason.contains("not a string"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_split_multichar_delimiter() {
        let mut context = create_test_context_with_string("data", "part1::part2::part3");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Split,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("data".to_string()),
                RunParameter::Delimiter("::".to_string()),
            ],
        };
        
        execute_split(&operation, &mut context).expect("Split should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        
        match &result.value {
            ResolvedValue::Collection(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ResolvedValue::String("part1".to_string()));
                assert_eq!(items[1], ResolvedValue::String("part2".to_string()));
                assert_eq!(items[2], ResolvedValue::String("part3".to_string()));
            }
            _ => panic!("Expected Collection"),
        }
    }
}

#[cfg(test)]
mod substring_tests {
    use super::*;
    use crate::types::common::{DataType, ResolvedValue};
    use crate::types::execution_context::ExecutionContext;
    use crate::types::resolution_context::{DeferredOperation, ResolutionContext};
    use crate::types::runtime_operation::RunParameter;
    use crate::types::variable::ResolvedVariable;
    use crate::types::RuntimeOperationType;

    /// Helper to create test context with a string variable
    fn create_test_context_with_string(var_name: &str, value: &str) -> ExecutionContext {
        let mut res_context = ResolutionContext::new();
        
        res_context.resolved_variables.insert(
            var_name.to_string(),
            ResolvedVariable::new(
                var_name.to_string(),
                DataType::String,
                ResolvedValue::String(value.to_string()),
            ),
        );
        
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }
    
    /// Helper to create empty test context
    fn create_empty_test_context() -> ExecutionContext {
        let res_context = ResolutionContext::new();
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }

    #[test]
    fn test_substring_basic() {
        let mut context = create_test_context_with_string("text", "Hello World");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(5),
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("result").expect("Target variable should exist");
        assert_eq!(result.value, ResolvedValue::String("Hello".to_string()));
    }

    #[test]
    fn test_substring_middle() {
        let mut context = create_test_context_with_string("text", "Hello World");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(6),
                RunParameter::Length(5),
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        assert_eq!(result.value, ResolvedValue::String("World".to_string()));
    }

    #[test]
    fn test_substring_entire_string() {
        let mut context = create_test_context_with_string("text", "Test");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(4),
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        assert_eq!(result.value, ResolvedValue::String("Test".to_string()));
    }

    #[test]
    fn test_substring_length_exceeds_string() {
        let mut context = create_test_context_with_string("text", "Short");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(100), // Exceeds string length
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        // Should return entire string when length exceeds
        assert_eq!(result.value, ResolvedValue::String("Short".to_string()));
    }

    #[test]
    fn test_substring_start_beyond_string() {
        let mut context = create_test_context_with_string("text", "Test");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(100), // Beyond string
                RunParameter::Length(5),
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        // Should return empty string when start is beyond string length
        assert_eq!(result.value, ResolvedValue::String("".to_string()));
    }

    #[test]
    fn test_substring_zero_length() {
        let mut context = create_test_context_with_string("text", "Hello");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(0), // Zero length
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        // Should return empty string for zero length
        assert_eq!(result.value, ResolvedValue::String("".to_string()));
    }

    #[test]
    fn test_substring_negative_start() {
        let mut context = create_test_context_with_string("text", "Hello");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(-1), // Negative start
                RunParameter::Length(3),
            ],
        };
        
        let result = execute_substring(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SUBSTRING");
            assert!(reason.contains("cannot be negative"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_substring_negative_length() {
        let mut context = create_test_context_with_string("text", "Hello");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(-5), // Negative length
            ],
        };
        
        let result = execute_substring(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SUBSTRING");
            assert!(reason.contains("cannot be negative"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_substring_missing_source_variable() {
        let mut context = create_empty_test_context();
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("nonexistent".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(5),
            ],
        };
        
        let result = execute_substring(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SUBSTRING");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_substring_missing_start_parameter() {
        let mut context = create_test_context_with_string("text", "Hello");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Length(5), // Missing start
            ],
        };
        
        let result = execute_substring(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SUBSTRING");
            assert!(reason.contains("start"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_substring_missing_length_parameter() {
        let mut context = create_test_context_with_string("text", "Hello");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::StartPosition(0), // Missing length
            ],
        };
        
        let result = execute_substring(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SUBSTRING");
            assert!(reason.contains("length"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_substring_wrong_type() {
        let mut res_context = ResolutionContext::new();
        
        // Create integer variable instead of string
        res_context.resolved_variables.insert(
            "num".to_string(),
            ResolvedVariable::new(
                "num".to_string(),
                DataType::Int,
                ResolvedValue::Integer(12345),
            ),
        );
        
        let mut context = ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create context");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("num".to_string()),
                RunParameter::StartPosition(0),
                RunParameter::Length(3),
            ],
        };
        
        let result = execute_substring(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "SUBSTRING");
            assert!(reason.contains("not a string"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_substring_extract_version_number() {
        // Practical use case: extract version from string like "v3.14.159"
        let mut context = create_test_context_with_string("version", "v3.14.159");
        
        let operation = DeferredOperation {
            target_variable: "version_number".to_string(),
            operation_type: RuntimeOperationType::Substring,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("version".to_string()),
                RunParameter::StartPosition(1), // Skip the 'v'
                RunParameter::Length(8),
            ],
        };
        
        execute_substring(&operation, &mut context).expect("Substring should succeed");
        
        let result = context.global_variables.get("version_number").unwrap();
        assert_eq!(result.value, ResolvedValue::String("3.14.159".to_string()));
    }
}

#[cfg(test)]
mod regex_capture_tests {
    use super::*;
    use crate::types::common::{DataType, ResolvedValue};
    use crate::types::execution_context::ExecutionContext;
    use crate::types::resolution_context::{DeferredOperation, ResolutionContext};
    use crate::types::runtime_operation::RunParameter;
    use crate::types::variable::ResolvedVariable;
    use crate::types::RuntimeOperationType;

    /// Helper to create test context with a string variable
    fn create_test_context_with_string(var_name: &str, value: &str) -> ExecutionContext {
        let mut res_context = ResolutionContext::new();
        
        res_context.resolved_variables.insert(
            var_name.to_string(),
            ResolvedVariable::new(
                var_name.to_string(),
                DataType::String,
                ResolvedValue::String(value.to_string()),
            ),
        );
        
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }
    
    /// Helper to create empty test context
    fn create_empty_test_context() -> ExecutionContext {
        let res_context = ResolutionContext::new();
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }

    #[test]
    fn test_regex_capture_single_group() {
        let mut context = create_test_context_with_string("log", "Error: 404 not found");
        
        let operation = DeferredOperation {
            target_variable: "error_code".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("log".to_string()),
                RunParameter::Pattern(r"Error: (\d+)".to_string()),
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("error_code").expect("Target variable should exist");
        assert_eq!(result.value, ResolvedValue::String("404".to_string()));
    }

    #[test]
    fn test_regex_capture_multiple_groups_returns_first() {
        let mut context = create_test_context_with_string("text", "2024-11-01 15:30:45");
        
        let operation = DeferredOperation {
            target_variable: "year".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Pattern(r"(\d{4})-(\d{2})-(\d{2})".to_string()),
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("year").unwrap();
        // Should return first capture group (year)
        assert_eq!(result.value, ResolvedValue::String("2024".to_string()));
    }

    #[test]
    fn test_regex_capture_no_groups_returns_full_match() {
        let mut context = create_test_context_with_string("text", "test@example.com");
        
        let operation = DeferredOperation {
            target_variable: "email".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Pattern(r"\w+@\w+\.\w+".to_string()), // No capture groups
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("email").unwrap();
        // Should return full match when no capture groups
        assert_eq!(result.value, ResolvedValue::String("test@example.com".to_string()));
    }

    #[test]
    fn test_regex_capture_no_match() {
        let mut context = create_test_context_with_string("text", "no numbers here");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Pattern(r"\d+".to_string()),
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        // Should return empty string when no match
        assert_eq!(result.value, ResolvedValue::String("".to_string()));
    }

    #[test]
    fn test_regex_capture_ip_address() {
        let mut context = create_test_context_with_string(
            "log",
            "Connection from 192.168.1.100:8080"
        );
        
        let operation = DeferredOperation {
            target_variable: "ip".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("log".to_string()),
                RunParameter::Pattern(r"(\d+\.\d+\.\d+\.\d+)".to_string()),
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("ip").unwrap();
        assert_eq!(result.value, ResolvedValue::String("192.168.1.100".to_string()));
    }

    #[test]
    fn test_regex_capture_version_number() {
        let mut context = create_test_context_with_string(
            "package",
            "openssl-3.0.7-27.el9.x86_64"
        );
        
        let operation = DeferredOperation {
            target_variable: "version".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("package".to_string()),
                RunParameter::Pattern(r"-(\d+\.\d+\.\d+)-".to_string()),
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("version").unwrap();
        assert_eq!(result.value, ResolvedValue::String("3.0.7".to_string()));
    }

    #[test]
    fn test_regex_capture_empty_string() {
        let mut context = create_test_context_with_string("text", "");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Pattern(r"\d+".to_string()),
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("result").unwrap();
        assert_eq!(result.value, ResolvedValue::String("".to_string()));
    }

    #[test]
    fn test_regex_capture_invalid_pattern() {
        let mut context = create_test_context_with_string("text", "test");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Pattern(r"[invalid(regex".to_string()), // Invalid regex
            ],
        };
        
        let result = execute_regex_capture(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "REGEX_CAPTURE");
            assert!(reason.contains("Invalid regex pattern"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_regex_capture_missing_source_variable() {
        let mut context = create_empty_test_context();
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("nonexistent".to_string()),
                RunParameter::Pattern(r"\d+".to_string()),
            ],
        };
        
        let result = execute_regex_capture(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "REGEX_CAPTURE");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_regex_capture_missing_pattern() {
        let mut context = create_test_context_with_string("text", "test");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                // Missing pattern parameter
            ],
        };
        
        let result = execute_regex_capture(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "REGEX_CAPTURE");
            assert!(reason.contains("pattern"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_regex_capture_wrong_type() {
        let mut res_context = ResolutionContext::new();
        
        // Create integer variable instead of string
        res_context.resolved_variables.insert(
            "num".to_string(),
            ResolvedVariable::new(
                "num".to_string(),
                DataType::Int,
                ResolvedValue::Integer(12345),
            ),
        );
        
        let mut context = ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create context");
        
        let operation = DeferredOperation {
            target_variable: "result".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("num".to_string()),
                RunParameter::Pattern(r"\d+".to_string()),
            ],
        };
        
        let result = execute_regex_capture(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "REGEX_CAPTURE");
            assert!(reason.contains("not a string"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_regex_capture_case_insensitive() {
        let mut context = create_test_context_with_string("text", "ERROR: System failure");
        
        let operation = DeferredOperation {
            target_variable: "level".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
                RunParameter::Pattern(r"(?i)(error|warning|info)".to_string()), // Case insensitive
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("level").unwrap();
        assert_eq!(result.value, ResolvedValue::String("ERROR".to_string()));
    }

    #[test]
    fn test_regex_capture_greedy_vs_lazy() {
        let mut context = create_test_context_with_string("html", "<div>content</div>");
        
        // Greedy match
        let operation = DeferredOperation {
            target_variable: "tag".to_string(),
            operation_type: RuntimeOperationType::RegexCapture,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("html".to_string()),
                RunParameter::Pattern(r"<(.+)>".to_string()), // Greedy
            ],
        };
        
        execute_regex_capture(&operation, &mut context).expect("Regex capture should succeed");
        
        let result = context.global_variables.get("tag").unwrap();
        // Greedy match captures everything between first < and last >
        assert_eq!(result.value, ResolvedValue::String("div>content</div".to_string()));
    }
}

#[cfg(test)]
mod count_tests {
    use super::*;
    use crate::types::common::{DataType, ResolvedValue};
    use crate::types::execution_context::ExecutionContext;
    use crate::types::resolution_context::{DeferredOperation, ResolutionContext};
    use crate::types::runtime_operation::RunParameter;
    use crate::types::variable::ResolvedVariable;
    use crate::types::RuntimeOperationType;

    /// Helper to create test context with a collection variable
    fn create_test_context_with_collection(var_name: &str, items: Vec<&str>) -> ExecutionContext {
        let mut res_context = ResolutionContext::new();
        
        let collection: Vec<ResolvedValue> = items
            .into_iter()
            .map(|s| ResolvedValue::String(s.to_string()))
            .collect();
        
        res_context.resolved_variables.insert(
            var_name.to_string(),
            ResolvedVariable::new(
                var_name.to_string(),
                DataType::String, // Collection of strings
                ResolvedValue::Collection(collection),
            ),
        );
        
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }
    
    /// Helper to create empty test context
    fn create_empty_test_context() -> ExecutionContext {
        let res_context = ResolutionContext::new();
        ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create execution context")
    }

    #[test]
    fn test_count_collection_basic() {
        let mut context = create_test_context_with_collection(
            "fruits",
            vec!["apple", "banana", "cherry"]
        );
        
        let operation = DeferredOperation {
            target_variable: "fruit_count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("fruits".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("fruit_count").expect("Target variable should exist");
        assert_eq!(result.value, ResolvedValue::Integer(3));
    }

    #[test]
    fn test_count_empty_collection() {
        let mut context = create_test_context_with_collection("empty", vec![]);
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("empty".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("count").unwrap();
        assert_eq!(result.value, ResolvedValue::Integer(0));
    }

    #[test]
    fn test_count_single_item_collection() {
        let mut context = create_test_context_with_collection("single", vec!["only_one"]);
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("single".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("count").unwrap();
        assert_eq!(result.value, ResolvedValue::Integer(1));
    }

    #[test]
    fn test_count_large_collection() {
        let items: Vec<&str> = (0..100).map(|_| "item").collect();
        let mut context = create_test_context_with_collection("large", items);
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("large".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("count").unwrap();
        assert_eq!(result.value, ResolvedValue::Integer(100));
    }

    #[test]
    fn test_count_non_collection_string() {
        let mut res_context = ResolutionContext::new();
        
        res_context.resolved_variables.insert(
            "text".to_string(),
            ResolvedVariable::new(
                "text".to_string(),
                DataType::String,
                ResolvedValue::String("hello".to_string()),
            ),
        );
        
        let mut context = ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create context");
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("text".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("count").unwrap();
        // Non-empty string counts as 1
        assert_eq!(result.value, ResolvedValue::Integer(1));
    }

    #[test]
    fn test_count_non_collection_empty_string() {
        let mut res_context = ResolutionContext::new();
        
        res_context.resolved_variables.insert(
            "empty".to_string(),
            ResolvedVariable::new(
                "empty".to_string(),
                DataType::String,
                ResolvedValue::String("".to_string()),
            ),
        );
        
        let mut context = ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create context");
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("empty".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("count").unwrap();
        // Empty string counts as 0
        assert_eq!(result.value, ResolvedValue::Integer(0));
    }

    #[test]
    fn test_count_non_collection_integer() {
        let mut res_context = ResolutionContext::new();
        
        res_context.resolved_variables.insert(
            "num".to_string(),
            ResolvedVariable::new(
                "num".to_string(),
                DataType::Int,
                ResolvedValue::Integer(42),
            ),
        );
        
        let mut context = ExecutionContext::from_resolution_context(&res_context)
            .expect("Failed to create context");
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("num".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("count").unwrap();
        // Single integer counts as 1
        assert_eq!(result.value, ResolvedValue::Integer(1));
    }

    #[test]
    fn test_count_missing_source_variable() {
        let mut context = create_empty_test_context();
        
        let operation = DeferredOperation {
            target_variable: "count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("nonexistent".to_string()),
            ],
        };
        
        let result = execute_count(&operation, &mut context);
        assert!(result.is_err());
        
        if let Err(ExecutionError::DeferredOperationFailed { operation, reason }) = result {
            assert_eq!(operation, "COUNT");
            assert!(reason.contains("not found"));
        } else {
            panic!("Expected DeferredOperationFailed error");
        }
    }

    #[test]
    fn test_count_after_split() {
        // Practical use case: count items after splitting
        let mut context = create_test_context_with_collection(
            "parts",
            vec!["a", "b", "c", "d", "e"]
        );
        
        let operation = DeferredOperation {
            target_variable: "part_count".to_string(),
            operation_type: RuntimeOperationType::Count,
            source_object_id: None,
            parameters: vec![
                RunParameter::Variable("parts".to_string()),
            ],
        };
        
        execute_count(&operation, &mut context).expect("Count should succeed");
        
        let result = context.global_variables.get("part_count").unwrap();
        assert_eq!(result.value, ResolvedValue::Integer(5));
    }
}