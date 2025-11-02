//! # Record Data Traits
//!
//! This module defines the core abstractions for handling structured record data
//! in the ICS SDK. It provides format-agnostic interfaces that allow the SDK
//! to work with JSON, XML, SQL results, PowerShell objects, and other structured
//! data sources without being tied to any specific format.
//!
//! ## Design Principles
//!
//! - **Format Agnostic**: Core SDK logic works with any data format
//! - **Extensible**: Scanner implementers can add custom record types
//! - **Type Safe**: Consistent interfaces with compile-time guarantees
//! - **Performance**: Each format can optimize its access patterns
//!
//! ## Usage
//!
//! ```rust
//! use ics_sdk::types::{RecordAccess, FieldPath, ResolvedValue};
//!
//! fn validate_record(record: &dyn RecordAccess, path: &FieldPath) -> bool {
//!     match record.get_field(path) {
//!         Ok(Some(ResolvedValue::String(s))) => !s.is_empty(),
//!         Ok(Some(ResolvedValue::Integer(i))) => i > 0,
//!         Ok(Some(_)) => true, // Other types are valid if present
//!         _ => false, // Field missing or error
//!     }
//! }
//! ```

use crate::types::common::{FieldPath, ResolvedValue};
use crate::types::RecordDataError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core trait for reading structured record data in any format
///
/// This trait provides a unified interface for accessing field data regardless
/// of the underlying storage format (JSON, XML, SQL, etc.). All record types
/// in the ICS system must implement this trait.
pub trait RecordAccess: Send + Sync + std::fmt::Debug {
    /// Get a field value by path, returning None if the field doesn't exist
    ///
    /// # Arguments
    /// * `path` - The field path to retrieve (supports dot notation)
    ///
    /// # Returns
    /// * `Ok(Some(value))` - Field exists and was successfully retrieved
    /// * `Ok(None)` - Field does not exist
    /// * `Err(error)` - Error occurred during field access
    fn get_field(&self, path: &FieldPath) -> Result<Option<ResolvedValue>, RecordError>;

    /// Check if a field exists at the given path without retrieving the value
    fn has_field(&self, path: &FieldPath) -> bool;

    /// List all available field paths in this record
    ///
    /// For nested structures, this returns flattened dot-notation paths.
    /// Large records may implement pagination or lazy loading.
    fn list_fields(&self) -> Vec<FieldPath>;

    /// Get the total number of fields in this record
    ///
    /// For nested structures, this counts all leaf fields.
    fn field_count(&self) -> usize;

    /// Get a hint about the underlying data format
    ///
    /// Returns format identifiers like "json", "xml", "powershell", "sql", etc.
    /// This helps the execution engine choose appropriate processing strategies.
    fn format_hint(&self) -> Option<&str>;

    /// Get a human-readable description of this record for debugging
    fn record_description(&self) -> String {
        format!(
            "{} record with {} fields",
            self.format_hint().unwrap_or("unknown"),
            self.field_count()
        )
    }

    /// Validate that this record's structure is consistent
    ///
    /// Default implementation always returns Ok, but specific formats
    /// can override this to perform validation checks.
    fn validate_structure(&self) -> Result<(), RecordError> {
        Ok(())
    }
}

/// Extension trait for modifying record data
///
/// Not all record types support modification (e.g., SQL query results may be read-only).
/// This trait is optional and only implemented by mutable record types.
pub trait RecordAccessMut: RecordAccess {
    /// Set a field value at the given path
    ///
    /// Creates intermediate path components if they don't exist.
    fn set_field(&mut self, path: &FieldPath, value: ResolvedValue) -> Result<(), RecordError>;

    /// Remove a field at the given path, returning the previous value if it existed
    fn remove_field(&mut self, path: &FieldPath) -> Result<Option<ResolvedValue>, RecordError>;

    /// Clear all fields from this record
    fn clear(&mut self) -> Result<(), RecordError>;

    /// Merge another record into this one
    ///
    /// Conflicting fields are overwritten with values from the other record.
    fn merge(&mut self, other: &dyn RecordAccess) -> Result<(), RecordError> {
        for field_path in other.list_fields() {
            if let Ok(Some(value)) = other.get_field(&field_path) {
                self.set_field(&field_path, value)?;
            }
        }
        Ok(())
    }
}

/// Factory trait for creating record instances from raw data
///
/// Adapters parse format-specific data (JSON bytes, XML documents, etc.)
/// into record instances that implement RecordAccess.
pub trait RecordAdapter: Send + Sync + std::fmt::Debug {
    /// Parse raw data into a record instance
    ///
    /// # Arguments
    /// * `data` - Raw bytes in the format this adapter understands
    ///
    /// # Returns
    /// * `Ok(record)` - Successfully parsed record
    /// * `Err(error)` - Parse error with details
    fn parse(&self, data: &[u8]) -> Result<Box<dyn RecordAccess>, RecordError>;

    /// Check if this adapter supports the given format identifier
    fn supports_format(&self, format: &str) -> bool;

    /// Get the name of this adapter for registry management
    fn adapter_name(&self) -> &str;

    /// Get all format identifiers this adapter supports
    fn supported_formats(&self) -> Vec<&str>;

    /// Validate raw data before parsing (optional optimization)
    ///
    /// Returns true if the data appears to be in the correct format.
    /// This can be used for format detection or early validation.
    fn can_parse(&self, data: &[u8]) -> bool {
        // Default implementation attempts to parse and checks for success
        self.parse(data).is_ok()
    }
}

/// Comprehensive error types for record operations
#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    /// Field does not exist at the specified path
    #[error("Field '{0}' not found")]
    FieldNotFound(String),

    /// Invalid field path syntax
    #[error("Invalid field path '{path}': {reason}")]
    InvalidPath { path: String, reason: String },

    /// Error parsing data into record format
    #[error("Parse error for format '{format}': {details}")]
    ParseError { format: String, details: String },

    /// Unsupported data format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Error converting between value types
    #[error("Type conversion error: {details}")]
    TypeConversion { details: String },

    /// Record structure is invalid or corrupted
    #[error("Invalid record structure: {details}")]
    InvalidStructure { details: String },

    /// Operation not supported by this record type
    #[error("Operation '{operation}' not supported by {record_type}")]
    UnsupportedOperation {
        operation: String,
        record_type: String,
    },

    /// Access denied (e.g., read-only record)
    #[error("Access denied: {reason}")]
    AccessDenied { reason: String },

    /// Field value is too large or record exceeds limits
    #[error("Resource limit exceeded: {details}")]
    ResourceLimit { details: String },

    /// Wrapped error from underlying format-specific libraries
    #[error("Underlying format error: {0}")]
    FormatSpecific(Box<dyn std::error::Error + Send + Sync>),
}

impl RecordError {
    /// Create a field not found error
    pub fn field_not_found(field_path: &FieldPath) -> Self {
        Self::FieldNotFound(field_path.to_dot_notation())
    }

    /// Create an invalid path error
    pub fn invalid_path(path: &str, reason: &str) -> Self {
        Self::InvalidPath {
            path: path.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create a parse error
    pub fn parse_error(format: &str, details: &str) -> Self {
        Self::ParseError {
            format: format.to_string(),
            details: details.to_string(),
        }
    }

    /// Create a type conversion error
    pub fn type_conversion(details: &str) -> Self {
        Self::TypeConversion {
            details: details.to_string(),
        }
    }

    /// Create an unsupported operation error
    pub fn unsupported_operation(operation: &str, record_type: &str) -> Self {
        Self::UnsupportedOperation {
            operation: operation.to_string(),
            record_type: record_type.to_string(),
        }
    }
}

/// Default JSON record implementation using your existing RecordData logic
///
/// This preserves backward compatibility while providing the new trait interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRecord {
    /// The underlying JSON data structure
    data: serde_json::Value,
    /// Optional metadata about this record
    metadata: Option<JsonRecordMetadata>,
}

/// Metadata for JSON records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRecordMetadata {
    /// Source of this record (e.g., "parameter", "select", "variable")
    pub source: Option<String>,
    /// Schema version if applicable
    pub schema_version: Option<String>,
    /// Additional context information
    pub context: HashMap<String, String>,
}

impl JsonRecord {
    /// Create a new JSON record from serde_json::Value
    pub fn from_json_value(data: serde_json::Value) -> Self {
        Self {
            data,
            metadata: None,
        }
    }

    /// Create a JSON record from a JSON string
    pub fn from_json_str(json_str: &str) -> Result<Self, RecordError> {
        let data = serde_json::from_str(json_str)
            .map_err(|e| RecordError::parse_error("json", &e.to_string()))?;
        Ok(Self::from_json_value(data))
    }

    /// Create a JSON record from field pairs (for object parameters/select)
    pub fn from_field_pairs(fields: &[(String, String)]) -> Self {
        let mut map = serde_json::Map::new();
        for (key, value) in fields {
            map.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        Self::from_json_value(serde_json::Value::Object(map))
    }

    /// Get direct access to the underlying JSON value
    pub fn as_json_value(&self) -> &serde_json::Value {
        &self.data
    }

    /// Set metadata for this record
    pub fn with_metadata(mut self, metadata: JsonRecordMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Convert ResolvedValue to serde_json::Value for storage
    fn resolved_value_to_json(value: &ResolvedValue) -> Result<serde_json::Value, RecordError> {
        match value {
            ResolvedValue::String(s) => Ok(serde_json::Value::String(s.clone())),
            ResolvedValue::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
            ResolvedValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| RecordError::type_conversion("Invalid float value")),
            ResolvedValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            ResolvedValue::RecordData(record_data) => Ok(record_data.as_json_value().clone()),
            ResolvedValue::Binary(_) => Err(RecordError::type_conversion(
                "Binary data cannot be stored in JSON record",
            )),
            ResolvedValue::Version(v) => Ok(serde_json::Value::String(v.clone())),
            ResolvedValue::EvrString(e) => Ok(serde_json::Value::String(e.clone())),
            ResolvedValue::Collection(items) => {
                let json_items: Result<Vec<serde_json::Value>, RecordDataError> = items
                    .iter()
                    .map(|item| match item {
                        ResolvedValue::String(s) => Ok(serde_json::Value::String(s.clone())),
                        ResolvedValue::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
                        ResolvedValue::Float(f) => serde_json::Number::from_f64(*f)
                            .map(serde_json::Value::Number)
                            .ok_or_else(|| {
                                RecordDataError::InvalidOperation("Invalid float value".to_string())
                            }),
                        ResolvedValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
                        _ => Err(RecordDataError::InvalidOperation(
                            "Unsupported collection item type".to_string(),
                        )),
                    })
                    .collect();

                json_items.map(serde_json::Value::Array).map_err(|e| {
                    RecordError::type_conversion(&format!("Collection conversion failed: {:?}", e))
                })
            }
        }
    }

    /// Convert serde_json::Value to ResolvedValue
    fn json_value_to_resolved(value: &serde_json::Value) -> Result<ResolvedValue, RecordError> {
        match value {
            serde_json::Value::String(s) => Ok(ResolvedValue::String(s.clone())),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(ResolvedValue::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(ResolvedValue::Float(f))
                } else {
                    Err(RecordError::type_conversion("Invalid number format"))
                }
            }
            serde_json::Value::Bool(b) => Ok(ResolvedValue::Boolean(*b)),
            serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                Ok(ResolvedValue::RecordData(
                    crate::types::common::RecordData::from_json_value(value.clone()),
                ))
            }
            serde_json::Value::Null => Err(RecordError::type_conversion(
                "Cannot convert null to ResolvedValue",
            )),
        }
    }
}

impl RecordAccess for JsonRecord {
    fn get_field(&self, path: &FieldPath) -> Result<Option<ResolvedValue>, RecordError> {
        // Navigate through the JSON structure using the field path
        let mut current = &self.data;

        for component in &path.components {
            current = match current.get(component) {
                Some(value) => value,
                None => return Ok(None), // Field doesn't exist
            };
        }

        // Convert the JSON value to ResolvedValue
        let resolved_value = Self::json_value_to_resolved(current)?;
        Ok(Some(resolved_value))
    }

    fn has_field(&self, path: &FieldPath) -> bool {
        let mut current = &self.data;

        for component in &path.components {
            current = match current.get(component) {
                Some(value) => value,
                None => return false,
            };
        }

        true
    }

    fn list_fields(&self) -> Vec<FieldPath> {
        let mut fields = Vec::new();
        self.collect_field_paths(&self.data, &[], &mut fields);
        fields
    }

    fn field_count(&self) -> usize {
        self.count_fields(&self.data)
    }

    fn format_hint(&self) -> Option<&str> {
        Some("json")
    }

    fn validate_structure(&self) -> Result<(), RecordError> {
        // JSON is always structurally valid if it parsed successfully
        Ok(())
    }
}

impl JsonRecord {
    /// Recursively collect field paths from JSON structure
    fn collect_field_paths(
        &self,
        value: &serde_json::Value,
        current_path: &[String],
        fields: &mut Vec<FieldPath>,
    ) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    let mut new_path = current_path.to_vec();
                    new_path.push(key.clone());

                    // Add this field path
                    fields.push(FieldPath::new(new_path.clone()));

                    // Recursively process nested objects/arrays
                    self.collect_field_paths(value, &new_path, fields);
                }
            }
            serde_json::Value::Array(arr) => {
                for (index, value) in arr.iter().enumerate() {
                    let mut new_path = current_path.to_vec();
                    new_path.push(index.to_string());

                    // Add this field path
                    fields.push(FieldPath::new(new_path.clone()));

                    // Recursively process array elements
                    self.collect_field_paths(value, &new_path, fields);
                }
            }
            _ => {
                // Leaf value - path already added by parent
            }
        }
    }

    /// Count total number of leaf fields (values that aren't objects or arrays)
    fn count_fields(&self, value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                if map.is_empty() {
                    1 // Empty object counts as one field
                } else {
                    map.values().map(|v| self.count_fields(v)).sum::<usize>()
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    1 // Empty array counts as one field
                } else {
                    arr.iter().map(|v| self.count_fields(v)).sum::<usize>()
                }
            }
            _ => 1, // Leaf value
        }
    }
}

impl RecordAccessMut for JsonRecord {
    fn set_field(&mut self, path: &FieldPath, value: ResolvedValue) -> Result<(), RecordError> {
        let json_value = Self::resolved_value_to_json(&value)?;

        // Navigate to the parent of the target field, creating structure as needed
        let mut current = &mut self.data;

        for (i, component) in path.components.iter().enumerate() {
            let is_last = i == path.components.len() - 1;

            if is_last {
                // Set the final field value
                match current {
                    serde_json::Value::Object(map) => {
                        map.insert(component.clone(), json_value);
                        return Ok(());
                    }
                    _ => {
                        return Err(RecordError::invalid_path(
                            &path.to_dot_notation(),
                            "Cannot set field on non-object value",
                        ));
                    }
                }
            } else {
                // Navigate or create intermediate path
                match current {
                    serde_json::Value::Object(map) => {
                        current = map
                            .entry(component.clone())
                            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
                    }
                    _ => {
                        return Err(RecordError::invalid_path(
                            &path.to_dot_notation(),
                            "Cannot navigate through non-object value",
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    fn remove_field(&mut self, path: &FieldPath) -> Result<Option<ResolvedValue>, RecordError> {
        if path.components.is_empty() {
            return Err(RecordError::invalid_path("", "Empty field path"));
        }

        // Navigate to the parent of the target field
        let mut current = &mut self.data;

        for component in &path.components[..path.components.len() - 1] {
            current = match current {
                serde_json::Value::Object(map) => {
                    match map.get_mut(component) {
                        Some(value) => value,
                        None => return Ok(None), // Parent doesn't exist
                    }
                }
                _ => return Ok(None), // Cannot navigate through non-object
            };
        }

        // Remove the final field
        let final_component = &path.components[path.components.len() - 1];
        match current {
            serde_json::Value::Object(map) => {
                let removed_value = map.remove(final_component);
                match removed_value {
                    Some(json_val) => {
                        let resolved_val = Self::json_value_to_resolved(&json_val)?;
                        Ok(Some(resolved_val))
                    }
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    fn clear(&mut self) -> Result<(), RecordError> {
        self.data = serde_json::Value::Object(serde_json::Map::new());
        Ok(())
    }
}

/// Default JSON adapter for creating JsonRecord instances
#[derive(Debug)]
pub struct JsonRecordAdapter {
    name: String,
}

impl JsonRecordAdapter {
    pub fn new() -> Self {
        Self {
            name: "json".to_string(),
        }
    }
}

impl Default for JsonRecordAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordAdapter for JsonRecordAdapter {
    fn parse(&self, data: &[u8]) -> Result<Box<dyn RecordAccess>, RecordError> {
        let json_str = std::str::from_utf8(data)
            .map_err(|e| RecordError::parse_error("json", &e.to_string()))?;

        let record = JsonRecord::from_json_str(json_str)?;
        Ok(Box::new(record))
    }

    fn supports_format(&self, format: &str) -> bool {
        matches!(format.to_lowercase().as_str(), "json" | "application/json")
    }

    fn adapter_name(&self) -> &str {
        &self.name
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["json", "application/json"]
    }

    fn can_parse(&self, data: &[u8]) -> bool {
        if let Ok(json_str) = std::str::from_utf8(data) {
            serde_json::from_str::<serde_json::Value>(json_str).is_ok()
        } else {
            false
        }
    }
}

/// Extension trait providing convenience methods for working with records
pub trait RecordAccessExt: RecordAccess {
    /// Get a field as a specific ResolvedValue variant
    fn get_string_field(&self, path: &FieldPath) -> Result<Option<String>, RecordError> {
        match self.get_field(path)? {
            Some(ResolvedValue::String(s)) => Ok(Some(s)),
            Some(_) => Err(RecordError::type_conversion("Field is not a string")),
            None => Ok(None),
        }
    }

    /// Get a field as an integer
    fn get_integer_field(&self, path: &FieldPath) -> Result<Option<i64>, RecordError> {
        match self.get_field(path)? {
            Some(ResolvedValue::Integer(i)) => Ok(Some(i)),
            Some(_) => Err(RecordError::type_conversion("Field is not an integer")),
            None => Ok(None),
        }
    }

    /// Get a field as a boolean
    fn get_boolean_field(&self, path: &FieldPath) -> Result<Option<bool>, RecordError> {
        match self.get_field(path)? {
            Some(ResolvedValue::Boolean(b)) => Ok(Some(b)),
            Some(_) => Err(RecordError::type_conversion("Field is not a boolean")),
            None => Ok(None),
        }
    }
}

/// Blanket implementation for all types that implement RecordAccess
impl<T: RecordAccess + ?Sized> RecordAccessExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::common::FieldPath;

    #[test]
    fn test_json_record_basic_operations() {
        let json_data = serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true,
            "nested": {
                "value": "inner"
            }
        });

        let record = JsonRecord::from_json_value(json_data);

        // Test field access
        let name_path = FieldPath::from_dot_notation("name");
        assert!(record.has_field(&name_path));

        let name_value = record.get_field(&name_path).unwrap();
        assert!(matches!(name_value, Some(ResolvedValue::String(ref s)) if s == "test"));

        // Test nested field access
        let nested_path = FieldPath::from_dot_notation("nested.value");
        assert!(record.has_field(&nested_path));

        let nested_value = record.get_field(&nested_path).unwrap();
        assert!(matches!(nested_value, Some(ResolvedValue::String(ref s)) if s == "inner"));

        // Test non-existent field
        let missing_path = FieldPath::from_dot_notation("missing");
        assert!(!record.has_field(&missing_path));
        assert!(matches!(record.get_field(&missing_path).unwrap(), None));
    }

    #[test]
    fn test_json_record_mutable_operations() {
        let mut record = JsonRecord::from_json_value(serde_json::json!({}));

        // Test setting fields
        let name_path = FieldPath::from_dot_notation("name");
        record
            .set_field(&name_path, ResolvedValue::String("test".to_string()))
            .unwrap();

        assert!(record.has_field(&name_path));
        let value = record.get_field(&name_path).unwrap();
        assert!(matches!(value, Some(ResolvedValue::String(ref s)) if s == "test"));

        // Test removing fields
        let removed = record.remove_field(&name_path).unwrap();
        assert!(matches!(removed, Some(ResolvedValue::String(ref s)) if s == "test"));
        assert!(!record.has_field(&name_path));
    }

    #[test]
    fn test_json_adapter() {
        let adapter = JsonRecordAdapter::new();

        let json_data = br#"{"name": "test", "count": 42}"#;
        let record = adapter.parse(json_data).unwrap();

        assert_eq!(record.format_hint(), Some("json"));
        assert_eq!(record.field_count(), 2);

        let name_path = FieldPath::from_dot_notation("name");
        assert!(record.has_field(&name_path));
    }
}
