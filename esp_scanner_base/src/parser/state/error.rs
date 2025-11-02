use crate::types::common::{DataType, Operation};

/// State-specific parsing errors
#[derive(Debug, Clone)]
pub enum StateParsingError {
    /// Failed to parse state declaration from JSON
    StateDeclarationParsingFailed { state_id: String, cause: String },

    /// State field parsing failed
    StateFieldParsingFailed {
        state_id: String,
        field_index: usize,
        field_name: String,
        cause: String,
    },

    /// Invalid operation for field data type
    InvalidOperationForDataType {
        state_id: String,
        field_name: String,
        data_type: DataType,
        operation: Operation,
    },

    /// Entity check parsing failed
    EntityCheckParsingFailed {
        state_id: String,
        field_name: String,
        entity_check: String,
    },

    /// Record check parsing failed
    RecordCheckParsingFailed {
        state_id: String,
        record_index: usize,
        cause: String,
    },

    /// Invalid record content type
    InvalidRecordContentType {
        state_id: String,
        record_index: usize,
        content_type: String,
    },

    /// Record field path parsing failed
    RecordFieldPathParsingFailed {
        state_id: String,
        field_path: String,
        cause: String,
    },

    /// Invalid field path format
    InvalidFieldPathFormat {
        state_id: String,
        field_path: String,
        expected_format: String,
    },

    /// Empty state definition (no fields or record checks)
    EmptyStateDefinition { state_id: String },

    /// Missing required field in state field definition
    MissingRequiredField {
        state_id: String,
        field_name: String,
        missing_field: String,
    },

    /// Invalid data type specification
    InvalidDataType {
        state_id: String,
        field_name: String,
        data_type: String,
    },

    /// State identifier validation failed
    InvalidStateIdentifier { state_id: String, cause: String },

    /// CTN state extraction failed
    CtnStateExtractionFailed {
        ctn_node_id: usize,
        ctn_type: String,
        cause: String,
    },

    /// Value parsing failed for state field
    ValueParsingFailed {
        state_id: String,
        field_name: String,
        value_content: String,
        cause: String,
    },

    /// Invalid operation specification
    InvalidOperation {
        state_id: String,
        field_name: String,
        operation: String,
        valid_operations: Vec<String>,
    },

    /// Record data type validation failed
    RecordDataTypeValidationFailed {
        state_id: String,
        record_index: usize,
        data_type: String,
    },

    /// Nested record field validation failed
    NestedRecordFieldValidationFailed {
        state_id: String,
        field_path: String,
        validation_error: String,
    },

    /// Direct record operation validation failed
    DirectRecordOperationValidationFailed {
        state_id: String,
        operation: String,
        cause: String,
    },

    /// State reference parsing failed
    StateReferenceParsingFailed {
        context: String,
        reference_content: String,
        cause: String,
    },

    /// Multiple entity checks specified (only one allowed)
    MultipleEntityChecks {
        state_id: String,
        field_name: String,
        entity_checks: Vec<String>,
    },

    /// Field name collision within state
    FieldNameCollision {
        state_id: String,
        field_name: String,
        first_occurrence_index: usize,
        second_occurrence_index: usize,
    },
}

impl StateParsingError {
    /// Create state declaration parsing error
    pub fn state_declaration_parsing_failed(state_id: &str, cause: &str) -> Self {
        Self::StateDeclarationParsingFailed {
            state_id: state_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create state field parsing error
    pub fn state_field_parsing_failed(
        state_id: &str,
        field_index: usize,
        field_name: &str,
        cause: &str,
    ) -> Self {
        Self::StateFieldParsingFailed {
            state_id: state_id.to_string(),
            field_index,
            field_name: field_name.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid operation for data type error
    pub fn invalid_operation_for_data_type(
        state_id: &str,
        field_name: &str,
        data_type: DataType,
        operation: Operation,
    ) -> Self {
        Self::InvalidOperationForDataType {
            state_id: state_id.to_string(),
            field_name: field_name.to_string(),
            data_type,
            operation,
        }
    }

    /// Create empty state definition error
    pub fn empty_state_definition(state_id: &str) -> Self {
        Self::EmptyStateDefinition {
            state_id: state_id.to_string(),
        }
    }

    /// Create CTN state extraction error
    pub fn ctn_state_extraction_failed(ctn_node_id: usize, ctn_type: &str, cause: &str) -> Self {
        Self::CtnStateExtractionFailed {
            ctn_node_id,
            ctn_type: ctn_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create missing required field error
    pub fn missing_required_field(state_id: &str, field_name: &str, missing_field: &str) -> Self {
        Self::MissingRequiredField {
            state_id: state_id.to_string(),
            field_name: field_name.to_string(),
            missing_field: missing_field.to_string(),
        }
    }

    /// Create invalid state identifier error
    pub fn invalid_state_identifier(state_id: &str, cause: &str) -> Self {
        Self::InvalidStateIdentifier {
            state_id: state_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create field name collision error
    pub fn field_name_collision(
        state_id: &str,
        field_name: &str,
        first_index: usize,
        second_index: usize,
    ) -> Self {
        Self::FieldNameCollision {
            state_id: state_id.to_string(),
            field_name: field_name.to_string(),
            first_occurrence_index: first_index,
            second_occurrence_index: second_index,
        }
    }
}

impl std::fmt::Display for StateParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StateDeclarationParsingFailed { state_id, cause } => {
                write!(f, "Failed to parse state '{}': {}", state_id, cause)
            }
            Self::StateFieldParsingFailed {
                state_id,
                field_index,
                field_name,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse field '{}' (index {}) in state '{}': {}",
                    field_name, field_index, state_id, cause
                )
            }
            Self::InvalidOperationForDataType {
                state_id,
                field_name,
                data_type,
                operation,
            } => {
                write!(
                    f,
                    "Invalid operation '{:?}' for data type '{:?}' in field '{}' of state '{}'",
                    operation, data_type, field_name, state_id
                )
            }
            Self::EntityCheckParsingFailed {
                state_id,
                field_name,
                entity_check,
            } => {
                write!(
                    f,
                    "Failed to parse entity check '{}' for field '{}' in state '{}'",
                    entity_check, field_name, state_id
                )
            }
            Self::RecordCheckParsingFailed {
                state_id,
                record_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse record check {} in state '{}': {}",
                    record_index, state_id, cause
                )
            }
            Self::InvalidRecordContentType {
                state_id,
                record_index,
                content_type,
            } => {
                write!(
                    f,
                    "Invalid record content type '{}' in record {} of state '{}'",
                    content_type, record_index, state_id
                )
            }
            Self::RecordFieldPathParsingFailed {
                state_id,
                field_path,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse record field path '{}' in state '{}': {}",
                    field_path, state_id, cause
                )
            }
            Self::InvalidFieldPathFormat {
                state_id,
                field_path,
                expected_format,
            } => {
                write!(
                    f,
                    "Invalid field path format '{}' in state '{}'. Expected: {}",
                    field_path, state_id, expected_format
                )
            }
            Self::EmptyStateDefinition { state_id } => {
                write!(
                    f,
                    "State '{}' has no fields or record checks (empty states not allowed)",
                    state_id
                )
            }
            Self::MissingRequiredField {
                state_id,
                field_name,
                missing_field,
            } => {
                write!(
                    f,
                    "Field '{}' in state '{}' missing required field '{}'",
                    field_name, state_id, missing_field
                )
            }
            Self::InvalidDataType {
                state_id,
                field_name,
                data_type,
            } => {
                write!(
                    f,
                    "Invalid data type '{}' for field '{}' in state '{}'",
                    data_type, field_name, state_id
                )
            }
            Self::InvalidStateIdentifier { state_id, cause } => {
                write!(f, "Invalid state identifier '{}': {}", state_id, cause)
            }
            Self::CtnStateExtractionFailed {
                ctn_node_id,
                ctn_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to extract local states from CTN {} ({}): {}",
                    ctn_node_id, ctn_type, cause
                )
            }
            Self::ValueParsingFailed {
                state_id,
                field_name,
                value_content,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse value '{}' for field '{}' in state '{}': {}",
                    value_content, field_name, state_id, cause
                )
            }
            Self::InvalidOperation {
                state_id,
                field_name,
                operation,
                valid_operations,
            } => {
                write!(
                    f,
                    "Invalid operation '{}' for field '{}' in state '{}'. Valid: [{}]",
                    operation,
                    field_name,
                    state_id,
                    valid_operations.join(", ")
                )
            }
            Self::RecordDataTypeValidationFailed {
                state_id,
                record_index,
                data_type,
            } => {
                write!(
                    f,
                    "Record data type validation failed for '{}' in record {} of state '{}'",
                    data_type, record_index, state_id
                )
            }
            Self::NestedRecordFieldValidationFailed {
                state_id,
                field_path,
                validation_error,
            } => {
                write!(
                    f,
                    "Nested record field validation failed for '{}' in state '{}': {}",
                    field_path, state_id, validation_error
                )
            }
            Self::DirectRecordOperationValidationFailed {
                state_id,
                operation,
                cause,
            } => {
                write!(
                    f,
                    "Direct record operation '{}' validation failed in state '{}': {}",
                    operation, state_id, cause
                )
            }
            Self::StateReferenceParsingFailed {
                context,
                reference_content,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse state reference '{}' in context '{}': {}",
                    reference_content, context, cause
                )
            }
            Self::MultipleEntityChecks {
                state_id,
                field_name,
                entity_checks,
            } => {
                write!(f, "Multiple entity checks specified for field '{}' in state '{}': [{}] (only one allowed)", 
                       field_name, state_id, entity_checks.join(", "))
            }
            Self::FieldNameCollision {
                state_id,
                field_name,
                first_occurrence_index,
                second_occurrence_index,
            } => {
                write!(f, "Field name collision in state '{}': field '{}' appears at indices {} and {} (duplicate names not allowed)", 
                       state_id, field_name, first_occurrence_index, second_occurrence_index)
            }
        }
    }
}

impl std::error::Error for StateParsingError {}
