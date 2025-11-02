use super::filter::{FilterSpec, ResolvedFilterSpec};
use super::object::ObjectDeclaration;
use serde::{Deserialize, Serialize};

/// Set operation declaration from ICS definition (all sets are global)
/// EBNF: set_block ::= "SET" space set_identifier space set_operation statement_end set_content "SET_END" statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetOperation {
    pub set_id: String,
    pub operation: SetOperationType,
    pub operands: Vec<SetOperand>,
    pub filter: Option<FilterSpec>,
}

/// Resolved set operation with validated operands and filters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSetOperation {
    pub set_id: String,
    pub operation: SetOperationType,
    pub resolved_operands: Vec<ResolvedSetOperand>,
    pub resolved_filter: Option<ResolvedFilterSpec>,
}

/// Set operation types with operand count validation
/// EBNF: set_operation ::= "union" | "intersection" | "complement"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SetOperationType {
    Union,        // union (1+ operands)
    Intersection, // intersection (2+ operands)
    Complement,   // complement (exactly 2 operands)
}

impl SetOperationType {
    /// Parse set operation from string (case-sensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "union" => Some(Self::Union),
            "intersection" => Some(Self::Intersection),
            "complement" => Some(Self::Complement),
            "Union" => Some(Self::Union),
            "Intersection" => Some(Self::Intersection),
            "Complement" => Some(Self::Complement),
            _ => None,
        }
    }

    /// Get the operation as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Union => "union",
            Self::Intersection => "intersection",
            Self::Complement => "complement",
        }
    }

    /// Validate operand count for this operation type
    pub fn validate_operand_count(&self, count: usize) -> Result<(), String> {
        match self {
            Self::Union => {
                if count == 0 {
                    Err("Union operation requires at least 1 operand".to_string())
                } else {
                    Ok(())
                }
            }
            Self::Intersection => {
                if count < 2 {
                    Err("Intersection operation requires at least 2 operands".to_string())
                } else {
                    Ok(())
                }
            }
            Self::Complement => {
                if count != 2 {
                    Err("Complement operation requires exactly 2 operands".to_string())
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Get minimum required operands for this operation
    pub fn min_operands(&self) -> usize {
        match self {
            Self::Union => 1,
            Self::Intersection => 2,
            Self::Complement => 2,
        }
    }

    /// Get maximum allowed operands for this operation (None = unlimited)
    pub fn max_operands(&self) -> Option<usize> {
        match self {
            Self::Union => None,
            Self::Intersection => None,
            Self::Complement => Some(2),
        }
    }
}

/// Set operand types
/// EBNF: operand_type ::= object_spec | object_reference | set_reference
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetOperand {
    /// Reference to a global object (OBJECT_REF obj_id)
    ObjectRef(String),

    /// Reference to another set (SET_REF set_id)
    SetRef(String),

    /// Inline object definition within the set
    /// EBNF: object_spec ::= "OBJECT" statement_end object_content "OBJECT_END"
    InlineObject(ObjectDeclaration),
}

/// Resolved set operand with validated references
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolvedSetOperand {
    /// Validated object reference
    ObjectRef(String),

    /// Validated set reference  
    SetRef(String),

    /// Resolved inline object
    InlineObject(super::object::ResolvedObject),
}

/// Set reference for referencing global sets
/// EBNF: set_reference ::= "SET_REF" space set_identifier statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetRef {
    /// Referenced set ID (must be global)
    pub set_id: String,
}

impl SetRef {
    /// Create a new set reference
    pub fn new(set_id: impl Into<String>) -> Self {
        Self {
            set_id: set_id.into(),
        }
    }
}

// Utility implementations
impl SetOperation {
    /// Create a new set operation
    pub fn new(
        set_id: String,
        operation: SetOperationType,
        operands: Vec<SetOperand>,
        filter: Option<FilterSpec>,
    ) -> Result<Self, String> {
        // Validate operand count
        operation.validate_operand_count(operands.len())?;

        Ok(Self {
            set_id,
            operation,
            operands,
            filter,
        })
    }

    /// Check if this set operation has a filter
    pub fn has_filter(&self) -> bool {
        self.filter.is_some()
    }

    /// Get operand count
    pub fn operand_count(&self) -> usize {
        self.operands.len()
    }

    /// Validate that this set operation is properly formed
    pub fn validate(&self) -> Result<(), String> {
        // Check operand count
        self.operation.validate_operand_count(self.operands.len())?;

        // Validate filter if present
        if let Some(filter) = &self.filter {
            filter.validate()?;
        }

        // Check for empty operands
        if self.operands.is_empty() {
            return Err("Set operation cannot have zero operands".to_string());
        }

        Ok(())
    }

    /// Get all object references from operands
    pub fn get_object_references(&self) -> Vec<String> {
        self.operands
            .iter()
            .filter_map(|operand| match operand {
                SetOperand::ObjectRef(obj_id) => Some(obj_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get all set references from operands
    pub fn get_set_references(&self) -> Vec<String> {
        self.operands
            .iter()
            .filter_map(|operand| match operand {
                SetOperand::SetRef(set_id) => Some(set_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Get all state dependencies from filters
    pub fn get_filter_state_dependencies(&self) -> Vec<String> {
        if let Some(filter) = &self.filter {
            filter.get_state_dependencies()
        } else {
            Vec::new()
        }
    }

    /// Check if this set has any variable references (from inline objects)
    pub fn has_variable_references(&self) -> bool {
        self.operands.iter().any(|operand| match operand {
            SetOperand::InlineObject(obj) => obj.has_variable_references(),
            _ => false,
        })
    }

    /// Get all variable references from inline objects
    pub fn get_variable_references(&self) -> Vec<String> {
        let mut refs = Vec::new();
        for operand in &self.operands {
            if let SetOperand::InlineObject(obj) = operand {
                refs.extend(obj.get_variable_references());
            }
        }
        refs.sort();
        refs.dedup();
        refs
    }

    /// Check if this set references external symbols (objects, sets, states via filters)
    pub fn has_external_references(&self) -> bool {
        self.operands
            .iter()
            .any(|operand| operand.has_external_references())
            || self.has_filter()
    }
}

impl SetOperand {
    /// Check if this operand has external references
    pub fn has_external_references(&self) -> bool {
        match self {
            SetOperand::ObjectRef(_) => true,
            SetOperand::SetRef(_) => true,
            SetOperand::InlineObject(obj) => obj.has_external_references(),
        }
    }

    /// Get the operand type name for debugging
    pub fn operand_type_name(&self) -> &'static str {
        match self {
            SetOperand::ObjectRef(_) => "ObjectRef",
            SetOperand::SetRef(_) => "SetRef",
            SetOperand::InlineObject(_) => "InlineObject",
        }
    }

    /// Check if this operand is a reference (not inline)
    pub fn is_reference(&self) -> bool {
        matches!(self, SetOperand::ObjectRef(_) | SetOperand::SetRef(_))
    }

    /// Check if this operand is an inline definition
    pub fn is_inline(&self) -> bool {
        matches!(self, SetOperand::InlineObject(_))
    }
}

// Display implementations
impl std::fmt::Display for SetOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for SetOperand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetOperand::ObjectRef(obj_id) => write!(f, "OBJECT_REF {}", obj_id),
            SetOperand::SetRef(set_id) => write!(f, "SET_REF {}", set_id),
            SetOperand::InlineObject(obj) => write!(f, "OBJECT {} (inline)", obj.identifier),
        }
    }
}

impl std::fmt::Display for SetOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SET {} {} ({} operands)",
            self.set_id,
            self.operation,
            self.operands.len()
        )?;
        if self.has_filter() {
            write!(f, " with filter")?;
        }
        Ok(())
    }
}
