use super::common::{DataType, Value};
use crate::types::ResolutionContext;
use serde::{Deserialize, Serialize};

/// Runtime operation declaration from ICS definition
/// EBNF: run_block ::= "RUN" space variable_name space operation_type statement_end run_parameters "RUN_END" statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeOperation {
    /// Variable to store result in
    pub target_variable: String,
    /// Type of operation
    pub operation_type: RuntimeOperationType,
    /// Operation parameters
    pub parameters: Vec<RunParameter>,
}

/// Runtime operation types
/// EBNF: operation_type ::= "CONCAT" | "SPLIT" | "SUBSTRING" | "REGEX_CAPTURE" | "ARITHMETIC" | "COUNT" | "UNIQUE" | "END" | "MERGE" | "EXTRACT"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeOperationType {
    Concat,       // CONCAT
    Split,        // SPLIT
    Substring,    // SUBSTRING
    RegexCapture, // REGEX_CAPTURE
    Arithmetic,   // ARITHMETIC
    Count,        // COUNT
    Unique,       // UNIQUE
    End,          // END
    Merge,        // MERGE
    Extract,      // EXTRACT
}

impl RuntimeOperationType {
    /// Parse runtime operation from string (case-sensitive, uppercase)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Concat" => Some(Self::Concat),
            "Split" => Some(Self::Split),
            "Substring" => Some(Self::Substring),
            "RegexCapture" => Some(Self::RegexCapture),
            "Arithmetic" => Some(Self::Arithmetic),
            "Count" => Some(Self::Count),
            "Unique" => Some(Self::Unique),
            "End" => Some(Self::End),
            "Merge" => Some(Self::Merge),
            "Extract" => Some(Self::Extract),
            _ => None,
        }
    }

    /// Get the operation as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Concat => "Concat",
            Self::Split => "Split",
            Self::Substring => "SUBSTRING",
            Self::RegexCapture => "RegexCapture",
            Self::Arithmetic => "Arithmetic",
            Self::Count => "Count",
            Self::Unique => "Unique",
            Self::End => "End",
            Self::Merge => "Merge",
            Self::Extract => "Extract",
        }
    }

    /// Returns the expected output type for this operation
    pub fn output_type(&self) -> DataType {
        match self {
            Self::Concat | Self::Split | Self::Substring | Self::RegexCapture => DataType::String,
            Self::Count => DataType::Int,
            Self::Arithmetic => DataType::Int, // Could be Float depending on inputs
            Self::Extract => DataType::String, // Varies by extracted field
            Self::Unique | Self::Merge | Self::End => DataType::String, // Depends on input type
        }
    }

    /// Returns required parameter types for validation
    pub fn required_parameters(&self) -> Vec<&'static str> {
        match self {
            Self::Concat => vec!["literal", "variable", "object_extraction"],
            Self::Split => vec!["delimiter"],
            Self::Substring => vec!["start", "length"],
            Self::RegexCapture => vec!["pattern"],
            Self::Arithmetic => vec!["arithmetic_op"],
            Self::Extract => vec!["object_extraction"],
            _ => vec![],
        }
    }

    /// Check if this operation can accept multiple input parameters
    pub fn accepts_multiple_inputs(&self) -> bool {
        matches!(self, Self::Concat | Self::Arithmetic | Self::Merge)
    }

    /// Check if this operation requires at least one parameter
    pub fn requires_parameters(&self) -> bool {
        !matches!(self, Self::Count | Self::Unique | Self::End)
    }
}

/// Runtime operation parameters from EBNF (run_parameter)
/// EBNF: run_parameter ::= parameter_line statement_end
/// EBNF: parameter_line ::= literal_component | variable_component | object_component | pattern_spec | delimiter_spec | character_spec | start_position | length_value | arithmetic_op
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RunParameter {
    /// Literal value (literal space value)
    /// EBNF: literal_component ::= "literal" space (backtick_string | integer_value)
    Literal(Value),

    /// Variable reference (VAR space variable_name)
    /// EBNF: variable_component ::= "VAR" space variable_name
    Variable(String),

    /// Object field extraction (OBJ space object_id space field)
    /// EBNF: object_component ::= object_extraction
    /// EBNF: object_extraction ::= "OBJ" space object_identifier space item_field
    ObjectExtraction { object_id: String, field: String },

    /// Pattern specification (pattern space backtick_string)
    /// EBNF: pattern_spec ::= "pattern" space backtick_string
    Pattern(String),

    /// Delimiter specification (delimiter space backtick_string)
    /// EBNF: delimiter_spec ::= "delimiter" space backtick_string
    Delimiter(String),

    /// Character specification (character space backtick_string)
    /// EBNF: character_spec ::= "character" space backtick_string
    Character(String),

    /// Start position (start space integer_value)
    /// EBNF: start_position ::= "start" space integer_value
    StartPosition(i64),

    /// Length value (length space integer_value)
    /// EBNF: length_value ::= "length" space integer_value
    Length(i64),

    /// Arithmetic operation (arithmetic_op)
    /// EBNF: arithmetic_op ::= "+" | "*" | "-" | "/" | "%"
    ArithmeticOp {
        operator: ArithmeticOperator,
        operand: Value,
    },
}

/// Arithmetic operators for RUN parameters
/// EBNF: arithmetic_op ::= "+" | "*" | "-" | "/" | "%"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArithmeticOperator {
    Add,      // +
    Multiply, // *
    Subtract, // -
    Divide,   // /
    Modulus,  // %
}

impl ArithmeticOperator {
    /// Parse arithmetic operator from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Add" => Some(Self::Add),
            "Multiply" => Some(Self::Multiply),
            "Subtract" => Some(Self::Subtract),
            "Divide" => Some(Self::Divide),
            "Modulus" => Some(Self::Modulus),
            _ => None,
        }
    }

    /// Get the operator as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Multiply => "*",
            Self::Subtract => "-",
            Self::Divide => "/",
            Self::Modulus => "%",
        }
    }

    /// Check if this is a commutative operation
    pub fn is_commutative(&self) -> bool {
        matches!(self, Self::Add | Self::Multiply)
    }

    /// Check if this operation can result in division by zero
    pub fn can_divide_by_zero(&self) -> bool {
        matches!(self, Self::Divide | Self::Modulus)
    }
}

// Utility implementations
impl RuntimeOperation {
    /// Create a new runtime operation
    pub fn new(
        target_variable: String,
        operation_type: RuntimeOperationType,
        parameters: Vec<RunParameter>,
    ) -> Self {
        Self {
            target_variable,
            operation_type,
            parameters,
        }
    }

    /// Check if this operation has any variable references in its parameters
    pub fn has_variable_references(&self) -> bool {
        self.parameters
            .iter()
            .any(|param| param.has_variable_references())
    }

    /// Get all variable references from parameters
    pub fn get_variable_references(&self) -> Vec<String> {
        let mut refs = Vec::new();
        for param in &self.parameters {
            refs.extend(param.get_variable_references());
        }
        refs.sort();
        refs.dedup();
        refs
    }

    /// Get all object dependencies from parameters
    pub fn get_object_dependencies(&self) -> Vec<String> {
        self.parameters
            .iter()
            .filter_map(|param| match param {
                RunParameter::ObjectExtraction { object_id, .. } => Some(object_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get parameter count
    pub fn parameter_count(&self) -> usize {
        self.parameters.len()
    }

    /// Validate that this operation has appropriate parameters for its type
    pub fn validate(&self) -> Result<(), String> {
        // Check if operation requires parameters
        if self.operation_type.requires_parameters() && self.parameters.is_empty() {
            return Err(format!(
                "Operation {} requires at least one parameter",
                self.operation_type.as_str()
            ));
        }

        // Validate parameter types for specific operations
        match self.operation_type {
            RuntimeOperationType::Split => {
                if !self.has_delimiter_parameter() {
                    return Err("SPLIT operation requires a delimiter parameter".to_string());
                }
            }
            RuntimeOperationType::Substring => {
                if !self.has_start_parameter() || !self.has_length_parameter() {
                    return Err(
                        "SUBSTRING operation requires start and length parameters".to_string()
                    );
                }
            }
            RuntimeOperationType::RegexCapture => {
                if !self.has_pattern_parameter() {
                    return Err("REGEX_CAPTURE operation requires a pattern parameter".to_string());
                }
            }
            RuntimeOperationType::Extract => {
                if !self.has_object_extraction_parameter() {
                    return Err(
                        "EXTRACT operation requires an object extraction parameter".to_string()
                    );
                }
            }
            _ => {} // Other operations have flexible parameter requirements
        }

        Ok(())
    }

    /// Check if this operation has a delimiter parameter
    pub fn has_delimiter_parameter(&self) -> bool {
        self.parameters
            .iter()
            .any(|param| matches!(param, RunParameter::Delimiter(_)))
    }

    /// Check if this operation has a pattern parameter
    pub fn has_pattern_parameter(&self) -> bool {
        self.parameters
            .iter()
            .any(|param| matches!(param, RunParameter::Pattern(_)))
    }

    /// Check if this operation has start/length parameters
    pub fn has_start_parameter(&self) -> bool {
        self.parameters
            .iter()
            .any(|param| matches!(param, RunParameter::StartPosition(_)))
    }

    pub fn has_length_parameter(&self) -> bool {
        self.parameters
            .iter()
            .any(|param| matches!(param, RunParameter::Length(_)))
    }

    /// Check if this operation has object extraction parameters
    pub fn has_object_extraction_parameter(&self) -> bool {
        self.parameters
            .iter()
            .any(|param| matches!(param, RunParameter::ObjectExtraction { .. }))
    }

    /// Categorize this operation for execution timing
    pub fn categorize(&self, context: &ResolutionContext) -> OperationCategory {
        match self.operation_type {
            // Always scan-time (require collected data or objects)
            RuntimeOperationType::Extract
            | RuntimeOperationType::Unique
            | RuntimeOperationType::Merge
            | RuntimeOperationType::End => OperationCategory::ScanTime,

            // Resolution-time (work with variables/literals only)
            RuntimeOperationType::Concat
            | RuntimeOperationType::Arithmetic
            | RuntimeOperationType::Split
            | RuntimeOperationType::Substring => {
                if self.is_resolvable_now(context) {
                    OperationCategory::ResolutionTime
                } else {
                    OperationCategory::ScanTime
                }
            }

            // Context-dependent
            RuntimeOperationType::RegexCapture | RuntimeOperationType::Count => {
                if self.has_object_dependency() {
                    OperationCategory::ScanTime
                } else if self.is_resolvable_now(context) {
                    OperationCategory::ResolutionTime
                } else {
                    OperationCategory::ScanTime
                }
            }
        }
    }

    /// Check if all parameters can be resolved with current context
    pub fn is_resolvable_now(
        &self,
        context: &crate::types::resolution_context::ResolutionContext,
    ) -> bool {
        self.parameters
            .iter()
            .all(|param| Self::param_is_resolvable(param, context))
    }

    /// Check if a single parameter is resolvable
    fn param_is_resolvable(
        param: &RunParameter,
        context: &crate::types::resolution_context::ResolutionContext,
    ) -> bool {
        match param {
            RunParameter::Literal(_) => true,
            RunParameter::Variable(var_name) => context.resolved_variables.contains_key(var_name),
            RunParameter::ObjectExtraction { .. } => false,
            RunParameter::Pattern(_)
            | RunParameter::Delimiter(_)
            | RunParameter::Character(_)
            | RunParameter::StartPosition(_)
            | RunParameter::Length(_) => true,
            RunParameter::ArithmeticOp { operand, .. } => {
                match operand {
                    crate::types::common::Value::Variable(var_name) => {
                        context.resolved_variables.contains_key(var_name)
                    }
                    _ => true, // Literals are always resolvable
                }
            }
        }
    }

    /// Check if this operation depends on collected object data
    pub fn has_object_dependency(&self) -> bool {
        self.parameters
            .iter()
            .any(|p| matches!(p, RunParameter::ObjectExtraction { .. }))
    }

    /// Extract object ID if this operation has ObjectExtraction parameter
    pub fn extract_object_id(&self) -> Option<String> {
        self.parameters.iter().find_map(|p| {
            if let RunParameter::ObjectExtraction { object_id, .. } = p {
                Some(object_id.clone())
            } else {
                None
            }
        })
    }
}

// Category for runtime operation execution timing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationCategory {
    /// Can be executed during resolution phase (all inputs available)
    ResolutionTime,
    /// Must be deferred to scan phase (requires collected data)
    ScanTime,
}

impl RunParameter {
    /// Check if this parameter contains variable references
    pub fn has_variable_references(&self) -> bool {
        match self {
            RunParameter::Literal(value) => value.has_variable_reference(),
            RunParameter::Variable(_) => true,
            RunParameter::ObjectExtraction { .. } => false, // Object extraction doesn't contain variable refs
            RunParameter::Pattern(_) => false,
            RunParameter::Delimiter(_) => false,
            RunParameter::Character(_) => false,
            RunParameter::StartPosition(_) => false,
            RunParameter::Length(_) => false,
            RunParameter::ArithmeticOp { operand, .. } => operand.has_variable_reference(),
        }
    }

    /// Get variable references from this parameter
    pub fn get_variable_references(&self) -> Vec<String> {
        match self {
            RunParameter::Literal(value) => {
                if let Some(var_name) = value.get_variable_name() {
                    vec![var_name.to_string()]
                } else {
                    Vec::new()
                }
            }
            RunParameter::Variable(var_name) => vec![var_name.clone()],
            RunParameter::ObjectExtraction { .. } => Vec::new(),
            RunParameter::Pattern(_) => Vec::new(),
            RunParameter::Delimiter(_) => Vec::new(),
            RunParameter::Character(_) => Vec::new(),
            RunParameter::StartPosition(_) => Vec::new(),
            RunParameter::Length(_) => Vec::new(),
            RunParameter::ArithmeticOp { operand, .. } => {
                if let Some(var_name) = operand.get_variable_name() {
                    vec![var_name.to_string()]
                } else {
                    Vec::new()
                }
            }
        }
    }

    /// Get the parameter type name for debugging
    pub fn parameter_type_name(&self) -> &'static str {
        match self {
            RunParameter::Literal(_) => "Literal",
            RunParameter::Variable(_) => "Variable",
            RunParameter::ObjectExtraction { .. } => "ObjectExtraction",
            RunParameter::Pattern(_) => "Pattern",
            RunParameter::Delimiter(_) => "Delimiter",
            RunParameter::Character(_) => "Character",
            RunParameter::StartPosition(_) => "StartPosition",
            RunParameter::Length(_) => "Length",
            RunParameter::ArithmeticOp { .. } => "ArithmeticOp",
        }
    }
}

// Display implementations
impl std::fmt::Display for RuntimeOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for ArithmeticOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for RunParameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunParameter::Literal(value) => {
                write!(
                    f,
                    "literal {}",
                    match value {
                        Value::String(s) => format!("`{}`", s),
                        Value::Integer(i) => i.to_string(),
                        Value::Float(fl) => fl.to_string(),
                        Value::Boolean(b) => b.to_string(),
                        Value::Variable(v) => format!("VAR {}", v),
                    }
                )
            }
            RunParameter::Variable(name) => write!(f, "VAR {}", name),
            RunParameter::ObjectExtraction { object_id, field } => {
                write!(f, "OBJ {} {}", object_id, field)
            }
            RunParameter::Pattern(pattern) => write!(f, "pattern `{}`", pattern),
            RunParameter::Delimiter(delimiter) => write!(f, "delimiter `{}`", delimiter),
            RunParameter::Character(character) => write!(f, "character `{}`", character),
            RunParameter::StartPosition(pos) => write!(f, "start {}", pos),
            RunParameter::Length(len) => write!(f, "length {}", len),
            RunParameter::ArithmeticOp { operator, operand } => {
                write!(
                    f,
                    "{} {}",
                    operator,
                    match operand {
                        Value::String(s) => format!("`{}`", s),
                        Value::Integer(i) => i.to_string(),
                        Value::Float(fl) => fl.to_string(),
                        Value::Boolean(b) => b.to_string(),
                        Value::Variable(v) => format!("VAR {}", v),
                    }
                )
            }
        }
    }
}

impl std::fmt::Display for RuntimeOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RUN {} {} ({} parameters)",
            self.target_variable,
            self.operation_type,
            self.parameters.len()
        )
    }
}
