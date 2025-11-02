// parser/metadata/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::resolution::ResolutionError;
use crate::types::metadata::MetaDataBlock;
use std::collections::HashMap;

/// Required fields for ICS scan output generation
const REQUIRED_FIELDS: &[&str] = &[
    "ics_scan_id",
    "control_framework",
    "control",
    "platform",
    "criticality",
    "tags",
];

/// Extract metadata from AST JSON - REQUIRED for ICS definitions
/// Metadata is a sibling to the definition object in the JSON structure
pub fn extract_metadata_from_json(
    ast_json: &serde_json::Value,
) -> Result<MetaDataBlock, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting metadata extraction from AST JSON",
        &[("ast_is_object", &ast_json.is_object().to_string())],
    );

    // Metadata must exist as a sibling to definition at root level
    let metadata_json = ast_json.get("metadata").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            "No metadata found in AST - metadata is required for ICS definitions",
            &[(
                "available_keys",
                &ast_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            )],
        );
        ResolutionError::InvalidInput {
            message: "ICS definition missing required metadata block".to_string(),
        }
    })?;

    // Metadata cannot be null
    if metadata_json.is_null() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            "Metadata is null - metadata is required for ICS definitions",
            &[],
        );
        return Err(ResolutionError::InvalidInput {
            message: "ICS definition has null metadata block".to_string(),
        });
    }

    let _ = log_consumer_debug(
        "Found metadata in AST root, parsing and validating...",
        &[("metadata_is_object", &metadata_json.is_object().to_string())],
    );

    // Parse metadata structure from JSON
    let mut metadata_block = parse_metadata_from_json(metadata_json)?;

    // Validate mandatory fields for ICS scan output
    validate_mandatory_fields(&metadata_block)?;

    // Validate field values for ICS scan output
    validate_ics_scan_field_values(&metadata_block)?;

    let _ = log_consumer_debug(
        "Metadata extraction and validation completed successfully",
        &[("field_count", &metadata_block.fields.len().to_string())],
    );

    Ok(metadata_block)
}

/// Parse metadata block from JSON (handles array-based field structure from AST)
pub fn parse_metadata_from_json(
    metadata_json: &serde_json::Value,
) -> Result<MetaDataBlock, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing metadata block structure",
        &[("metadata_is_object", &metadata_json.is_object().to_string())],
    );

    // Metadata must be an object
    if !metadata_json.is_object() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "Metadata must be a JSON object",
            &[("actual_type", &get_json_type_name(metadata_json))],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Metadata must be an object, found {}",
                get_json_type_name(metadata_json)
            ),
        });
    }

    let fields_array = metadata_json.get("fields").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "Metadata missing 'fields' array",
            &[(
                "available_keys",
                &metadata_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            )],
        );
        ResolutionError::InvalidInput {
            message: "Metadata missing fields array".to_string(),
        }
    })?;

    if !fields_array.is_array() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "Metadata 'fields' must be an array",
            &[("actual_type", &get_json_type_name(fields_array))],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!(
                "Metadata fields must be an array, found {}",
                get_json_type_name(fields_array)
            ),
        });
    }

    let fields_vec = fields_array.as_array().unwrap();
    let mut fields = HashMap::new();

    let _ = log_consumer_debug(
        "Processing metadata fields from array",
        &[("field_count", &fields_vec.len().to_string())],
    );

    for (index, field_obj) in fields_vec.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing metadata field",
            &[("field_index", &index.to_string())],
        );

        // Each field must be an object
        if !field_obj.is_object() {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!("Metadata field at index {} must be an object", index),
                &[
                    ("field_index", &index.to_string()),
                    ("actual_type", &get_json_type_name(field_obj)),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Metadata field at index {} must be an object", index),
            });
        }

        let name = field_obj
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!("Metadata field at index {} missing 'name' string", index),
                    &[("field_index", &index.to_string())],
                );
                ResolutionError::InvalidInput {
                    message: format!("Metadata field at index {} missing 'name'", index),
                }
            })?;

        let value = field_obj
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!("Metadata field '{}' missing 'value' string", name),
                    &[("field_name", name), ("field_index", &index.to_string())],
                );
                ResolutionError::InvalidInput {
                    message: format!("Metadata field '{}' missing 'value'", name),
                }
            })?;

        // Validate that the name is not empty or whitespace only
        if name.trim().is_empty() {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                "Metadata field name cannot be empty or whitespace only",
                &[("field_name", name), ("field_index", &index.to_string())],
            );
            return Err(ResolutionError::InvalidInput {
                message: "Metadata field name cannot be empty or whitespace only".to_string(),
            });
        }

        // Check for duplicate field names
        if fields.contains_key(name) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!("Duplicate metadata field name: '{}'", name),
                &[("field_name", name), ("field_index", &index.to_string())],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Duplicate metadata field name: '{}'", name),
            });
        }

        let _ = log_consumer_debug(
            "Successfully parsed metadata field",
            &[
                ("field_name", name),
                ("value_length", &value.len().to_string()),
                ("field_index", &index.to_string()),
            ],
        );

        fields.insert(name.to_string(), value.to_string());
    }

    // Validate minimum field count
    if fields.is_empty() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            "Metadata must contain at least one field",
            &[("fields_array_length", &fields_vec.len().to_string())],
        );
        return Err(ResolutionError::InvalidInput {
            message: "Metadata must contain at least one field".to_string(),
        });
    }

    let _ = log_consumer_debug(
        "Metadata parsing completed successfully",
        &[("total_fields_parsed", &fields.len().to_string())],
    );

    Ok(MetaDataBlock { fields })
}

/// Validate mandatory fields required for ICS scan output generation
pub fn validate_mandatory_fields(metadata: &MetaDataBlock) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Starting mandatory field validation for ICS scan output",
        &[
            ("total_fields", &metadata.fields.len().to_string()),
            ("required_fields", &REQUIRED_FIELDS.len().to_string()),
        ],
    );

    let mut missing_fields = Vec::new();
    let mut empty_fields = Vec::new();

    for &required_field in REQUIRED_FIELDS {
        let _ = log_consumer_debug("Checking required field", &[("field_name", required_field)]);

        match metadata.fields.get(required_field) {
            Some(value) => {
                // Field exists, check if it has meaningful content
                if value.trim().is_empty() {
                    let _ = log_consumer_error(
                        consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                        &format!("Required metadata field '{}' is empty", required_field),
                        &[("field_name", required_field)],
                    );
                    empty_fields.push(required_field.to_string());
                } else {
                    let _ = log_consumer_debug(
                        "Required field validation passed",
                        &[
                            ("field_name", required_field),
                            ("value_length", &value.len().to_string()),
                        ],
                    );
                }
            }
            None => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &format!("Required metadata field '{}' is missing", required_field),
                    &[("field_name", required_field)],
                );
                missing_fields.push(required_field.to_string());
            }
        }
    }

    // Report all validation failures at once
    if !missing_fields.is_empty() || !empty_fields.is_empty() {
        let mut error_message = String::from("ICS scan output validation failed:");

        if !missing_fields.is_empty() {
            error_message.push_str(&format!(
                " Missing required fields: [{}]",
                missing_fields.join(", ")
            ));
        }

        if !empty_fields.is_empty() {
            error_message.push_str(&format!(
                " Empty required fields: [{}]",
                empty_fields.join(", ")
            ));
        }

        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &error_message,
            &[
                ("missing_count", &missing_fields.len().to_string()),
                ("empty_count", &empty_fields.len().to_string()),
            ],
        );

        return Err(ResolutionError::InvalidInput {
            message: error_message,
        });
    }

    let _ = log_consumer_debug(
        "Mandatory field validation completed successfully",
        &[("validated_fields", &REQUIRED_FIELDS.len().to_string())],
    );

    Ok(())
}

/// Validate specific field values for ICS scan output (additional validation beyond existence)
pub fn validate_ics_scan_field_values(metadata: &MetaDataBlock) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Starting ICS scan field value validation",
        &[("total_fields", &metadata.fields.len().to_string())],
    );

    // Validate ics_scan_id format (should be UUID or similar unique identifier)
    if let Some(scan_id) = metadata.fields.get("ics_scan_id") {
        if scan_id.len() < 8 {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid ics_scan_id format: '{}' (must be at least 8 characters)",
                    scan_id
                ),
                &[
                    ("ics_scan_id", scan_id),
                    ("length", &scan_id.len().to_string()),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Invalid ics_scan_id format: '{}' (too short)", scan_id),
            });
        }

        if scan_id.contains(' ') {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid ics_scan_id format: '{}' (cannot contain spaces)",
                    scan_id
                ),
                &[("ics_scan_id", scan_id)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Invalid ics_scan_id format: '{}' (spaces not allowed)",
                    scan_id
                ),
            });
        }

        let _ = log_consumer_debug("ics_scan_id validation passed", &[("ics_scan_id", scan_id)]);
    }

    // Validate criticality values (should be standard severity levels)
    if let Some(criticality) = metadata.fields.get("criticality") {
        let valid_criticality_levels = ["low", "medium", "high", "critical"];
        let criticality_lower = criticality.to_lowercase();

        if !valid_criticality_levels.contains(&criticality_lower.as_str()) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Invalid criticality value: '{}' (must be one of: {})",
                    criticality,
                    valid_criticality_levels.join(", ")
                ),
                &[("criticality", criticality)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Invalid criticality value: '{}' (valid: {})",
                    criticality,
                    valid_criticality_levels.join(", ")
                ),
            });
        }

        let _ = log_consumer_debug(
            "criticality validation passed",
            &[("criticality", criticality)],
        );
    }

    // Validate control_framework is not empty beyond whitespace
    if let Some(framework) = metadata.fields.get("control_framework") {
        if framework.trim().len() < 2 {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Control framework too short: '{}' (must be at least 2 meaningful characters)",
                    framework
                ),
                &[("control_framework", framework)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Control framework too short: '{}'", framework),
            });
        }

        let _ = log_consumer_debug(
            "control_framework validation passed",
            &[("control_framework", framework)],
        );
    }

    // Validate control field has meaningful content
    if let Some(control) = metadata.fields.get("control") {
        if control.trim().len() < 2 {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Control identifier too short: '{}' (must be at least 2 meaningful characters)",
                    control
                ),
                &[("control", control)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Control identifier too short: '{}'", control),
            });
        }

        let _ = log_consumer_debug("control validation passed", &[("control", control)]);
    }

    // Validate platform field has meaningful content
    if let Some(platform) = metadata.fields.get("platform") {
        if platform.trim().len() < 2 {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!("Platform identifier too short: '{}' (must be at least 2 meaningful characters)", platform),
                &[("platform", platform)]
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Platform identifier too short: '{}'", platform),
            });
        }

        let _ = log_consumer_debug("platform validation passed", &[("platform", platform)]);
    }

    // Validate tags format (should contain meaningful tag data)
    if let Some(tags) = metadata.fields.get("tags") {
        if tags.trim().len() < 2 {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Tags field too short: '{}' (should contain meaningful tag data)",
                    tags
                ),
                &[("tags", tags)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!("Tags field too short: '{}'", tags),
            });
        }

        let _ = log_consumer_debug("tags validation passed", &[("tags", tags)]);
    }

    let _ = log_consumer_debug(
        "ICS scan field value validation completed successfully",
        &[],
    );

    Ok(())
}

/// Get the list of required fields for ICS scan output
pub fn get_required_fields() -> &'static [&'static str] {
    REQUIRED_FIELDS
}

/// Check if metadata contains any specific field
pub fn metadata_has_field(metadata: &MetaDataBlock, field_name: &str) -> bool {
    let has_field = metadata.fields.contains_key(field_name);

    let _ = log_consumer_debug(
        "Checking metadata for field",
        &[
            ("field_name", field_name),
            ("has_field", &has_field.to_string()),
        ],
    );

    has_field
}

/// Get a field value from metadata, returning None if not present
pub fn get_metadata_field<'a>(metadata: &'a MetaDataBlock, field_name: &str) -> Option<&'a String> {
    let field_value = metadata.fields.get(field_name);

    let _ = log_consumer_debug(
        "Getting metadata field value",
        &[
            ("field_name", field_name),
            ("found", &field_value.is_some().to_string()),
        ],
    );

    field_value
}

/// Get all field names from metadata
pub fn get_metadata_field_names(metadata: &MetaDataBlock) -> Vec<String> {
    let field_names: Vec<String> = metadata.fields.keys().cloned().collect();

    let _ = log_consumer_debug(
        "Getting all metadata field names",
        &[("field_count", &field_names.len().to_string())],
    );

    field_names
}

/// Validate metadata block according to business rules and constraints
pub fn validate_metadata(metadata: &MetaDataBlock) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating metadata block according to business rules",
        &[("field_count", &metadata.fields.len().to_string())],
    );

    // Check for any reserved field names that might conflict with system fields
    let reserved_fields = ["id", "type", "version", "schema"]; // Example reserved fields

    for reserved_field in &reserved_fields {
        if metadata.fields.contains_key(*reserved_field) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Metadata cannot use reserved field name '{}'",
                    reserved_field
                ),
                &[("reserved_field", reserved_field)],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Reserved field name '{}' not allowed in metadata",
                    reserved_field
                ),
            });
        }
    }

    // Check for excessively long field names or values
    const MAX_FIELD_NAME_LENGTH: usize = 100;
    const MAX_FIELD_VALUE_LENGTH: usize = 1000;

    for (key, value) in &metadata.fields {
        if key.len() > MAX_FIELD_NAME_LENGTH {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Metadata field name '{}' exceeds maximum length of {}",
                    key, MAX_FIELD_NAME_LENGTH
                ),
                &[
                    ("field_name", key),
                    ("actual_length", &key.len().to_string()),
                    ("max_length", &MAX_FIELD_NAME_LENGTH.to_string()),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Field name '{}' too long (max {} chars)",
                    key, MAX_FIELD_NAME_LENGTH
                ),
            });
        }

        if value.len() > MAX_FIELD_VALUE_LENGTH {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                &format!(
                    "Metadata field '{}' value exceeds maximum length of {}",
                    key, MAX_FIELD_VALUE_LENGTH
                ),
                &[
                    ("field_name", key),
                    ("actual_length", &value.len().to_string()),
                    ("max_length", &MAX_FIELD_VALUE_LENGTH.to_string()),
                ],
            );
            return Err(ResolutionError::InvalidInput {
                message: format!(
                    "Field '{}' value too long (max {} chars)",
                    key, MAX_FIELD_VALUE_LENGTH
                ),
            });
        }
    }

    let _ = log_consumer_debug(
        "Metadata business rule validation completed successfully",
        &[("field_count", &metadata.fields.len().to_string())],
    );

    Ok(())
}

/// Helper function to get JSON value type name for better error messages
fn get_json_type_name(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Array(_) => "array".to_string(),
        serde_json::Value::Object(_) => "object".to_string(),
    }
}
