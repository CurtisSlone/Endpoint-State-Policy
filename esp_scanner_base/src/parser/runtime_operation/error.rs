use crate::types::runtime_operation::RuntimeOperationType;

/// Runtime operation-specific parsing errors
#[derive(Debug, Clone)]
pub enum RuntimeOperationParsingError {
    /// Failed to parse runtime operation from JSON
    RuntimeOperationParsingFailed {
        target_variable: String,
        cause: String,
    },

    /// Invalid runtime operation type
    InvalidRuntimeOperationType {
        target_variable: String,
        operation_type: String,
        valid_operations: Vec<String>,
    },

    /// Runtime operation parameter parsing failed
    RunParameterParsingFailed {
        target_variable: String,
        parameter_index: usize,
        cause: String,
    },

    /// Unknown run parameter type
    UnknownRunParameterType {
        target_variable: String,
        parameter_index: usize,
        parameter_type: String,
        available_keys: Vec<String>,
    },

    /// Missing required parameters for operation type
    MissingRequiredParameters {
        target_variable: String,
        operation_type: RuntimeOperationType,
        missing_parameters: Vec<String>,
    },

    /// Invalid parameter count for operation type
    InvalidParameterCount {
        target_variable: String,
        operation_type: RuntimeOperationType,
        parameter_count: usize,
        expected_count: Option<usize>,
    },

    /// Object extraction parameter validation failed
    ObjectExtractionValidationFailed {
        target_variable: String,
        object_id: String,
        field_name: String,
        cause: String,
    },

    /// Variable reference validation failed in parameter
    VariableReferenceValidationFailed {
        target_variable: String,
        referenced_variable: String,
        cause: String,
    },

    /// Arithmetic operator validation failed
    ArithmeticOperatorValidationFailed {
        target_variable: String,
        operator: String,
        cause: String,
    },

    /// Pattern specification validation failed
    PatternSpecificationValidationFailed {
        target_variable: String,
        pattern: String,
        cause: String,
    },

    /// Delimiter specification validation failed
    DelimiterSpecificationValidationFailed {
        target_variable: String,
        delimiter: String,
        cause: String,
    },

    /// Position/length parameter validation failed
    PositionParameterValidationFailed {
        target_variable: String,
        parameter_type: String,
        value: i64,
        cause: String,
    },

    /// Empty runtime operation (no parameters)
    EmptyRuntimeOperation { target_variable: String },

    /// Target variable identifier validation failed
    InvalidTargetVariableIdentifier {
        target_variable: String,
        cause: String,
    },

    /// Missing required field in runtime operation definition
    MissingRequiredField {
        target_variable: String,
        missing_field: String,
    },

    /// Literal value parsing failed in parameter
    LiteralValueParsingFailed {
        target_variable: String,
        parameter_index: usize,
        literal_content: String,
        cause: String,
    },

    /// Runtime operation structure validation failed
    RuntimeOperationStructureValidationFailed {
        target_variable: String,
        expected_structure: String,
        actual_structure: String,
    },

    /// Parameter type mismatch for operation
    ParameterTypeMismatch {
        target_variable: String,
        operation_type: RuntimeOperationType,
        parameter_index: usize,
        expected_types: Vec<String>,
        found_type: String,
    },

    /// Multiple parameters of same type when only one allowed
    DuplicateParameterType {
        target_variable: String,
        parameter_type: String,
        first_occurrence_index: usize,
        second_occurrence_index: usize,
    },

    /// CONCAT operation validation failed
    ConcatOperationValidationFailed {
        target_variable: String,
        parameter_count: usize,
        cause: String,
    },

    /// SPLIT operation validation failed
    SplitOperationValidationFailed {
        target_variable: String,
        has_delimiter: bool,
        cause: String,
    },

    /// SUBSTRING operation validation failed
    SubstringOperationValidationFailed {
        target_variable: String,
        has_start: bool,
        has_length: bool,
        cause: String,
    },

    /// REGEX_CAPTURE operation validation failed
    RegexCaptureOperationValidationFailed {
        target_variable: String,
        has_pattern: bool,
        cause: String,
    },

    /// ARITHMETIC operation validation failed
    ArithmeticOperationValidationFailed {
        target_variable: String,
        arithmetic_parameter_count: usize,
        cause: String,
    },

    /// EXTRACT operation validation failed
    ExtractOperationValidationFailed {
        target_variable: String,
        has_object_extraction: bool,
        cause: String,
    },

    /// Runtime operation consistency validation failed
    RuntimeOperationConsistencyValidationFailed {
        target_variable: String,
        operation_type: RuntimeOperationType,
        inconsistency_reason: String,
    },
}

impl RuntimeOperationParsingError {
    /// Create runtime operation parsing error
    pub fn runtime_operation_parsing_failed(target_variable: &str, cause: &str) -> Self {
        Self::RuntimeOperationParsingFailed {
            target_variable: target_variable.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid runtime operation type error
    pub fn invalid_runtime_operation_type(target_variable: &str, operation_type: &str) -> Self {
        Self::InvalidRuntimeOperationType {
            target_variable: target_variable.to_string(),
            operation_type: operation_type.to_string(),
            valid_operations: vec![
                "CONCAT".to_string(),
                "SPLIT".to_string(),
                "SUBSTRING".to_string(),
                "REGEX_CAPTURE".to_string(),
                "ARITHMETIC".to_string(),
                "COUNT".to_string(),
                "UNIQUE".to_string(),
                "END".to_string(),
                "MERGE".to_string(),
                "EXTRACT".to_string(),
            ],
        }
    }

    /// Create run parameter parsing error
    pub fn run_parameter_parsing_failed(
        target_variable: &str,
        parameter_index: usize,
        cause: &str,
    ) -> Self {
        Self::RunParameterParsingFailed {
            target_variable: target_variable.to_string(),
            parameter_index,
            cause: cause.to_string(),
        }
    }

    /// Create unknown run parameter type error
    pub fn unknown_run_parameter_type(
        target_variable: &str,
        parameter_index: usize,
        parameter_type: &str,
        available_keys: Vec<String>,
    ) -> Self {
        Self::UnknownRunParameterType {
            target_variable: target_variable.to_string(),
            parameter_index,
            parameter_type: parameter_type.to_string(),
            available_keys,
        }
    }

    /// Create missing required parameters error
    pub fn missing_required_parameters(
        target_variable: &str,
        operation_type: RuntimeOperationType,
        missing_parameters: Vec<String>,
    ) -> Self {
        Self::MissingRequiredParameters {
            target_variable: target_variable.to_string(),
            operation_type,
            missing_parameters,
        }
    }

    /// Create empty runtime operation error
    pub fn empty_runtime_operation(target_variable: &str) -> Self {
        Self::EmptyRuntimeOperation {
            target_variable: target_variable.to_string(),
        }
    }

    /// Create missing required field error
    pub fn missing_required_field(target_variable: &str, missing_field: &str) -> Self {
        Self::MissingRequiredField {
            target_variable: target_variable.to_string(),
            missing_field: missing_field.to_string(),
        }
    }

    /// Create invalid target variable identifier error
    pub fn invalid_target_variable_identifier(target_variable: &str, cause: &str) -> Self {
        Self::InvalidTargetVariableIdentifier {
            target_variable: target_variable.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create object extraction validation error
    pub fn object_extraction_validation_failed(
        target_variable: &str,
        object_id: &str,
        field_name: &str,
        cause: &str,
    ) -> Self {
        Self::ObjectExtractionValidationFailed {
            target_variable: target_variable.to_string(),
            object_id: object_id.to_string(),
            field_name: field_name.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create duplicate parameter type error
    pub fn duplicate_parameter_type(
        target_variable: &str,
        parameter_type: &str,
        first_index: usize,
        second_index: usize,
    ) -> Self {
        Self::DuplicateParameterType {
            target_variable: target_variable.to_string(),
            parameter_type: parameter_type.to_string(),
            first_occurrence_index: first_index,
            second_occurrence_index: second_index,
        }
    }
}

impl std::fmt::Display for RuntimeOperationParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RuntimeOperationParsingFailed {
                target_variable,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse runtime operation for variable '{}': {}",
                    target_variable, cause
                )
            }
            Self::InvalidRuntimeOperationType {
                target_variable,
                operation_type,
                valid_operations,
            } => {
                write!(
                    f,
                    "Invalid runtime operation type '{}' for variable '{}'. Valid operations: [{}]",
                    operation_type,
                    target_variable,
                    valid_operations.join(", ")
                )
            }
            Self::RunParameterParsingFailed {
                target_variable,
                parameter_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse parameter {} for runtime operation '{}': {}",
                    parameter_index, target_variable, cause
                )
            }
            Self::UnknownRunParameterType {
                target_variable,
                parameter_index,
                parameter_type,
                available_keys,
            } => {
                write!(f, "Unknown parameter type '{}' at index {} for runtime operation '{}'. Available: [{}]", 
                       parameter_type, parameter_index, target_variable, available_keys.join(", "))
            }
            Self::MissingRequiredParameters {
                target_variable,
                operation_type,
                missing_parameters,
            } => {
                write!(
                    f,
                    "Missing required parameters for {:?} operation '{}': [{}]",
                    operation_type,
                    target_variable,
                    missing_parameters.join(", ")
                )
            }
            Self::InvalidParameterCount {
                target_variable,
                operation_type,
                parameter_count,
                expected_count,
            } => {
                if let Some(expected) = expected_count {
                    write!(
                        f,
                        "Invalid parameter count {} for {:?} operation '{}'. Expected: {}",
                        parameter_count, operation_type, target_variable, expected
                    )
                } else {
                    write!(
                        f,
                        "Invalid parameter count {} for {:?} operation '{}'",
                        parameter_count, operation_type, target_variable
                    )
                }
            }
            Self::ObjectExtractionValidationFailed {
                target_variable,
                object_id,
                field_name,
                cause,
            } => {
                write!(
                    f,
                    "Object extraction validation failed for '{}.{}' in runtime operation '{}': {}",
                    object_id, field_name, target_variable, cause
                )
            }
            Self::VariableReferenceValidationFailed {
                target_variable,
                referenced_variable,
                cause,
            } => {
                write!(
                    f,
                    "Variable reference validation failed for '{}' in runtime operation '{}': {}",
                    referenced_variable, target_variable, cause
                )
            }
            Self::ArithmeticOperatorValidationFailed {
                target_variable,
                operator,
                cause,
            } => {
                write!(
                    f,
                    "Arithmetic operator '{}' validation failed for runtime operation '{}': {}",
                    operator, target_variable, cause
                )
            }
            Self::PatternSpecificationValidationFailed {
                target_variable,
                pattern,
                cause,
            } => {
                write!(
                    f,
                    "Pattern specification '{}' validation failed for runtime operation '{}': {}",
                    pattern, target_variable, cause
                )
            }
            Self::DelimiterSpecificationValidationFailed {
                target_variable,
                delimiter,
                cause,
            } => {
                write!(
                    f,
                    "Delimiter specification '{}' validation failed for runtime operation '{}': {}",
                    delimiter, target_variable, cause
                )
            }
            Self::PositionParameterValidationFailed {
                target_variable,
                parameter_type,
                value,
                cause,
            } => {
                write!(f, "Position parameter '{}' with value {} validation failed for runtime operation '{}': {}", 
                       parameter_type, value, target_variable, cause)
            }
            Self::EmptyRuntimeOperation { target_variable } => {
                write!(
                    f,
                    "Runtime operation '{}' has no parameters (empty operations not allowed)",
                    target_variable
                )
            }
            Self::InvalidTargetVariableIdentifier {
                target_variable,
                cause,
            } => {
                write!(
                    f,
                    "Invalid target variable identifier '{}': {}",
                    target_variable, cause
                )
            }
            Self::MissingRequiredField {
                target_variable,
                missing_field,
            } => {
                write!(
                    f,
                    "Runtime operation '{}' missing required field '{}'",
                    target_variable, missing_field
                )
            }
            Self::LiteralValueParsingFailed {
                target_variable,
                parameter_index,
                literal_content,
                cause,
            } => {
                write!(f, "Failed to parse literal value '{}' at parameter {} for runtime operation '{}': {}", 
                       literal_content, parameter_index, target_variable, cause)
            }
            Self::RuntimeOperationStructureValidationFailed {
                target_variable,
                expected_structure,
                actual_structure,
            } => {
                write!(f, "Runtime operation structure validation failed for '{}'. Expected: {}, Found: {}", 
                       target_variable, expected_structure, actual_structure)
            }
            Self::ParameterTypeMismatch {
                target_variable,
                operation_type,
                parameter_index,
                expected_types,
                found_type,
            } => {
                write!(f, "Parameter type mismatch at index {} for {:?} operation '{}'. Expected: [{}], Found: {}", 
                       parameter_index, operation_type, target_variable, expected_types.join(", "), found_type)
            }
            Self::DuplicateParameterType {
                target_variable,
                parameter_type,
                first_occurrence_index,
                second_occurrence_index,
            } => {
                write!(f, "Duplicate parameter type '{}' in runtime operation '{}' at indices {} and {} (duplicates not allowed)", 
                       parameter_type, target_variable, first_occurrence_index, second_occurrence_index)
            }
            Self::ConcatOperationValidationFailed {
                target_variable,
                parameter_count,
                cause,
            } => {
                write!(
                    f,
                    "CONCAT operation validation failed for '{}' with {} parameters: {}",
                    target_variable, parameter_count, cause
                )
            }
            Self::SplitOperationValidationFailed {
                target_variable,
                has_delimiter,
                cause,
            } => {
                write!(
                    f,
                    "SPLIT operation validation failed for '{}' (has delimiter: {}): {}",
                    target_variable, has_delimiter, cause
                )
            }
            Self::SubstringOperationValidationFailed {
                target_variable,
                has_start,
                has_length,
                cause,
            } => {
                write!(f, "SUBSTRING operation validation failed for '{}' (has start: {}, has length: {}): {}", 
                       target_variable, has_start, has_length, cause)
            }
            Self::RegexCaptureOperationValidationFailed {
                target_variable,
                has_pattern,
                cause,
            } => {
                write!(
                    f,
                    "REGEX_CAPTURE operation validation failed for '{}' (has pattern: {}): {}",
                    target_variable, has_pattern, cause
                )
            }
            Self::ArithmeticOperationValidationFailed {
                target_variable,
                arithmetic_parameter_count,
                cause,
            } => {
                write!(f, "ARITHMETIC operation validation failed for '{}' with {} arithmetic parameters: {}", 
                       target_variable, arithmetic_parameter_count, cause)
            }
            Self::ExtractOperationValidationFailed {
                target_variable,
                has_object_extraction,
                cause,
            } => {
                write!(
                    f,
                    "EXTRACT operation validation failed for '{}' (has object extraction: {}): {}",
                    target_variable, has_object_extraction, cause
                )
            }
            Self::RuntimeOperationConsistencyValidationFailed {
                target_variable,
                operation_type,
                inconsistency_reason,
            } => {
                write!(
                    f,
                    "Runtime operation consistency validation failed for {:?} operation '{}': {}",
                    operation_type, target_variable, inconsistency_reason
                )
            }
        }
    }
}

impl std::error::Error for RuntimeOperationParsingError {}
