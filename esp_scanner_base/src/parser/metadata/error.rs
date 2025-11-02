// parser/metadata/error.rs

/// Metadata-specific parsing errors
#[derive(Debug, Clone)]
pub enum MetadataParsingError {
    /// Metadata block missing required 'fields' object
    MissingFieldsObject,

    /// Metadata fields must be an object, not another type
    InvalidFieldsType { actual_type: String },

    /// Metadata field key cannot be empty or whitespace only
    EmptyFieldKey { field_key: String },

    /// Metadata field value must be a string
    InvalidFieldValueType {
        field_key: String,
        actual_type: String,
    },

    /// Reserved field name cannot be used in metadata
    ReservedFieldName { field_name: String },

    /// Field name exceeds maximum allowed length
    FieldNameTooLong {
        field_name: String,
        actual_length: usize,
        max_length: usize,
    },

    /// Field value exceeds maximum allowed length
    FieldValueTooLong {
        field_name: String,
        actual_length: usize,
        max_length: usize,
    },

    /// Metadata contains duplicate field names (case-sensitive)
    DuplicateFieldName { field_name: String },

    /// Metadata validation failed for business rules
    ValidationFailed { rule: String, cause: String },

    /// JSON structure is invalid for metadata parsing
    InvalidJsonStructure { cause: String },

    /// Required field for ICS scan output is missing
    MissingRequiredField {
        field_name: String,
        required_for: String,
    },

    /// Required field for ICS scan output is empty
    EmptyRequiredField {
        field_name: String,
        required_for: String,
    },

    /// Multiple required fields are missing for ICS scan output
    MultipleRequiredFieldsMissing {
        missing_fields: Vec<String>,
        required_for: String,
    },

    /// Multiple required fields are empty for ICS scan output
    MultipleRequiredFieldsEmpty {
        empty_fields: Vec<String>,
        required_for: String,
    },

    /// ICS scan output validation failed (combination of missing and empty fields)
    IcsScanOutputValidationFailed {
        missing_fields: Vec<String>,
        empty_fields: Vec<String>,
    },

    /// Invalid ics_scan_id format
    InvalidIcsScanIdFormat {
        scan_id: String,
        format_requirements: String,
    },

    /// Invalid criticality level value
    InvalidCriticalityLevel {
        criticality: String,
        valid_levels: Vec<String>,
    },

    /// Invalid tags format
    InvalidTagsFormat {
        tags: String,
        format_requirements: String,
    },

    /// Invalid control framework value
    InvalidControlFramework {
        framework: String,
        validation_error: String,
    },

    /// Invalid control value
    InvalidControl {
        control: String,
        validation_error: String,
    },

    /// Invalid platform value
    InvalidPlatform {
        platform: String,
        validation_error: String,
    },

    /// Field value validation failed for ICS scan output
    FieldValueValidationFailed {
        field_name: String,
        field_value: String,
        validation_error: String,
    },
}

impl MetadataParsingError {
    /// Create missing fields object error
    pub fn missing_fields_object() -> Self {
        Self::MissingFieldsObject
    }

    /// Create invalid fields type error
    pub fn invalid_fields_type(actual_type: &str) -> Self {
        Self::InvalidFieldsType {
            actual_type: actual_type.to_string(),
        }
    }

    /// Create empty field key error
    pub fn empty_field_key(field_key: &str) -> Self {
        Self::EmptyFieldKey {
            field_key: field_key.to_string(),
        }
    }

    /// Create invalid field value type error
    pub fn invalid_field_value_type(field_key: &str, actual_type: &str) -> Self {
        Self::InvalidFieldValueType {
            field_key: field_key.to_string(),
            actual_type: actual_type.to_string(),
        }
    }

    /// Create reserved field name error
    pub fn reserved_field_name(field_name: &str) -> Self {
        Self::ReservedFieldName {
            field_name: field_name.to_string(),
        }
    }

    /// Create field name too long error
    pub fn field_name_too_long(field_name: &str, actual_length: usize, max_length: usize) -> Self {
        Self::FieldNameTooLong {
            field_name: field_name.to_string(),
            actual_length,
            max_length,
        }
    }

    /// Create field value too long error
    pub fn field_value_too_long(field_name: &str, actual_length: usize, max_length: usize) -> Self {
        Self::FieldValueTooLong {
            field_name: field_name.to_string(),
            actual_length,
            max_length,
        }
    }

    /// Create duplicate field name error
    pub fn duplicate_field_name(field_name: &str) -> Self {
        Self::DuplicateFieldName {
            field_name: field_name.to_string(),
        }
    }

    /// Create validation failed error
    pub fn validation_failed(rule: &str, cause: &str) -> Self {
        Self::ValidationFailed {
            rule: rule.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid JSON structure error
    pub fn invalid_json_structure(cause: &str) -> Self {
        Self::InvalidJsonStructure {
            cause: cause.to_string(),
        }
    }

    /// Create missing required field error
    pub fn missing_required_field(field_name: &str, required_for: &str) -> Self {
        Self::MissingRequiredField {
            field_name: field_name.to_string(),
            required_for: required_for.to_string(),
        }
    }

    /// Create empty required field error
    pub fn empty_required_field(field_name: &str, required_for: &str) -> Self {
        Self::EmptyRequiredField {
            field_name: field_name.to_string(),
            required_for: required_for.to_string(),
        }
    }

    /// Create multiple required fields missing error
    pub fn multiple_required_fields_missing(
        missing_fields: Vec<String>,
        required_for: &str,
    ) -> Self {
        Self::MultipleRequiredFieldsMissing {
            missing_fields,
            required_for: required_for.to_string(),
        }
    }

    /// Create multiple required fields empty error
    pub fn multiple_required_fields_empty(empty_fields: Vec<String>, required_for: &str) -> Self {
        Self::MultipleRequiredFieldsEmpty {
            empty_fields,
            required_for: required_for.to_string(),
        }
    }

    /// Create ICS scan output validation failed error
    pub fn ics_scan_output_validation_failed(
        missing_fields: Vec<String>,
        empty_fields: Vec<String>,
    ) -> Self {
        Self::IcsScanOutputValidationFailed {
            missing_fields,
            empty_fields,
        }
    }

    /// Create invalid ics_scan_id format error
    pub fn invalid_ics_scan_id_format(scan_id: &str, format_requirements: &str) -> Self {
        Self::InvalidIcsScanIdFormat {
            scan_id: scan_id.to_string(),
            format_requirements: format_requirements.to_string(),
        }
    }

    /// Create invalid criticality level error
    pub fn invalid_criticality_level(criticality: &str, valid_levels: Vec<String>) -> Self {
        Self::InvalidCriticalityLevel {
            criticality: criticality.to_string(),
            valid_levels,
        }
    }

    /// Create invalid tags format error
    pub fn invalid_tags_format(tags: &str, format_requirements: &str) -> Self {
        Self::InvalidTagsFormat {
            tags: tags.to_string(),
            format_requirements: format_requirements.to_string(),
        }
    }

    /// Create field value validation failed error
    pub fn field_value_validation_failed(
        field_name: &str,
        field_value: &str,
        validation_error: &str,
    ) -> Self {
        Self::FieldValueValidationFailed {
            field_name: field_name.to_string(),
            field_value: field_value.to_string(),
            validation_error: validation_error.to_string(),
        }
    }
}

impl std::fmt::Display for MetadataParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFieldsObject => {
                write!(f, "Metadata block missing required 'fields' object")
            }
            Self::InvalidFieldsType { actual_type } => {
                write!(
                    f,
                    "Metadata 'fields' must be an object, found {}",
                    actual_type
                )
            }
            Self::EmptyFieldKey { field_key } => {
                write!(
                    f,
                    "Metadata field key cannot be empty or whitespace only: '{}'",
                    field_key
                )
            }
            Self::InvalidFieldValueType {
                field_key,
                actual_type,
            } => {
                write!(
                    f,
                    "Metadata field '{}' value must be a string, found {}",
                    field_key, actual_type
                )
            }
            Self::ReservedFieldName { field_name } => {
                write!(
                    f,
                    "Metadata cannot use reserved field name '{}'",
                    field_name
                )
            }
            Self::FieldNameTooLong {
                field_name,
                actual_length,
                max_length,
            } => {
                write!(
                    f,
                    "Metadata field name '{}' is {} characters long, maximum allowed is {}",
                    field_name, actual_length, max_length
                )
            }
            Self::FieldValueTooLong {
                field_name,
                actual_length,
                max_length,
            } => {
                write!(
                    f,
                    "Metadata field '{}' value is {} characters long, maximum allowed is {}",
                    field_name, actual_length, max_length
                )
            }
            Self::DuplicateFieldName { field_name } => {
                write!(f, "Metadata contains duplicate field name '{}'", field_name)
            }
            Self::ValidationFailed { rule, cause } => {
                write!(
                    f,
                    "Metadata validation failed for rule '{}': {}",
                    rule, cause
                )
            }
            Self::InvalidJsonStructure { cause } => {
                write!(f, "Invalid JSON structure for metadata: {}", cause)
            }
            Self::MissingRequiredField {
                field_name,
                required_for,
            } => {
                write!(
                    f,
                    "Required metadata field '{}' is missing (required for {})",
                    field_name, required_for
                )
            }
            Self::EmptyRequiredField {
                field_name,
                required_for,
            } => {
                write!(
                    f,
                    "Required metadata field '{}' is empty (required for {})",
                    field_name, required_for
                )
            }
            Self::MultipleRequiredFieldsMissing {
                missing_fields,
                required_for,
            } => {
                write!(
                    f,
                    "Multiple required metadata fields are missing for {}: [{}]",
                    required_for,
                    missing_fields.join(", ")
                )
            }
            Self::MultipleRequiredFieldsEmpty {
                empty_fields,
                required_for,
            } => {
                write!(
                    f,
                    "Multiple required metadata fields are empty for {}: [{}]",
                    required_for,
                    empty_fields.join(", ")
                )
            }
            Self::IcsScanOutputValidationFailed {
                missing_fields,
                empty_fields,
            } => {
                let mut message = String::from("ICS scan output validation failed:");
                if !missing_fields.is_empty() {
                    message.push_str(&format!(" Missing fields: [{}]", missing_fields.join(", ")));
                }
                if !empty_fields.is_empty() {
                    message.push_str(&format!(" Empty fields: [{}]", empty_fields.join(", ")));
                }
                write!(f, "{}", message)
            }
            Self::InvalidIcsScanIdFormat {
                scan_id,
                format_requirements,
            } => {
                write!(
                    f,
                    "Invalid ics_scan_id format '{}': {}",
                    scan_id, format_requirements
                )
            }
            Self::InvalidCriticalityLevel {
                criticality,
                valid_levels,
            } => {
                write!(
                    f,
                    "Invalid criticality level '{}'. Valid levels: [{}]",
                    criticality,
                    valid_levels.join(", ")
                )
            }
            Self::InvalidTagsFormat {
                tags,
                format_requirements,
            } => {
                write!(f, "Invalid tags format '{}': {}", tags, format_requirements)
            }
            Self::InvalidControlFramework {
                framework,
                validation_error,
            } => {
                write!(
                    f,
                    "Invalid control framework '{}': {}",
                    framework, validation_error
                )
            }
            Self::InvalidControl {
                control,
                validation_error,
            } => {
                write!(f, "Invalid control '{}': {}", control, validation_error)
            }
            Self::InvalidPlatform {
                platform,
                validation_error,
            } => {
                write!(f, "Invalid platform '{}': {}", platform, validation_error)
            }
            Self::FieldValueValidationFailed {
                field_name,
                field_value,
                validation_error,
            } => {
                write!(
                    f,
                    "Field value validation failed for '{}' (value: '{}'): {}",
                    field_name, field_value, validation_error
                )
            }
        }
    }
}

impl std::error::Error for MetadataParsingError {}
