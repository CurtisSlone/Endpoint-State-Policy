use crate::types::set::SetOperationType;

/// Set-specific parsing errors
#[derive(Debug, Clone)]
pub enum SetParsingError {
    /// Failed to parse set operation from JSON
    SetOperationParsingFailed { set_id: String, cause: String },

    /// Invalid set operation type
    InvalidSetOperationType {
        set_id: String,
        operation_type: String,
        valid_operations: Vec<String>,
    },

    /// Set operand parsing failed
    SetOperandParsingFailed {
        set_id: String,
        operand_index: usize,
        cause: String,
    },

    /// Invalid operand count for operation type
    InvalidOperandCount {
        set_id: String,
        operation_type: SetOperationType,
        operand_count: usize,
        min_required: usize,
        max_allowed: Option<usize>,
    },

    /// Unknown set operand type
    UnknownSetOperandType {
        set_id: String,
        operand_index: usize,
        operand_type: String,
        available_keys: Vec<String>,
    },

    /// Set reference validation failed
    SetReferenceValidationFailed {
        set_id: String,
        referenced_set_id: String,
        cause: String,
    },

    /// Object reference validation failed in set operand
    ObjectReferenceValidationFailed {
        set_id: String,
        object_id: String,
        cause: String,
    },

    /// Inline object parsing failed in set operand
    InlineObjectParsingFailed {
        set_id: String,
        operand_index: usize,
        cause: String,
    },

    /// Filter specification parsing failed for set
    SetFilterParsingFailed { set_id: String, cause: String },

    /// Set identifier validation failed
    InvalidSetIdentifier { set_id: String, cause: String },

    /// Empty set definition (no operands)
    EmptySetDefinition { set_id: String },

    /// Missing required field in set definition
    MissingRequiredField {
        set_id: String,
        missing_field: String,
    },

    /// Set operation structure validation failed
    SetOperationStructureValidationFailed {
        set_id: String,
        expected_structure: String,
        actual_structure: String,
    },

    /// Circular set reference detected
    CircularSetReference {
        set_id: String,
        reference_chain: Vec<String>,
    },

    /// Set operand type mismatch
    SetOperandTypeMismatch {
        set_id: String,
        operand_index: usize,
        expected_types: Vec<String>,
        found_type: String,
    },

    /// Filter action validation failed for set
    SetFilterActionValidationFailed {
        set_id: String,
        filter_action: String,
        cause: String,
    },

    /// Filter state references validation failed for set
    SetFilterStateReferencesValidationFailed {
        set_id: String,
        state_references: Vec<String>,
        cause: String,
    },

    /// Set union operand validation failed
    UnionOperandValidationFailed {
        set_id: String,
        operand_index: usize,
        operand_content: String,
    },

    /// Set intersection operand validation failed
    IntersectionOperandValidationFailed {
        set_id: String,
        operand_index: usize,
        operand_content: String,
    },

    /// Set complement operand validation failed
    ComplementOperandValidationFailed {
        set_id: String,
        operand_index: usize,
        operand_content: String,
    },

    /// Set operation consistency validation failed
    SetOperationConsistencyValidationFailed {
        set_id: String,
        operation_type: SetOperationType,
        inconsistency_reason: String,
    },

    /// Nested set depth limit exceeded
    NestedSetDepthLimitExceeded {
        set_id: String,
        depth: usize,
        max_depth: usize,
    },

    /// Set operand reference resolution failed
    SetOperandReferenceResolutionFailed {
        set_id: String,
        operand_index: usize,
        reference_id: String,
        resolution_error: String,
    },
}

impl SetParsingError {
    /// Create set operation parsing error
    pub fn set_operation_parsing_failed(set_id: &str, cause: &str) -> Self {
        Self::SetOperationParsingFailed {
            set_id: set_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid set operation type error
    pub fn invalid_set_operation_type(set_id: &str, operation_type: &str) -> Self {
        Self::InvalidSetOperationType {
            set_id: set_id.to_string(),
            operation_type: operation_type.to_string(),
            valid_operations: vec![
                "union".to_string(),
                "intersection".to_string(),
                "complement".to_string(),
            ],
        }
    }

    /// Create set operand parsing error
    pub fn set_operand_parsing_failed(set_id: &str, operand_index: usize, cause: &str) -> Self {
        Self::SetOperandParsingFailed {
            set_id: set_id.to_string(),
            operand_index,
            cause: cause.to_string(),
        }
    }

    /// Create invalid operand count error
    pub fn invalid_operand_count(
        set_id: &str,
        operation_type: SetOperationType,
        operand_count: usize,
        min_required: usize,
        max_allowed: Option<usize>,
    ) -> Self {
        Self::InvalidOperandCount {
            set_id: set_id.to_string(),
            operation_type,
            operand_count,
            min_required,
            max_allowed,
        }
    }

    /// Create unknown set operand type error
    pub fn unknown_set_operand_type(
        set_id: &str,
        operand_index: usize,
        operand_type: &str,
        available_keys: Vec<String>,
    ) -> Self {
        Self::UnknownSetOperandType {
            set_id: set_id.to_string(),
            operand_index,
            operand_type: operand_type.to_string(),
            available_keys,
        }
    }

    /// Create empty set definition error
    pub fn empty_set_definition(set_id: &str) -> Self {
        Self::EmptySetDefinition {
            set_id: set_id.to_string(),
        }
    }

    /// Create missing required field error
    pub fn missing_required_field(set_id: &str, missing_field: &str) -> Self {
        Self::MissingRequiredField {
            set_id: set_id.to_string(),
            missing_field: missing_field.to_string(),
        }
    }

    /// Create invalid set identifier error
    pub fn invalid_set_identifier(set_id: &str, cause: &str) -> Self {
        Self::InvalidSetIdentifier {
            set_id: set_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create circular set reference error
    pub fn circular_set_reference(set_id: &str, reference_chain: Vec<String>) -> Self {
        Self::CircularSetReference {
            set_id: set_id.to_string(),
            reference_chain,
        }
    }

    /// Create set filter parsing error
    pub fn set_filter_parsing_failed(set_id: &str, cause: &str) -> Self {
        Self::SetFilterParsingFailed {
            set_id: set_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create inline object parsing error
    pub fn inline_object_parsing_failed(set_id: &str, operand_index: usize, cause: &str) -> Self {
        Self::InlineObjectParsingFailed {
            set_id: set_id.to_string(),
            operand_index,
            cause: cause.to_string(),
        }
    }
}

impl std::fmt::Display for SetParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetOperationParsingFailed { set_id, cause } => {
                write!(f, "Failed to parse set operation '{}': {}", set_id, cause)
            }
            Self::InvalidSetOperationType {
                set_id,
                operation_type,
                valid_operations,
            } => {
                write!(
                    f,
                    "Invalid set operation type '{}' for set '{}'. Valid operations: [{}]",
                    operation_type,
                    set_id,
                    valid_operations.join(", ")
                )
            }
            Self::SetOperandParsingFailed {
                set_id,
                operand_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse operand {} in set '{}': {}",
                    operand_index, set_id, cause
                )
            }
            Self::InvalidOperandCount {
                set_id,
                operation_type,
                operand_count,
                min_required,
                max_allowed,
            } => {
                if let Some(max) = max_allowed {
                    write!(
                        f,
                        "Invalid operand count {} for {:?} operation in set '{}'. Required: {}-{}",
                        operand_count, operation_type, set_id, min_required, max
                    )
                } else {
                    write!(f, "Invalid operand count {} for {:?} operation in set '{}'. Minimum required: {}", 
                           operand_count, operation_type, set_id, min_required)
                }
            }
            Self::UnknownSetOperandType {
                set_id,
                operand_index,
                operand_type,
                available_keys,
            } => {
                write!(
                    f,
                    "Unknown operand type '{}' at index {} in set '{}'. Available: [{}]",
                    operand_type,
                    operand_index,
                    set_id,
                    available_keys.join(", ")
                )
            }
            Self::SetReferenceValidationFailed {
                set_id,
                referenced_set_id,
                cause,
            } => {
                write!(
                    f,
                    "Set reference validation failed for '{}' in set '{}': {}",
                    referenced_set_id, set_id, cause
                )
            }
            Self::ObjectReferenceValidationFailed {
                set_id,
                object_id,
                cause,
            } => {
                write!(
                    f,
                    "Object reference validation failed for '{}' in set '{}': {}",
                    object_id, set_id, cause
                )
            }
            Self::InlineObjectParsingFailed {
                set_id,
                operand_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse inline object at operand {} in set '{}': {}",
                    operand_index, set_id, cause
                )
            }
            Self::SetFilterParsingFailed { set_id, cause } => {
                write!(f, "Failed to parse filter for set '{}': {}", set_id, cause)
            }
            Self::InvalidSetIdentifier { set_id, cause } => {
                write!(f, "Invalid set identifier '{}': {}", set_id, cause)
            }
            Self::EmptySetDefinition { set_id } => {
                write!(
                    f,
                    "Set '{}' has no operands (empty sets not allowed)",
                    set_id
                )
            }
            Self::MissingRequiredField {
                set_id,
                missing_field,
            } => {
                write!(
                    f,
                    "Set '{}' missing required field '{}'",
                    set_id, missing_field
                )
            }
            Self::SetOperationStructureValidationFailed {
                set_id,
                expected_structure,
                actual_structure,
            } => {
                write!(
                    f,
                    "Set operation structure validation failed for '{}'. Expected: {}, Found: {}",
                    set_id, expected_structure, actual_structure
                )
            }
            Self::CircularSetReference {
                set_id,
                reference_chain,
            } => {
                write!(
                    f,
                    "Circular set reference detected for '{}': {}",
                    set_id,
                    reference_chain.join(" -> ")
                )
            }
            Self::SetOperandTypeMismatch {
                set_id,
                operand_index,
                expected_types,
                found_type,
            } => {
                write!(
                    f,
                    "Set operand type mismatch at index {} in set '{}'. Expected: [{}], Found: {}",
                    operand_index,
                    set_id,
                    expected_types.join(", "),
                    found_type
                )
            }
            Self::SetFilterActionValidationFailed {
                set_id,
                filter_action,
                cause,
            } => {
                write!(
                    f,
                    "Set filter action '{}' validation failed for set '{}': {}",
                    filter_action, set_id, cause
                )
            }
            Self::SetFilterStateReferencesValidationFailed {
                set_id,
                state_references,
                cause,
            } => {
                write!(
                    f,
                    "Set filter state references [{}] validation failed for set '{}': {}",
                    state_references.join(", "),
                    set_id,
                    cause
                )
            }
            Self::UnionOperandValidationFailed {
                set_id,
                operand_index,
                operand_content,
            } => {
                write!(
                    f,
                    "Union operand validation failed at index {} in set '{}': {}",
                    operand_index, set_id, operand_content
                )
            }
            Self::IntersectionOperandValidationFailed {
                set_id,
                operand_index,
                operand_content,
            } => {
                write!(
                    f,
                    "Intersection operand validation failed at index {} in set '{}': {}",
                    operand_index, set_id, operand_content
                )
            }
            Self::ComplementOperandValidationFailed {
                set_id,
                operand_index,
                operand_content,
            } => {
                write!(
                    f,
                    "Complement operand validation failed at index {} in set '{}': {}",
                    operand_index, set_id, operand_content
                )
            }
            Self::SetOperationConsistencyValidationFailed {
                set_id,
                operation_type,
                inconsistency_reason,
            } => {
                write!(f, "Set operation consistency validation failed for {:?} operation in set '{}': {}", 
                       operation_type, set_id, inconsistency_reason)
            }
            Self::NestedSetDepthLimitExceeded {
                set_id,
                depth,
                max_depth,
            } => {
                write!(
                    f,
                    "Nested set depth limit exceeded for set '{}': {} > {} (max allowed)",
                    set_id, depth, max_depth
                )
            }
            Self::SetOperandReferenceResolutionFailed {
                set_id,
                operand_index,
                reference_id,
                resolution_error,
            } => {
                write!(
                    f,
                    "Set operand reference resolution failed for '{}' at index {} in set '{}': {}",
                    reference_id, operand_index, set_id, resolution_error
                )
            }
        }
    }
}

impl std::error::Error for SetParsingError {}
