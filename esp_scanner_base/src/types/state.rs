use super::common::{DataType, FieldPath, Operation, ResolvedValue, Value};
use serde::{Deserialize, Serialize};

/// State declaration from ICS definition
/// Can be either global (definition-level) or local (CTN-level)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDeclaration {
    pub identifier: String,
    pub fields: Vec<StateField>,
    pub record_checks: Vec<RecordCheck>,
    pub is_global: bool, // true = definition-level, false = CTN-level
}

/// Resolved state with all variable references substituted
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedState {
    pub identifier: String,
    pub resolved_fields: Vec<ResolvedStateField>,
    pub resolved_record_checks: Vec<ResolvedRecordCheck>,
    pub is_global: bool,
}

/// Individual field within a state definition
/// EBNF: state_field ::= field_name space data_type space operation space value_spec (space entity_check)? statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateField {
    /// Field name
    pub name: String,
    /// Field data type
    pub data_type: DataType,
    /// Operation to perform
    pub operation: Operation,
    /// Value to compare against
    pub value: Value,
    /// Optional entity check
    pub entity_check: Option<EntityCheck>,
}

/// Resolved state field with concrete value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedStateField {
    /// Field name
    pub name: String,
    /// Field data type
    pub data_type: DataType,
    /// Operation to perform
    pub operation: Operation,
    /// Resolved value to compare against
    pub value: ResolvedValue,
    /// Optional entity check
    pub entity_check: Option<EntityCheck>,
}

/// Entity check for validation scope (only used in states)
/// EBNF: entity_check ::= "all" | "at_least_one" | "none" | "only_one"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityCheck {
    All,        // all
    AtLeastOne, // at_least_one
    None,       // none
    OnlyOne,    // only_one
}

impl EntityCheck {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "at_least_one" => Some(Self::AtLeastOne),
            "none" => Some(Self::None),
            "only_one" => Some(Self::OnlyOne),

            "All" => Some(Self::All),
            "AtLeastOne" => Some(Self::AtLeastOne),
            "None" => Some(Self::None),
            "OnlyOne" => Some(Self::OnlyOne),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::AtLeastOne => "at_least_one",
            Self::None => "none",
            Self::OnlyOne => "only_one",
        }
    }

    /// Check if this entity check expects entities to satisfy the condition
    pub fn expects_satisfaction(&self) -> bool {
        match self {
            Self::All | Self::AtLeastOne | Self::OnlyOne => true,
            Self::None => false,
        }
    }

    /// Check if this entity check requires all entities to satisfy
    pub fn requires_all_entities(&self) -> bool {
        matches!(self, Self::All)
    }
}

/// Record check for record datatype validation in states
/// EBNF: record_check ::= "record" space data_type? statement_end record_content "record_end" statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordCheck {
    pub data_type: Option<DataType>,
    pub content: RecordContent,
}

/// Resolved record check with concrete data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedRecordCheck {
    pub data_type: Option<DataType>,
    pub content: ResolvedRecordContent,
}

/// Record content types for validation in states
/// EBNF: record_content ::= direct_operation | nested_fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RecordContent {
    /// Direct operation on entire record
    /// EBNF: direct_operation ::= operation space value_spec statement_end
    Direct { operation: Operation, value: Value },
    /// Nested field operations
    /// EBNF: nested_fields ::= record_field+
    Nested { fields: Vec<RecordField> },
}

/// Resolved record content with concrete values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolvedRecordContent {
    /// Direct operation with resolved value
    Direct {
        operation: Operation,
        value: ResolvedValue,
    },
    /// Nested fields with resolved values
    Nested { fields: Vec<ResolvedRecordField> },
}

/// Record field specification for nested record validation in states
/// EBNF: record_field ::= "field" space field_path space data_type space operation space value_spec (space entity_check)? statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordField {
    pub path: FieldPath,
    pub data_type: DataType,
    pub operation: Operation,
    pub value: Value,
    pub entity_check: Option<EntityCheck>,
}

/// Resolved record field with concrete value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedRecordField {
    pub path: FieldPath,
    pub data_type: DataType,
    pub operation: Operation,
    pub value: ResolvedValue,
    pub entity_check: Option<EntityCheck>,
}

/// State reference for referencing global states
/// EBNF: state_reference ::= "STATE_REF" space state_identifier statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateRef {
    /// Referenced state ID (must be global)
    pub state_id: String,
}

impl StateRef {
    /// Create a new state reference
    pub fn new(state_id: impl Into<String>) -> Self {
        Self {
            state_id: state_id.into(),
        }
    }
}

// Utility implementations
impl StateDeclaration {
    /// Check if this state has any variable references in its fields or record checks
    pub fn has_variable_references(&self) -> bool {
        self.fields
            .iter()
            .any(|field| field.has_variable_references())
            || self
                .record_checks
                .iter()
                .any(|record| record.has_variable_references())
    }

    /// Get all variable references used in this state
    pub fn get_variable_references(&self) -> Vec<String> {
        let mut refs = Vec::new();

        // Collect from fields
        for field in &self.fields {
            refs.extend(field.get_variable_references());
        }

        // Collect from record checks
        for record in &self.record_checks {
            refs.extend(record.get_variable_references());
        }

        refs.sort();
        refs.dedup();
        refs
    }

    /// Check if this is an empty state (no fields or record checks)
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.record_checks.is_empty()
    }

    /// Get total element count (for validation)
    pub fn element_count(&self) -> usize {
        self.fields.len() + self.record_checks.len()
    }

    /// Get all field names for this state
    pub fn get_field_names(&self) -> Vec<String> {
        self.fields.iter().map(|field| field.name.clone()).collect()
    }

    /// Check if this state has any entity checks
    pub fn has_entity_checks(&self) -> bool {
        self.fields.iter().any(|field| field.entity_check.is_some())
            || self
                .record_checks
                .iter()
                .any(|record| record.has_entity_checks())
    }
}

impl StateField {
    /// Check if this field has variable references
    pub fn has_variable_references(&self) -> bool {
        self.value.has_variable_reference()
    }

    /// Get variable references from this field
    pub fn get_variable_references(&self) -> Vec<String> {
        if let Some(var_name) = self.value.get_variable_name() {
            vec![var_name.to_string()]
        } else {
            Vec::new()
        }
    }

    /// Check if this field has an entity check
    pub fn has_entity_check(&self) -> bool {
        self.entity_check.is_some()
    }
}

impl RecordCheck {
    /// Check if this record check has variable references
    pub fn has_variable_references(&self) -> bool {
        self.content.has_variable_references()
    }

    /// Get variable references from this record check
    pub fn get_variable_references(&self) -> Vec<String> {
        self.content.get_variable_references()
    }

    /// Check if this record check has entity checks
    pub fn has_entity_checks(&self) -> bool {
        self.content.has_entity_checks()
    }
}

impl RecordContent {
    /// Check if this record content has variable references
    pub fn has_variable_references(&self) -> bool {
        match self {
            RecordContent::Direct { value, .. } => value.has_variable_reference(),
            RecordContent::Nested { fields } => fields
                .iter()
                .any(|field| field.value.has_variable_reference()),
        }
    }

    /// Get variable references from this record content
    pub fn get_variable_references(&self) -> Vec<String> {
        match self {
            RecordContent::Direct { value, .. } => {
                if let Some(var_name) = value.get_variable_name() {
                    vec![var_name.to_string()]
                } else {
                    Vec::new()
                }
            }
            RecordContent::Nested { fields } => {
                let mut refs = Vec::new();
                for field in fields {
                    if let Some(var_name) = field.value.get_variable_name() {
                        refs.push(var_name.to_string());
                    }
                }
                refs.sort();
                refs.dedup();
                refs
            }
        }
    }

    /// Check if this record content has entity checks
    pub fn has_entity_checks(&self) -> bool {
        match self {
            RecordContent::Direct { .. } => false, // Direct operations don't have entity checks
            RecordContent::Nested { fields } => {
                fields.iter().any(|field| field.entity_check.is_some())
            }
        }
    }
}

impl RecordField {
    /// Check if this record field has variable references
    pub fn has_variable_references(&self) -> bool {
        self.value.has_variable_reference()
    }

    /// Get variable references from this record field
    pub fn get_variable_references(&self) -> Vec<String> {
        if let Some(var_name) = self.value.get_variable_name() {
            vec![var_name.to_string()]
        } else {
            Vec::new()
        }
    }

    /// Check if this record field has an entity check
    pub fn has_entity_check(&self) -> bool {
        self.entity_check.is_some()
    }
}

// Display implementations
impl std::fmt::Display for EntityCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for StateField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(entity_check) = &self.entity_check {
            write!(
                f,
                "{} {} {} {} {}",
                self.name,
                self.data_type,
                self.operation,
                match &self.value {
                    Value::String(s) => format!("`{}`", s),
                    Value::Variable(v) => format!("VAR {}", v),
                    Value::Integer(i) => i.to_string(),
                    Value::Float(fl) => fl.to_string(),
                    Value::Boolean(b) => b.to_string(),
                },
                entity_check
            )
        } else {
            write!(
                f,
                "{} {} {} {}",
                self.name,
                self.data_type,
                self.operation,
                match &self.value {
                    Value::String(s) => format!("`{}`", s),
                    Value::Variable(v) => format!("VAR {}", v),
                    Value::Integer(i) => i.to_string(),
                    Value::Float(fl) => fl.to_string(),
                    Value::Boolean(b) => b.to_string(),
                }
            )
        }
    }
}
