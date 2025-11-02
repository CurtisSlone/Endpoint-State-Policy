use super::common::{DataType, ResolvedValue, Value};
use serde::{Deserialize, Serialize};

/// Resolved variable with concrete value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedVariable {
    pub identifier: String,
    pub data_type: DataType,
    pub value: ResolvedValue,
}

/// Variable declaration from ICS definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDeclaration {
    pub name: String,
    pub data_type: DataType,
    pub initial_value: Option<Value>,
}

impl ResolvedVariable {
    /// Create a new resolved variable
    pub fn new(identifier: String, data_type: DataType, value: ResolvedValue) -> Self {
        Self {
            identifier,
            data_type,
            value,
        }
    }

    /// Check if the resolved value matches the declared data type
    pub fn is_type_consistent(&self) -> bool {
        self.data_type.matches_resolved_value(&self.value)
    }

    /// Get the variable name
    pub fn name(&self) -> &str {
        &self.identifier
    }

    /// Get the resolved value
    pub fn resolved_value(&self) -> &ResolvedValue {
        &self.value
    }

    /// Check if this was originally a computed variable
    pub fn was_computed(&self) -> bool {
        // This information should come from the original declaration
        // We'll need to track this during resolution
        false // Placeholder - will be set during resolution
    }

    /// Create a resolved variable from a computed variable (populated by RUN operation)
    pub fn from_computed(identifier: String, data_type: DataType, value: ResolvedValue) -> Self {
        // Could add a flag here to track that it was computed
        Self {
            identifier,
            data_type,
            value,
        }
    }
}

impl VariableDeclaration {
    /// Create a new variable declaration
    pub fn new(name: String, data_type: DataType, initial_value: Option<Value>) -> Self {
        Self {
            name,
            data_type,
            initial_value,
        }
    }

    /// Check if this variable has an initial value
    pub fn has_initial_value(&self) -> bool {
        self.initial_value.is_some()
    }

    pub fn is_computed(&self) -> bool {
        self.initial_value.is_none()
    }

    /// Check if this variable references another variable in its initial value
    pub fn has_variable_reference(&self) -> bool {
        match &self.initial_value {
            Some(Value::Variable(_)) => true,
            _ => false,
        }
    }

    /// Get the referenced variable name if this variable references another variable
    pub fn get_variable_reference(&self) -> Option<&str> {
        match &self.initial_value {
            Some(Value::Variable(var_name)) => Some(var_name),
            _ => None,
        }
    }

    /// Check if this variable is initialized with a literal value (not a reference)
    pub fn has_literal_initial_value(&self) -> bool {
        match &self.initial_value {
            Some(Value::Variable(_)) => false, // Reference, not literal
            Some(_) => true,                   // Literal value
            None => false,                     // No initial value
        }
    }

    /// Check if this variable is initialized with a variable reference
    pub fn has_variable_reference_initialization(&self) -> bool {
        matches!(self.initial_value, Some(Value::Variable(_)))
    }

    /// Get the initialization dependency (variable name this depends on)
    pub fn get_initialization_dependency(&self) -> Option<&str> {
        match &self.initial_value {
            Some(Value::Variable(var_name)) => Some(var_name),
            _ => None,
        }
    }
}

// Display implementations
impl std::fmt::Display for ResolvedVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} = {:?}",
            self.identifier, self.data_type, self.value
        )
    }
}

impl std::fmt::Display for VariableDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.initial_value {
            Some(value) => write!(f, "VAR {} {} {:?}", self.name, self.data_type, value),
            None => write!(f, "VAR {} {}", self.name, self.data_type),
        }
    }
}
