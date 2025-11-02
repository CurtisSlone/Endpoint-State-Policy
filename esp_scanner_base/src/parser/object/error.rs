use crate::types::common::DataType;

/// Object-specific resolution errors
#[derive(Debug, Clone)]
pub enum ObjectResolutionError {
    /// Failed to parse object element from JSON
    ElementParsingFailed {
        object_id: String,
        element_index: usize,
        cause: String,
    },

    /// Unknown object element type encountered
    UnknownElementType {
        object_id: String,
        element_type: String,
        available_keys: Vec<String>,
    },

    /// Invalid field value format
    InvalidFieldValue {
        object_id: String,
        field_name: String,
        json_content: String,
    },

    /// Parameter block parsing failed
    ParameterParsingFailed {
        object_id: String,
        data_type: DataType,
        cause: String,
    },

    /// Select block parsing failed
    SelectParsingFailed {
        object_id: String,
        data_type: DataType,
        cause: String,
    },

    /// Invalid behavior value
    InvalidBehaviorValue {
        object_id: String,
        behavior_value: String,
    },

    /// Filter specification parsing failed
    FilterParsingFailed { object_id: String, cause: String },

    /// Invalid filter action
    InvalidFilterAction { object_id: String, action: String },

    /// Set reference validation failed
    SetReferenceValidationFailed {
        object_id: String,
        set_id: String,
        cause: String,
    },

    /// Record check parsing failed
    RecordCheckParsingFailed { object_id: String, cause: String },

    /// Invalid operation in record field
    InvalidOperation {
        object_id: String,
        field_path: String,
        operation: String,
    },

    /// Invalid entity check
    InvalidEntityCheck {
        object_id: String,
        field_path: String,
        entity_check: String,
    },

    /// Module specification parsing failed
    ModuleParsingFailed {
        object_id: String,
        module_field: String,
        cause: String,
    },

    /// Object has circular dependency
    CircularDependency {
        object_id: String,
        dependency_chain: Vec<String>,
    },

    /// Object references undefined variable
    UndefinedVariableReference {
        object_id: String,
        variable_name: String,
    },

    /// Type conversion failed during resolution
    TypeConversionFailed {
        object_id: String,
        field_name: String,
        from_type: String,
        to_type: DataType,
    },

    /// Empty object definition (no elements)
    EmptyObjectDefinition { object_id: String },

    /// CTN local object extraction failed
    CtnObjectExtractionFailed {
        ctn_node_id: usize,
        ctn_type: String,
        cause: String,
    },

    /// Multiple local objects in single CTN (violation of ICS rules)
    MultipleCtnObjects {
        ctn_node_id: usize,
        ctn_type: String,
        first_object: String,
        second_object: String,
    },

    /// Invalid data type specification
    InvalidDataType {
        object_id: String,
        element_type: String,
        data_type: String,
    },

    /// Field path parsing failed for record operations
    FieldPathParsingFailed {
        object_id: String,
        field_path: String,
        cause: String,
    },
}

impl ObjectResolutionError {
    /// Create element parsing error
    pub fn element_parsing_failed(object_id: &str, element_index: usize, cause: &str) -> Self {
        Self::ElementParsingFailed {
            object_id: object_id.to_string(),
            element_index,
            cause: cause.to_string(),
        }
    }

    /// Create unknown element type error
    pub fn unknown_element_type(
        object_id: &str,
        element_type: &str,
        available_keys: Vec<String>,
    ) -> Self {
        Self::UnknownElementType {
            object_id: object_id.to_string(),
            element_type: element_type.to_string(),
            available_keys,
        }
    }

    /// Create invalid field value error
    pub fn invalid_field_value(object_id: &str, field_name: &str, json_content: &str) -> Self {
        Self::InvalidFieldValue {
            object_id: object_id.to_string(),
            field_name: field_name.to_string(),
            json_content: json_content.to_string(),
        }
    }

    /// Create CTN object extraction error
    pub fn ctn_object_extraction_failed(ctn_node_id: usize, ctn_type: &str, cause: &str) -> Self {
        Self::CtnObjectExtractionFailed {
            ctn_node_id,
            ctn_type: ctn_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create multiple CTN objects error
    pub fn multiple_ctn_objects(
        ctn_node_id: usize,
        ctn_type: &str,
        first_object: &str,
        second_object: &str,
    ) -> Self {
        Self::MultipleCtnObjects {
            ctn_node_id,
            ctn_type: ctn_type.to_string(),
            first_object: first_object.to_string(),
            second_object: second_object.to_string(),
        }
    }

    /// Create empty object definition error
    pub fn empty_object_definition(object_id: &str) -> Self {
        Self::EmptyObjectDefinition {
            object_id: object_id.to_string(),
        }
    }

    /// Create undefined variable reference error
    pub fn undefined_variable_reference(object_id: &str, variable_name: &str) -> Self {
        Self::UndefinedVariableReference {
            object_id: object_id.to_string(),
            variable_name: variable_name.to_string(),
        }
    }
}

impl std::fmt::Display for ObjectResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ElementParsingFailed {
                object_id,
                element_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse element {} in object '{}': {}",
                    element_index, object_id, cause
                )
            }
            Self::UnknownElementType {
                object_id,
                element_type,
                available_keys,
            } => {
                write!(
                    f,
                    "Unknown element type '{}' in object '{}'. Available: [{}]",
                    element_type,
                    object_id,
                    available_keys.join(", ")
                )
            }
            Self::InvalidFieldValue {
                object_id,
                field_name,
                json_content,
            } => {
                write!(
                    f,
                    "Invalid field value for '{}' in object '{}': {}",
                    field_name, object_id, json_content
                )
            }
            Self::ParameterParsingFailed {
                object_id,
                data_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse parameter block ({:?}) in object '{}': {}",
                    data_type, object_id, cause
                )
            }
            Self::SelectParsingFailed {
                object_id,
                data_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse select block ({:?}) in object '{}': {}",
                    data_type, object_id, cause
                )
            }
            Self::InvalidBehaviorValue {
                object_id,
                behavior_value,
            } => {
                write!(
                    f,
                    "Invalid behavior value '{}' in object '{}'",
                    behavior_value, object_id
                )
            }
            Self::FilterParsingFailed { object_id, cause } => {
                write!(
                    f,
                    "Failed to parse filter in object '{}': {}",
                    object_id, cause
                )
            }
            Self::InvalidFilterAction { object_id, action } => {
                write!(
                    f,
                    "Invalid filter action '{}' in object '{}'",
                    action, object_id
                )
            }
            Self::SetReferenceValidationFailed {
                object_id,
                set_id,
                cause,
            } => {
                write!(
                    f,
                    "Set reference validation failed for '{}' in object '{}': {}",
                    set_id, object_id, cause
                )
            }
            Self::RecordCheckParsingFailed { object_id, cause } => {
                write!(
                    f,
                    "Failed to parse record check in object '{}': {}",
                    object_id, cause
                )
            }
            Self::InvalidOperation {
                object_id,
                field_path,
                operation,
            } => {
                write!(
                    f,
                    "Invalid operation '{}' for field '{}' in object '{}'",
                    operation, field_path, object_id
                )
            }
            Self::InvalidEntityCheck {
                object_id,
                field_path,
                entity_check,
            } => {
                write!(
                    f,
                    "Invalid entity check '{}' for field '{}' in object '{}'",
                    entity_check, field_path, object_id
                )
            }
            Self::ModuleParsingFailed {
                object_id,
                module_field,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse module field '{}' in object '{}': {}",
                    module_field, object_id, cause
                )
            }
            Self::CircularDependency {
                object_id,
                dependency_chain,
            } => {
                write!(
                    f,
                    "Circular dependency detected for object '{}': {}",
                    object_id,
                    dependency_chain.join(" -> ")
                )
            }
            Self::UndefinedVariableReference {
                object_id,
                variable_name,
            } => {
                write!(
                    f,
                    "Object '{}' references undefined variable '{}'",
                    object_id, variable_name
                )
            }
            Self::TypeConversionFailed {
                object_id,
                field_name,
                from_type,
                to_type,
            } => {
                write!(
                    f,
                    "Type conversion failed for field '{}' in object '{}': {} -> {:?}",
                    field_name, object_id, from_type, to_type
                )
            }
            Self::EmptyObjectDefinition { object_id } => {
                write!(
                    f,
                    "Object '{}' has no elements (empty object definitions not allowed)",
                    object_id
                )
            }
            Self::CtnObjectExtractionFailed {
                ctn_node_id,
                ctn_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to extract local object from CTN {} ({}): {}",
                    ctn_node_id, ctn_type, cause
                )
            }
            Self::MultipleCtnObjects {
                ctn_node_id,
                ctn_type,
                first_object,
                second_object,
            } => {
                write!(
                    f,
                    "CTN {} ({}) has multiple objects: '{}' and '{}' (max 1 allowed)",
                    ctn_node_id, ctn_type, first_object, second_object
                )
            }
            Self::InvalidDataType {
                object_id,
                element_type,
                data_type,
            } => {
                write!(
                    f,
                    "Invalid data type '{}' for {} element in object '{}'",
                    data_type, element_type, object_id
                )
            }
            Self::FieldPathParsingFailed {
                object_id,
                field_path,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse field path '{}' in object '{}': {}",
                    field_path, object_id, cause
                )
            }
        }
    }
}

impl std::error::Error for ObjectResolutionError {}
