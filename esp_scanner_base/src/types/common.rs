use crate::types::RecordAccess;
use serde::{Deserialize, Serialize};

/// Data types supported in ICS (EBNF: data_type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    String,
    Int,
    Float,
    Boolean,
    Binary,
    RecordData,
    Version,
    EvrString,
}

impl DataType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "string" => Some(Self::String),
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "boolean" => Some(Self::Boolean),
            "binary" => Some(Self::Binary),
            "record" | "recorddata" | "record_data" => Some(Self::RecordData),
            "version" => Some(Self::Version),
            "evr_string" | "evrstring" => Some(Self::EvrString),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Binary => "binary",
            Self::RecordData => "record_data",
            Self::Version => "version",
            Self::EvrString => "evr_string",
        }
    }

    /// Check if a ResolvedValue variant matches this data type
    pub fn matches_resolved_value(&self, value: &ResolvedValue) -> bool {
        match (self, value) {
            (DataType::String, ResolvedValue::String(_)) => true,
            (DataType::Int, ResolvedValue::Integer(_)) => true,
            (DataType::Float, ResolvedValue::Float(_)) => true,
            (DataType::Boolean, ResolvedValue::Boolean(_)) => true,
            (DataType::Binary, ResolvedValue::Binary(_)) => true,
            (DataType::RecordData, ResolvedValue::RecordData(_)) => true,
            (DataType::Version, ResolvedValue::Version(_)) => true,
            (DataType::EvrString, ResolvedValue::EvrString(_)) => true,
            // Allow int->float conversion
            (DataType::Float, ResolvedValue::Integer(_)) => true,
            _ => false,
        }
    }
}

/// Value specification as per EBNF (value_spec)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// String literal (backtick_string)
    String(String),
    /// Integer value (integer_value) - 64-bit signed per EBNF limits
    Integer(i64),
    /// Float value (for metadata) - IEEE 754 double precision
    Float(f64),
    /// Boolean value (boolean_value)
    Boolean(bool),
    /// Variable reference (VAR variable_name)
    Variable(String),
}

impl Value {
    /// Create a string value
    pub fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// Create an integer value
    pub fn integer(i: i64) -> Self {
        Self::Integer(i)
    }

    /// Create a float value
    pub fn float(f: f64) -> Self {
        Self::Float(f)
    }

    /// Create a boolean value
    pub fn boolean(b: bool) -> Self {
        Self::Boolean(b)
    }

    /// Create a variable reference
    pub fn variable(var: impl Into<String>) -> Self {
        Self::Variable(var.into())
    }

    /// Check if this is a variable reference
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable(_))
    }

    /// Check if this value is a variable reference
    pub fn has_variable_reference(&self) -> bool {
        matches!(self, Value::Variable(_))
    }

    /// Get the variable name if this is a variable reference
    pub fn get_variable_name(&self) -> Option<&str> {
        match self {
            Value::Variable(name) => Some(name),
            _ => None,
        }
    }
}

/// Resolved value with all variable references substituted
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolvedValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    RecordData(RecordData),
    Binary(Vec<u8>),
    Version(String),
    EvrString(String),
    Collection(Vec<ResolvedValue>),
}

impl ResolvedValue {
    /// Convert RecordData to trait object when needed for validation
    pub fn as_record_access(&self) -> Option<Box<dyn crate::types::RecordAccess>> {
        match self {
            ResolvedValue::RecordData(record_data) => {
                Some(record_data.clone().to_record_access()) // Clone the data
            }
            _ => None,
        }
    }
}

/// Container for structured data used throughout the ICS scanner system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordData {
    /// The underlying JSON data structure
    pub data: serde_json::Value,
}

impl RecordData {
    /// Create from JSON string (for variable resolution)
    pub fn from_json_str(json_str: &str) -> Result<Self, RecordDataError> {
        let data = serde_json::from_str(json_str)?;
        Ok(Self { data })
    }

    /// Create from serde_json::Value (for object outputs)
    pub fn from_json_value(data: serde_json::Value) -> Self {
        Self { data }
    }

    /// Create from key-value field pairs (for object parameters/select)
    pub fn from_field_pairs(fields: &[(String, String)]) -> Self {
        let mut map = serde_json::Map::new();
        for (key, value) in fields {
            map.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        Self {
            data: serde_json::Value::Object(map),
        }
    }

    /// Get direct access to underlying JSON value
    pub fn as_json_value(&self) -> &serde_json::Value {
        &self.data
    }

    /// Extract field using dot notation path
    pub fn get_field_by_path(
        &self,
        path: &[String],
    ) -> Result<&serde_json::Value, RecordDataError> {
        let mut current = &self.data;

        for component in path {
            current = current
                .get(component)
                .ok_or_else(|| RecordDataError::FieldNotFound(component.clone()))?;
        }

        Ok(current)
    }

    /// Check if field exists at path
    pub fn has_field(&self, path: &[String]) -> bool {
        self.get_field_by_path(path).is_ok()
    }

    pub fn to_record_access(self) -> Box<dyn RecordAccess> {
        Box::new(crate::types::JsonRecord::from_json_value(self.data))
    }

    /// Create from trait-based record
    pub fn from_record_access(record: &dyn RecordAccess) -> Result<Self, RecordDataError> {
        // Only support JSON records for now
        if record.format_hint() != Some("json") {
            return Err(RecordDataError::InvalidOperation(
                "Only JSON records are currently supported".to_string(),
            ));
        }

        // Collect all fields and reconstruct as JSON
        let mut json_map = serde_json::Map::new();

        for field_path in record.list_fields() {
            if let Ok(Some(value)) = record.get_field(&field_path) {
                let json_value = Self::resolved_value_to_json(&value)?;
                Self::set_nested_json_field(&mut json_map, &field_path.components, json_value)?;
            }
        }

        Ok(Self {
            data: serde_json::Value::Object(json_map),
        })
    }

    /// Helper to convert ResolvedValue to JSON
    fn resolved_value_to_json(
        value: &crate::types::ResolvedValue,
    ) -> Result<serde_json::Value, RecordDataError> {
        match value {
            crate::types::ResolvedValue::String(s) => Ok(serde_json::Value::String(s.clone())),
            crate::types::ResolvedValue::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
            crate::types::ResolvedValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| {
                    RecordDataError::InvalidOperation("Invalid float value".to_string())
                }),
            crate::types::ResolvedValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            crate::types::ResolvedValue::RecordData(record_data) => {
                Ok(record_data.as_json_value().clone())
            }
            crate::types::ResolvedValue::Binary(_) => Err(RecordDataError::InvalidOperation(
                "Binary data cannot be stored in JSON".to_string(),
            )),
            crate::types::ResolvedValue::Version(v) => Ok(serde_json::Value::String(v.clone())),
            crate::types::ResolvedValue::EvrString(e) => Ok(serde_json::Value::String(e.clone())),
            crate::types::ResolvedValue::Collection(items) => {
                let json_items: Result<Vec<serde_json::Value>, RecordDataError> = items
                    .iter()
                    .map(|item| Self::resolved_value_to_json(item)) // Add Self::
                    .collect();
                Ok(serde_json::Value::Array(json_items?))
            }
        }
    }

    /// Helper to set nested JSON fields from field path
    fn set_nested_json_field(
        map: &mut serde_json::Map<String, serde_json::Value>,
        path: &[String],
        value: serde_json::Value,
    ) -> Result<(), RecordDataError> {
        if path.is_empty() {
            return Err(RecordDataError::InvalidOperation(
                "Empty field path".to_string(),
            ));
        }

        if path.len() == 1 {
            map.insert(path[0].clone(), value);
            return Ok(());
        }

        // Navigate/create nested structure
        let current_key = &path[0];
        let remaining_path = &path[1..];

        let nested_map = map
            .entry(current_key.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

        match nested_map {
            serde_json::Value::Object(nested_obj) => {
                Self::set_nested_json_field(nested_obj, remaining_path, value)
            }
            _ => Err(RecordDataError::InvalidOperation(
                "Cannot create nested structure in non-object value".to_string(),
            )),
        }
    }

    /// Resolve a field path that may contain wildcards, returning all matching values
    /// 
    /// Supports:
    /// - `*` - matches any key in an object
    /// - `[*]` - matches all elements in an array
    pub fn resolve_path_with_wildcards(
        &self,
        path: &[String],
    ) -> Result<Vec<&serde_json::Value>, RecordDataError> {
        if path.is_empty() {
            return Ok(vec![&self.data]);
        }

        // Start with the root value
        let mut current_values = vec![&self.data];
        
        // Process each path component
        for component in path {
            if Self::is_wildcard_component(component) {
                // Expand wildcard: collect all matching values
                current_values = self.expand_wildcard(&current_values, component)?;
            } else {
                // Navigate to specific field
                current_values = self.navigate_to_field(&current_values, component)?;
            }
            
            // If we have no values at any point, we're done
            if current_values.is_empty() {
                return Ok(Vec::new());
            }
        }
        
        Ok(current_values)
    }

    /// Check if a path component is a wildcard
    fn is_wildcard_component(component: &str) -> bool {
        component == "*" || component == "[*]"
    }

    /// Expand a wildcard component across all current values
    fn expand_wildcard<'a>(
        &'a self,
        current_values: &[&'a serde_json::Value],
        wildcard: &str,
    ) -> Result<Vec<&'a serde_json::Value>, RecordDataError> {
        let mut expanded = Vec::new();
        
        for value in current_values {
            match (wildcard, *value) {
                // Object wildcard: collect all values from the object
                ("*", serde_json::Value::Object(map)) => {
                    expanded.extend(map.values());
                }
                // Array wildcard: collect all array elements
                ("[*]", serde_json::Value::Array(arr)) => {
                    expanded.extend(arr.iter());
                }
                // Array wildcard also works with "*" for backward compatibility
                ("*", serde_json::Value::Array(arr)) => {
                    expanded.extend(arr.iter());
                }
                // Wildcard on non-collection type: skip this value
                _ => {
                    // Don't error, just skip - allows partial matching
                    continue;
                }
            }
        }
        
        Ok(expanded)
    }

    /// Navigate to a specific field across all current values
    fn navigate_to_field<'a>(
        &'a self,
        current_values: &[&'a serde_json::Value],
        field_name: &str,
    ) -> Result<Vec<&'a serde_json::Value>, RecordDataError> {
        let mut navigated = Vec::new();
        
        for value in current_values {
            // Handle array index notation: field[0], field[1], etc.
            if let Some((base_field, index_str)) = Self::parse_array_index(field_name) {
                // First navigate to the base field
                if let serde_json::Value::Object(map) = value {
                    if let Some(array_value) = map.get(base_field) {
                        if let serde_json::Value::Array(arr) = array_value {
                            if let Ok(index) = index_str.parse::<usize>() {
                                if let Some(element) = arr.get(index) {
                                    navigated.push(element);
                                }
                            }
                        }
                    }
                }
            } else {
                // Regular field navigation
                match value {
                    serde_json::Value::Object(map) => {
                        if let Some(field_value) = map.get(field_name) {
                            navigated.push(field_value);
                        }
                    }
                    serde_json::Value::Array(arr) => {
                        // If navigating through an array without index, treat as wildcard
                        for element in arr {
                            if let serde_json::Value::Object(map) = element {
                                if let Some(field_value) = map.get(field_name) {
                                    navigated.push(field_value);
                                }
                            }
                        }
                    }
                    _ => {
                        // Can't navigate through scalar value - skip
                        continue;
                    }
                }
            }
        }
        
        Ok(navigated)
    }

    /// Parse array index notation: "field[0]" -> Some(("field", "0"))
    fn parse_array_index(field_name: &str) -> Option<(&str, &str)> {
        if let Some(open_bracket) = field_name.find('[') {
            if let Some(close_bracket) = field_name.find(']') {
                if close_bracket > open_bracket + 1 {
                    let base = &field_name[..open_bracket];
                    let index = &field_name[open_bracket + 1..close_bracket];
                    return Some((base, index));
                }
            }
        }
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RecordDataError {
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Field '{0}' not found in record data")]
    FieldNotFound(String),

    #[error("Invalid operation on record data: {0}")]
    InvalidOperation(String),
}

/// Field path for record navigation (dot notation)
/// EBNF: field_path ::= identifier ("." identifier)*
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Ord, Eq, PartialOrd)]
pub struct FieldPath {
    pub components: Vec<String>,
}

impl FieldPath {
    pub fn new(components: Vec<String>) -> Self {
        Self { components }
    }

    pub fn from_dot_notation(path: &str) -> Self {
        Self {
            components: path.split('.').map(|s| s.to_string()).collect(),
        }
    }

    pub fn to_dot_notation(&self) -> String {
        self.components.join(".")
    }

    /// Resolve this path against RecordData
    pub fn resolve<'a>(
        &self,
        data: &'a RecordData,
    ) -> Result<&'a serde_json::Value, RecordDataError> {
        data.get_field_by_path(&self.components)
    }

    /// Check if this is a simple (single component) field path
    pub fn is_simple(&self) -> bool {
        self.components.len() == 1
    }

     pub fn has_wildcards(&self) -> bool {
        self.components.iter().any(|comp| {
            comp == "*" || comp == "[*]" || comp.contains("*")
        })
    }

    /// Check if a specific component is a wildcard
    pub fn is_wildcard_component(component: &str) -> bool {
        component == "*" || component == "[*]" || component.contains("*")
    }

    /// Split path into segments at wildcard boundaries
    /// Returns (prefix_before_wildcard, wildcard_component, suffix_after_wildcard)
    pub fn split_at_wildcard(&self) -> Option<(Vec<String>, String, Vec<String>)> {
        for (idx, comp) in self.components.iter().enumerate() {
            if Self::is_wildcard_component(comp) {
                let prefix = self.components[..idx].to_vec();
                let wildcard = comp.clone();
                let suffix = self.components[idx + 1..].to_vec();
                return Some((prefix, wildcard, suffix));
            }
        }
        None
    }

    /// Get all wildcard positions in this path
    /// Returns indices of components that are wildcards
    pub fn wildcard_positions(&self) -> Vec<usize> {
        self.components
            .iter()
            .enumerate()
            .filter_map(|(idx, comp)| {
                if Self::is_wildcard_component(comp) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if this path has multiple wildcards (nested wildcards)
    pub fn has_nested_wildcards(&self) -> bool {
        self.wildcard_positions().len() > 1
    }
}

/// Operations for field validation (EBNF: operation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operation {
    // Comparison operations (EBNF: comparison_op)
    Equals,             // =
    NotEqual,           // !=
    GreaterThan,        // >
    LessThan,           // <
    GreaterThanOrEqual, // >=
    LessThanOrEqual,    // <=
    // String operations (EBNF: string_op)
    CaseInsensitiveEquals,   // ieq
    CaseInsensitiveNotEqual, // ine
    Contains,                // contains
    StartsWith,              // starts
    EndsWith,                // ends
    NotContains,             // not_contains
    NotStartsWith,           // not_starts
    NotEndsWith,             // not_ends
    // Pattern operations (EBNF: pattern_op)
    PatternMatch, // pattern_match
    Matches,      // matches
    // Set operations (EBNF: set_op)
    SubsetOf,   // subset_of
    SupersetOf, // superset_of
}

impl Operation {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // ICS syntax
            "=" => Some(Operation::Equals),
            "!=" | "≠" => Some(Operation::NotEqual),
            ">" => Some(Operation::GreaterThan),
            "<" => Some(Operation::LessThan),
            ">=" | "≥" => Some(Operation::GreaterThanOrEqual),
            "<=" | "≤" => Some(Operation::LessThanOrEqual),
            "ieq" => Some(Operation::CaseInsensitiveEquals),
            "ine" => Some(Operation::CaseInsensitiveNotEqual),
            "contains" => Some(Operation::Contains),
            "not_contains" => Some(Operation::NotContains),
            "starts" | "starts_with" => Some(Operation::StartsWith),
            "ends" | "ends_with" => Some(Operation::EndsWith),
            "not_starts" => Some(Operation::NotStartsWith),
            "not_ends" => Some(Operation::NotEndsWith),
            "pattern_match" => Some(Operation::PatternMatch),
            "matches" => Some(Operation::Matches),
            "subset_of" => Some(Operation::SubsetOf),
            "superset_of" => Some(Operation::SupersetOf),

            // JSON/Enum style (what's coming from the parsed AST)
            "Equals" => Some(Operation::Equals),
            "NotEqual" => Some(Operation::NotEqual),
            "GreaterThan" => Some(Operation::GreaterThan),
            "LessThan" => Some(Operation::LessThan),
            "GreaterThanOrEqual" => Some(Operation::GreaterThanOrEqual),
            "LessThanOrEqual" => Some(Operation::LessThanOrEqual),
            "CaseInsensitiveEquals" => Some(Operation::CaseInsensitiveEquals), // ADD
            "CaseInsensitiveNotEqual" => Some(Operation::CaseInsensitiveNotEqual), // ADD
            "Contains" => Some(Operation::Contains),
            "NotContains" => Some(Operation::NotContains),
            "StartsWith" => Some(Operation::StartsWith),
            "EndsWith" => Some(Operation::EndsWith),
            "NotStartsWith" => Some(Operation::NotStartsWith), // ADD
            "NotEndsWith" => Some(Operation::NotEndsWith),     // ADD
            "PatternMatch" => Some(Operation::PatternMatch),
            "Matches" => Some(Operation::Matches),       // ADD
            "SubsetOf" => Some(Operation::SubsetOf),     // ADD
            "SupersetOf" => Some(Operation::SupersetOf), // ADD

            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equals => "=",
            Self::NotEqual => "!=",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::GreaterThanOrEqual => ">=",
            Self::LessThanOrEqual => "<=",
            Self::CaseInsensitiveEquals => "ieq",
            Self::CaseInsensitiveNotEqual => "ine",
            Self::Contains => "contains",
            Self::StartsWith => "starts",
            Self::EndsWith => "ends",
            Self::NotContains => "not_contains",
            Self::NotStartsWith => "not_starts",
            Self::NotEndsWith => "not_ends",
            Self::PatternMatch => "pattern_match",
            Self::Matches => "matches",
            Self::SubsetOf => "subset_of",
            Self::SupersetOf => "superset_of",
        }
    }
}

// Add this to your common.rs file after the Operation enum

/// Logical operators for criteria (EBNF: logical_operator)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogicalOp {
    And, // AND
    Or,  // OR
}

impl LogicalOp {
    /// Parse logical operator (case-sensitive, uppercase)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // ICS syntax (from original file)
            "AND" => Some(LogicalOp::And),
            "OR" => Some(LogicalOp::Or),

            // JSON/Enum style (from parsed AST)
            "And" => Some(LogicalOp::And),
            "Or" => Some(LogicalOp::Or),

            // Case variations
            "and" => Some(LogicalOp::And),
            "or" => Some(LogicalOp::Or),

            _ => None,
        }
    }

    /// Get the operator as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
        }
    }

    /// Check if this operator is commutative
    pub fn is_commutative(&self) -> bool {
        true // Both AND and OR are commutative
    }

    /// Get the identity element for this operator
    pub fn identity_value(&self) -> bool {
        match self {
            Self::And => true, // AND identity: true
            Self::Or => false, // OR identity: false
        }
    }
}

impl std::fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Display implementations
impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.components.join("."))
    }
}

#[cfg(test)]
mod field_path_tests {
    use super::*;

    #[test]
    fn test_has_wildcards() {
        // Simple paths without wildcards
        let path = FieldPath::from_dot_notation("user.name");
        assert!(!path.has_wildcards());

        // Single wildcard
        let path = FieldPath::from_dot_notation("users.*.name");
        assert!(path.has_wildcards());

        // Array wildcard
        let path = FieldPath::from_dot_notation("items[*].value");
        assert!(path.has_wildcards());

        // Wildcard at start
        let path = FieldPath::from_dot_notation("*.name");
        assert!(path.has_wildcards());

        // Wildcard at end
        let path = FieldPath::from_dot_notation("user.*");
        assert!(path.has_wildcards());
    }

    #[test]
    fn test_is_wildcard_component() {
        assert!(FieldPath::is_wildcard_component("*"));
        assert!(FieldPath::is_wildcard_component("[*]"));
        assert!(!FieldPath::is_wildcard_component("name"));
        assert!(!FieldPath::is_wildcard_component("user"));
    }

    #[test]
    fn test_split_at_wildcard() {
        let path = FieldPath::from_dot_notation("users.*.name");
        let (prefix, wildcard, suffix) = path.split_at_wildcard().unwrap();
        
        assert_eq!(prefix, vec!["users"]);
        assert_eq!(wildcard, "*");
        assert_eq!(suffix, vec!["name"]);

        // Path without wildcard
        let path = FieldPath::from_dot_notation("user.name");
        assert!(path.split_at_wildcard().is_none());

        // Wildcard at start
        let path = FieldPath::from_dot_notation("*.name");
        let (prefix, wildcard, suffix) = path.split_at_wildcard().unwrap();
        assert_eq!(prefix, Vec::<String>::new());
        assert_eq!(wildcard, "*");
        assert_eq!(suffix, vec!["name"]);

        // Wildcard at end
        let path = FieldPath::from_dot_notation("user.*");
        let (prefix, wildcard, suffix) = path.split_at_wildcard().unwrap();
        assert_eq!(prefix, vec!["user"]);
        assert_eq!(wildcard, "*");
        assert_eq!(suffix, Vec::<String>::new());
    }

    #[test]
    fn test_wildcard_positions() {
        let path = FieldPath::from_dot_notation("users.*.roles.*");
        let positions = path.wildcard_positions();
        assert_eq!(positions, vec![1, 3]);

        let path = FieldPath::from_dot_notation("user.name");
        let positions = path.wildcard_positions();
        assert_eq!(positions, Vec::<usize>::new());
    }

    #[test]
    fn test_has_nested_wildcards() {
        let path = FieldPath::from_dot_notation("users.*.roles.*");
        assert!(path.has_nested_wildcards());

        let path = FieldPath::from_dot_notation("users.*.name");
        assert!(!path.has_nested_wildcards());

        let path = FieldPath::from_dot_notation("user.name");
        assert!(!path.has_nested_wildcards());
    }
}

#[cfg(test)]
mod record_data_wildcard_tests {
    use super::*;

    #[test]
    fn test_resolve_path_with_wildcards_object_wildcard() {
        let data = RecordData::from_json_str(r#"{
            "users": {
                "alice": {"role": "admin"},
                "bob": {"role": "user"},
                "charlie": {"role": "guest"}
            }
        }"#).unwrap();
        
        // Get all user objects
        let results = data.resolve_path_with_wildcards(&[
            "users".to_string(),
            "*".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_resolve_path_with_wildcards_array_wildcard() {
        let data = RecordData::from_json_str(r#"{
            "users": [
                {"name": "alice", "role": "admin"},
                {"name": "bob", "role": "user"},
                {"name": "charlie", "role": "guest"}
            ]
        }"#).unwrap();
        
        // Get all user objects
        let results = data.resolve_path_with_wildcards(&[
            "users".to_string(),
            "[*]".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_resolve_path_with_wildcards_nested_field() {
        let data = RecordData::from_json_str(r#"{
            "users": [
                {"name": "alice", "role": "admin"},
                {"name": "bob", "role": "user"}
            ]
        }"#).unwrap();
        
        // Get all user roles using wildcard path
        let results = data.resolve_path_with_wildcards(&[
            "users".to_string(),
            "[*]".to_string(),
            "role".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 2);
        
        // Check the actual values
        let roles: Vec<String> = results.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();
        
        assert!(roles.contains(&"admin".to_string()));
        assert!(roles.contains(&"user".to_string()));
    }

    #[test]
    fn test_resolve_path_with_wildcards_object_nested() {
        let data = RecordData::from_json_str(r#"{
            "servers": {
                "web1": {"type": "nginx", "port": 80},
                "web2": {"type": "apache", "port": 8080},
                "db1": {"type": "postgres", "port": 5432}
            }
        }"#).unwrap();
        
        // Get all server types
        let results = data.resolve_path_with_wildcards(&[
            "servers".to_string(),
            "*".to_string(),
            "type".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_resolve_path_without_wildcards() {
        let data = RecordData::from_json_str(r#"{
            "user": {"name": "alice", "role": "admin"}
        }"#).unwrap();
        
        // Regular path without wildcards should still work
        let results = data.resolve_path_with_wildcards(&[
            "user".to_string(),
            "role".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_str(), Some("admin"));
    }

    #[test]
    fn test_resolve_path_nonexistent_field() {
        let data = RecordData::from_json_str(r#"{
            "users": [
                {"name": "alice"}
            ]
        }"#).unwrap();
        
        // Path to nonexistent field returns empty
        let results = data.resolve_path_with_wildcards(&[
            "users".to_string(),
            "[*]".to_string(),
            "role".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_resolve_path_empty_array() {
        let data = RecordData::from_json_str(r#"{
            "users": []
        }"#).unwrap();
        
        let results = data.resolve_path_with_wildcards(&[
            "users".to_string(),
            "[*]".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_resolve_path_mixed_types() {
        let data = RecordData::from_json_str(r#"{
            "data": [
                {"value": 42},
                {"value": "text"},
                {"value": true}
            ]
        }"#).unwrap();
        
        let results = data.resolve_path_with_wildcards(&[
            "data".to_string(),
            "[*]".to_string(),
            "value".to_string(),
        ]).unwrap();
        
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_is_wildcard_component() {
        assert!(RecordData::is_wildcard_component("*"));
        assert!(RecordData::is_wildcard_component("[*]"));
        assert!(!RecordData::is_wildcard_component("name"));
        assert!(!RecordData::is_wildcard_component("users"));
    }

    #[test]
    fn test_parse_array_index() {
        assert_eq!(
            RecordData::parse_array_index("users[0]"),
            Some(("users", "0"))
        );
        assert_eq!(
            RecordData::parse_array_index("items[123]"),
            Some(("items", "123"))
        );
        assert_eq!(RecordData::parse_array_index("name"), None);
        assert_eq!(RecordData::parse_array_index("users[]"), None);
    }
}