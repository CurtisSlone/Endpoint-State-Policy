use super::common::{DataType, RecordData, ResolvedValue, Value};
use super::filter::{FilterSpec, ResolvedFilterSpec};
use serde::{Deserialize, Serialize};

/// Object declaration from ICS definition
/// Can be either global (definition-level) or local (CTN-level)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectDeclaration {
    pub identifier: String,
    pub elements: Vec<ObjectElement>,
    pub is_global: bool, // true = definition-level, false = CTN-level
}

/// Resolved object with all variable references substituted
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedObject {
    pub identifier: String,
    pub resolved_elements: Vec<ResolvedObjectElement>,
    pub is_global: bool,
}

/// Individual elements within an object definition
/// EBNF: object_element ::= module_element | parameter_element | select_element | behavior_element | filter_spec | set_reference | object_field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectElement {
    /// Module specification for platform-specific modules (PowerShell, WMI, etc.)
    /// EBNF: module_element ::= module_field space backtick_string statement_end
    Module {
        field: ModuleField, // module_name, verb, noun, module_id, module_version
        value: String,      // Always string literal in ICS
    },

    /// Parameters block with data type and nested fields
    /// EBNF: parameter_element ::= "parameters" space data_type statement_end parameter_fields? "parameters_end" statement_end
    Parameter {
        data_type: DataType,
        fields: Vec<(String, String)>, // (key, value) pairs - values are always strings in JSON
    },

    /// Select block with data type and nested fields  
    /// EBNF: select_element ::= "select" space data_type statement_end select_fields? "select_end" statement_end
    Select {
        data_type: DataType,
        fields: Vec<(String, String)>, // (key, value) pairs - values are always strings in JSON
    },

    /// Behavior specification with array of identifiers
    /// EBNF: behavior_element ::= "behavior" space behavior_value+ statement_end
    Behavior {
        values: Vec<String>, // Behavior identifiers/flags
    },

    /// Filter specification (references global states only)
    /// EBNF: filter_spec ::= "FILTER" space filter_action? statement_end filter_references "FILTER_END" statement_end
    Filter(FilterSpec),

    /// Set reference (references global sets only)
    /// EBNF: set_reference ::= "SET_REF" space set_identifier statement_end
    SetRef { set_id: String },

    /// Simple field with name-value pair
    /// EBNF: object_field ::= field_name space field_value statement_end
    Field {
        name: String,
        value: Value, // Can be literal or Variable reference
    },
}

/// Resolved object element with all variables substituted
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResolvedObjectElement {
    /// Module specification (unchanged during resolution)
    Module { field: ModuleField, value: String },

    /// Resolved parameters as structured data
    Parameter {
        data_type: DataType,
        data: RecordData, // Converted from field pairs to structured data
    },

    /// Resolved select as structured data
    Select {
        data_type: DataType,
        data: RecordData, // Converted from field pairs to structured data
    },

    /// Behavior specification (unchanged during resolution)
    Behavior { values: Vec<String> },

    /// Resolved filter with state validation
    Filter(ResolvedFilterSpec),

    /// Set reference (validated during resolution)
    SetRef { set_id: String },

    /// Resolved field with concrete value
    Field { name: String, value: ResolvedValue },
}

/// Module field types for platform-specific operations
/// EBNF: module_field ::= "module_name" | "verb" | "noun" | "module_id" | "module_version"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModuleField {
    ModuleName,    // module_name
    Verb,          // verb
    Noun,          // noun
    ModuleId,      // module_id
    ModuleVersion, // module_version
}

impl ModuleField {
    /// Parse module field from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "module_name" => Some(Self::ModuleName),
            "verb" => Some(Self::Verb),
            "noun" => Some(Self::Noun),
            "module_id" => Some(Self::ModuleId),
            "module_version" => Some(Self::ModuleVersion),
            _ => None,
        }
    }

    /// Get the field as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ModuleName => "module_name",
            Self::Verb => "verb",
            Self::Noun => "noun",
            Self::ModuleId => "module_id",
            Self::ModuleVersion => "module_version",
        }
    }
}

/// Object reference for referencing global objects
/// EBNF: object_reference ::= "OBJECT_REF" space object_identifier statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectRef {
    /// Referenced object ID (must be global)
    pub object_id: String,
}

impl ObjectRef {
    /// Create a new object reference
    pub fn new(object_id: impl Into<String>) -> Self {
        Self {
            object_id: object_id.into(),
        }
    }
}

/// Set reference for referencing global sets within objects
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
impl ObjectDeclaration {
    /// Check if this object has any variable references in its elements
    pub fn has_variable_references(&self) -> bool {
        self.elements
            .iter()
            .any(|element| element.has_variable_references())
    }

    /// Get all variable references used in this object
    pub fn get_variable_references(&self) -> Vec<String> {
        let mut refs = Vec::new();
        for element in &self.elements {
            refs.extend(element.get_variable_references());
        }
        refs.sort();
        refs.dedup();
        refs
    }

    /// Check if this object has any filter elements
    pub fn has_filters(&self) -> bool {
        self.elements
            .iter()
            .any(|element| matches!(element, ObjectElement::Filter(_)))
    }

    /// Get all filter specifications from this object
    pub fn get_filters(&self) -> Vec<&FilterSpec> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                ObjectElement::Filter(filter) => Some(filter),
                _ => None,
            })
            .collect()
    }

    /// Get all state dependencies from filters in this object
    pub fn get_filter_state_dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();
        for element in &self.elements {
            if let ObjectElement::Filter(filter) = element {
                deps.extend(filter.get_state_dependencies());
            }
        }
        deps.sort();
        deps.dedup();
        deps
    }

    /// Check if this is an empty object (no elements)
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get element count (for validation)
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Get all set references from this object
    pub fn get_set_references(&self) -> Vec<String> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                ObjectElement::SetRef { set_id } => Some(set_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Check if this object references external symbols (sets, states via filters)
    pub fn has_external_references(&self) -> bool {
        self.elements
            .iter()
            .any(|element| element.has_external_references())
    }
}

impl ObjectElement {
    /// Check if this element has variable references
    pub fn has_variable_references(&self) -> bool {
        match self {
            ObjectElement::Field { value, .. } => value.has_variable_reference(),
            // Other elements don't contain variable references in current ICS spec
            _ => false,
        }
    }

    /// Get variable references from this element
    pub fn get_variable_references(&self) -> Vec<String> {
        match self {
            ObjectElement::Field { value, .. } => {
                if let Some(var_name) = value.get_variable_name() {
                    vec![var_name.to_string()]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    /// Check if this element is a filter
    pub fn is_filter(&self) -> bool {
        matches!(self, ObjectElement::Filter(_))
    }

    /// Get filter specification if this element is a filter
    pub fn as_filter(&self) -> Option<&FilterSpec> {
        match self {
            ObjectElement::Filter(filter) => Some(filter),
            _ => None,
        }
    }

    /// Check if this element references external symbols (sets, filters)
    pub fn has_external_references(&self) -> bool {
        match self {
            ObjectElement::SetRef { .. } => true,
            ObjectElement::Filter(_) => true, // Filters reference states
            _ => false,
        }
    }

    /// Get the element type name for debugging
    pub fn element_type_name(&self) -> &'static str {
        match self {
            ObjectElement::Module { .. } => "Module",
            ObjectElement::Parameter { .. } => "Parameter",
            ObjectElement::Select { .. } => "Select",
            ObjectElement::Behavior { .. } => "Behavior",
            ObjectElement::Filter(_) => "Filter",
            ObjectElement::SetRef { .. } => "SetRef",
            ObjectElement::Field { .. } => "Field",
        }
    }
}

// Display implementations
impl std::fmt::Display for ModuleField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for ObjectElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectElement::Module { field, value } => write!(f, "{} `{}`", field, value),
            ObjectElement::Parameter { data_type, fields } => {
                write!(f, "parameters {} ({} fields)", data_type, fields.len())
            }
            ObjectElement::Select { data_type, fields } => {
                write!(f, "select {} ({} fields)", data_type, fields.len())
            }
            ObjectElement::Behavior { values } => {
                write!(f, "behavior [{}]", values.join(", "))
            }
            ObjectElement::Filter(filter) => write!(f, "{}", filter),
            ObjectElement::SetRef { set_id } => write!(f, "SET_REF {}", set_id),
            ObjectElement::Field { name, value } => match value {
                Value::String(s) => write!(f, "{} `{}`", name, s),
                Value::Variable(v) => write!(f, "{} VAR {}", name, v),
                Value::Integer(i) => write!(f, "{} {}", name, i),
                Value::Float(fl) => write!(f, "{} {}", name, fl),
                Value::Boolean(b) => write!(f, "{} {}", name, b),
            },
        }
    }
}
