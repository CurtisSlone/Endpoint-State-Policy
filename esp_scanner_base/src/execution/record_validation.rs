//! Record check validation logic
//!
//! Handles validation of RecordData against record checks from states.

use crate::execution::comparisons::ComparisonExt;
use crate::types::common::{DataType, Operation, RecordData, ResolvedValue};
use crate::types::execution_context::{
    ExecutableRecordCheck, ExecutableRecordContent, ExecutableRecordField,
};
use crate::types::field_path_extensions::FieldPathExt;
use crate::types::EntityCheck;

/// Result of validating a single record field or check
#[derive(Debug, Clone)]
pub struct RecordValidationResult {
    pub field_path: String,
    pub passed: bool,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

/// Validate all record checks against collected record data
pub fn validate_record_checks(
    record_data: &RecordData,
    record_checks: &[ExecutableRecordCheck],
) -> Result<Vec<RecordValidationResult>, String> {
    let mut results = Vec::new();

    for check in record_checks {
        match &check.content {
            ExecutableRecordContent::Direct { operation, value } => {
                let result = validate_direct_record(record_data, *operation, value)?;
                results.push(result);
            }
            ExecutableRecordContent::Nested { fields } => {
                let field_results = validate_nested_fields(record_data, fields)?;
                results.extend(field_results);
            }
        }
    }

    Ok(results)
}

/// Validate entire record with direct operation
fn validate_direct_record(
    record_data: &RecordData,
    operation: Operation,
    expected: &ResolvedValue,
) -> Result<RecordValidationResult, String> {
    let actual = ResolvedValue::RecordData(Box::new(record_data.clone()));

    let passed = actual
        .compare_with(expected, operation)
        .map_err(|e| format!("Direct record comparison failed: {}", e))?;

    Ok(RecordValidationResult {
        field_path: "<record>".to_string(),
        passed,
        message: if passed {
            "Record validation passed".to_string()
        } else {
            format!("Record validation failed for operation {:?}", operation)
        },
        expected: Some(format!("{:?}", expected)),
        actual: Some("<record_data>".to_string()),
    })
}

/// Validate nested fields in record
fn validate_nested_fields(
    record_data: &RecordData,
    fields: &[ExecutableRecordField],
) -> Result<Vec<RecordValidationResult>, String> {
    let mut results = Vec::new();

    for field in fields {
        // Check if field path has wildcards
        if field.path.has_wildcards() {
            // Collection validation with entity check
            let collection_result = validate_field_collection(record_data, field)?;
            results.push(collection_result);
        } else {
            // Single value validation (existing logic)
            let single_result = validate_field_single(record_data, field)?;
            results.push(single_result);
        }
    }

    Ok(results)
}

/// Validate a field with wildcard path (collection validation)
fn validate_field_collection(
    _record_data: &RecordData,
    field: &ExecutableRecordField,
) -> Result<RecordValidationResult, String> {
    // Resolve wildcard path to get all matching values
    let json_values: Vec<&serde_json::Value> = Vec::new();

    if json_values.is_empty() {
        return Ok(RecordValidationResult {
            field_path: field.path.to_dot_notation(),
            passed: false,
            message: "No values matched wildcard path".to_string(),
            expected: Some(format!("{:?}", field.value)),
            actual: None,
        });
    }

    // Convert each JSON value and perform comparison
    let mut comparison_results = Vec::new();
    let mut actual_values = Vec::new();

    for json_value in &json_values {
        // Convert JSON to ResolvedValue
        let actual_value = json_to_resolved_value(json_value, field.data_type)
            .map_err(|e| format!("Type conversion failed: {}", e))?;

        // Perform comparison using the appropriate comparison function
        let passed = match (&actual_value, &field.value, field.operation) {
            // String comparisons
            (ResolvedValue::String(actual), ResolvedValue::String(expected), op) => {
                use crate::execution::comparisons::string;
                string::compare(actual, expected, op).unwrap_or(false)
            }

            // Integer comparisons
            (ResolvedValue::Integer(actual), ResolvedValue::Integer(expected), op) => match op {
                Operation::Equals => actual == expected,
                Operation::NotEqual => actual != expected,
                Operation::GreaterThan => actual > expected,
                Operation::LessThan => actual < expected,
                Operation::GreaterThanOrEqual => actual >= expected,
                Operation::LessThanOrEqual => actual <= expected,
                _ => false,
            },

            // Float comparisons
            (ResolvedValue::Float(actual), ResolvedValue::Float(expected), op) => match op {
                Operation::Equals => (actual - expected).abs() < f64::EPSILON,
                Operation::NotEqual => (actual - expected).abs() >= f64::EPSILON,
                Operation::GreaterThan => actual > expected,
                Operation::LessThan => actual < expected,
                Operation::GreaterThanOrEqual => actual >= expected,
                Operation::LessThanOrEqual => actual <= expected,
                _ => false,
            },

            // Boolean comparisons
            (ResolvedValue::Boolean(actual), ResolvedValue::Boolean(expected), op) => match op {
                Operation::Equals => actual == expected,
                Operation::NotEqual => actual != expected,
                _ => false,
            },

            // Type mismatch or unsupported operation
            _ => false,
        };

        comparison_results.push(passed);
        actual_values.push(actual_value);
    }

    // Apply entity check to aggregate results
    let entity_check = field.entity_check.unwrap_or(EntityCheck::All);
    let final_passed = apply_entity_check_to_collection(&comparison_results, entity_check);

    // Create detailed message
    let passing_count = comparison_results.iter().filter(|&&p| p).count();
    let total_count = comparison_results.len();

    let message = if final_passed {
        format!(
            "Collection validation passed ({}): {} of {} items matched (entity check: {})",
            field.path.to_dot_notation(),
            passing_count,
            total_count,
            entity_check.as_str()
        )
    } else {
        format!(
            "Collection validation failed ({}): {} of {} items matched (entity check: {} requires {})",
            field.path.to_dot_notation(),
            passing_count,
            total_count,
            entity_check.as_str(),
            match entity_check {
                EntityCheck::All => "all items",
                EntityCheck::AtLeastOne => "at least one item",
                EntityCheck::None => "no items",
                EntityCheck::OnlyOne => "exactly one item",
            }
        )
    };

    Ok(RecordValidationResult {
        field_path: field.path.to_dot_notation(),
        passed: final_passed,
        message,
        expected: Some(format!("{:?}", field.value)),
        actual: Some(format!(
            "{} values: {:?}",
            actual_values.len(),
            actual_values
        )),
    })
}

/// Validate a single field value
fn validate_field_single(
    record_data: &RecordData,
    field: &ExecutableRecordField,
) -> Result<RecordValidationResult, String> {
    // Extract field value from record using path
    // FIXED: get_field_by_path returns Option, not Result
    let json_value = match record_data.get_field_by_path(&field.path.to_dot_notation()) {
        Some(v) => v,
        None => {
            return Ok(RecordValidationResult {
                field_path: field.path.to_dot_notation(),
                passed: false,
                message: "Field not found".to_string(),
                expected: Some(format!("{:?}", field.value)),
                actual: None,
            });
        }
    };

    // Convert JSON value to ResolvedValue
    let actual_value = match json_to_resolved_value(json_value, field.data_type) {
        Ok(v) => v,
        Err(e) => {
            return Ok(RecordValidationResult {
                field_path: field.path.to_dot_notation(),
                passed: false,
                message: format!("Type conversion failed: {}", e),
                expected: Some(format!("{:?}", field.value)),
                actual: Some(format!("{:?}", json_value)),
            });
        }
    };

    // Perform comparison
    let comparison_passed = actual_value
        .compare_with(&field.value, field.operation)
        .unwrap_or_else(|_e| {
            // If comparison trait fails, try direct comparison for simple types
            match (field.operation, &actual_value, &field.value) {
                (Operation::Equals, a, b) => a == b,
                (Operation::NotEqual, a, b) => a != b,
                (Operation::GreaterThan, ResolvedValue::Integer(a), ResolvedValue::Integer(b)) => {
                    a > b
                }
                (Operation::LessThan, ResolvedValue::Integer(a), ResolvedValue::Integer(b)) => {
                    a < b
                }
                (
                    Operation::GreaterThanOrEqual,
                    ResolvedValue::Integer(a),
                    ResolvedValue::Integer(b),
                ) => a >= b,
                (
                    Operation::LessThanOrEqual,
                    ResolvedValue::Integer(a),
                    ResolvedValue::Integer(b),
                ) => a <= b,
                _ => false,
            }
        });

    // Apply entity check if present (for single values)
    let final_passed = if let Some(entity_check) = field.entity_check {
        apply_entity_check(comparison_passed, entity_check)
    } else {
        comparison_passed
    };

    Ok(RecordValidationResult {
        field_path: field.path.to_dot_notation(),
        passed: final_passed,
        message: if final_passed {
            format!("Field '{}' validation passed", field.path.to_dot_notation())
        } else {
            format!(
                "Field '{}' validation failed: expected {} {:?} {}",
                field.path.to_dot_notation(),
                format_value(&field.value),
                field.operation,
                format_value(&actual_value)
            )
        },
        expected: Some(format_value(&field.value)),
        actual: Some(format_value(&actual_value)),
    })
}

/// Convert JSON value to ResolvedValue with type hint
fn json_to_resolved_value(
    json: &serde_json::Value,
    data_type: DataType,
) -> Result<ResolvedValue, String> {
    match (json, data_type) {
        (serde_json::Value::String(s), DataType::String) => Ok(ResolvedValue::String(s.clone())),
        (serde_json::Value::Number(n), DataType::Int) => n
            .as_i64()
            .map(ResolvedValue::Integer)
            .ok_or_else(|| "Number is not a valid integer".to_string()),
        (serde_json::Value::Number(n), DataType::Float) => n
            .as_f64()
            .map(ResolvedValue::Float)
            .ok_or_else(|| "Number is not a valid float".to_string()),
        // FIXED: Remove dereference - bools are Copy
        (serde_json::Value::Bool(b), DataType::Boolean) => Ok(ResolvedValue::Boolean(*b)),
        (serde_json::Value::Array(items), _) => {
            let resolved_items: Result<Vec<_>, _> = items
                .iter()
                .map(|item| json_to_resolved_value(item, data_type))
                .collect();
            Ok(ResolvedValue::Collection(resolved_items?))
        }
        (serde_json::Value::Object(_), DataType::RecordData) => Ok(ResolvedValue::RecordData(
            Box::new(RecordData::from_json_value(json.clone())),
        )),
        _ => Err(format!(
            "Cannot convert JSON type {:?} to {:?}",
            json, data_type
        )),
    }
}

/// Apply entity check to validation result (for single values)
fn apply_entity_check(passed: bool, entity_check: EntityCheck) -> bool {
    match entity_check {
        EntityCheck::All => passed,        // Must pass
        EntityCheck::AtLeastOne => passed, // Must pass
        EntityCheck::None => !passed,      // Must fail
        EntityCheck::OnlyOne => passed,    // Must pass (single value)
    }
}

/// Apply entity check to collection of boolean results
fn apply_entity_check_to_collection(results: &[bool], entity_check: EntityCheck) -> bool {
    match entity_check {
        EntityCheck::All => results.iter().all(|&r| r),
        EntityCheck::AtLeastOne => results.iter().any(|&r| r),
        EntityCheck::None => !results.iter().any(|&r| r),
        EntityCheck::OnlyOne => results.iter().filter(|&&r| r).count() == 1,
    }
}

/// Format a ResolvedValue for display
fn format_value(value: &ResolvedValue) -> String {
    match value {
        ResolvedValue::String(s) => format!("\"{}\"", s),
        ResolvedValue::Integer(i) => i.to_string(),
        ResolvedValue::Float(f) => f.to_string(),
        ResolvedValue::Boolean(b) => b.to_string(),
        ResolvedValue::Version(v) => format!("version({})", v),
        ResolvedValue::EvrString(e) => format!("evr({})", e),
        ResolvedValue::Binary(b) => format!("binary({} bytes)", b.len()),
        ResolvedValue::RecordData(_) => "<record>".to_string(),
        ResolvedValue::Collection(items) => format!("[{}]", items.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_resolved_value() {
        let json = serde_json::json!("test");
        let result = json_to_resolved_value(&json, DataType::String);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResolvedValue::String("test".to_string()));
    }

    #[test]
    fn test_json_to_integer() {
        let json = serde_json::json!(42);
        let result = json_to_resolved_value(&json, DataType::Int);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResolvedValue::Integer(42));
    }

    #[test]
    fn test_json_to_boolean() {
        let json = serde_json::json!(true);
        let result = json_to_resolved_value(&json, DataType::Boolean);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ResolvedValue::Boolean(true));
    }

    #[test]
    fn test_entity_check_application() {
        assert_eq!(apply_entity_check(true, EntityCheck::All), true);
        assert_eq!(apply_entity_check(false, EntityCheck::None), true);
        assert_eq!(apply_entity_check(true, EntityCheck::None), false);
        assert_eq!(apply_entity_check(true, EntityCheck::AtLeastOne), true);
    }

    #[test]
    fn test_entity_check_collection_all() {
        assert_eq!(
            apply_entity_check_to_collection(&[true, true, true], EntityCheck::All),
            true
        );
        assert_eq!(
            apply_entity_check_to_collection(&[true, false, true], EntityCheck::All),
            false
        );
    }

    #[test]
    fn test_entity_check_collection_at_least_one() {
        assert_eq!(
            apply_entity_check_to_collection(&[false, false, true], EntityCheck::AtLeastOne),
            true
        );
        assert_eq!(
            apply_entity_check_to_collection(&[false, false, false], EntityCheck::AtLeastOne),
            false
        );
    }

    #[test]
    fn test_entity_check_collection_none() {
        assert_eq!(
            apply_entity_check_to_collection(&[false, false, false], EntityCheck::None),
            true
        );
        assert_eq!(
            apply_entity_check_to_collection(&[true, false, false], EntityCheck::None),
            false
        );
    }

    #[test]
    fn test_entity_check_collection_only_one() {
        assert_eq!(
            apply_entity_check_to_collection(&[false, true, false], EntityCheck::OnlyOne),
            true
        );
        assert_eq!(
            apply_entity_check_to_collection(&[true, true, false], EntityCheck::OnlyOne),
            false
        );
        assert_eq!(
            apply_entity_check_to_collection(&[false, false, false], EntityCheck::OnlyOne),
            false
        );
    }

    #[test]
    fn test_format_value() {
        assert_eq!(
            format_value(&ResolvedValue::String("test".to_string())),
            "\"test\""
        );
        assert_eq!(format_value(&ResolvedValue::Integer(42)), "42");
        assert_eq!(format_value(&ResolvedValue::Boolean(true)), "true");
    }
}
