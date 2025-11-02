// parser/object/mod.rs
pub mod error;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::resolution::ResolutionError;
use crate::types::common::{DataType, Value};
use crate::types::filter::{FilterAction, FilterSpec};
use crate::types::object::{ModuleField, ObjectDeclaration, ObjectElement};
use crate::types::resolution_context::{DeferredValidation, ResolutionContext, ValidationType};
use std::collections::HashMap;

/// Enhanced context for object parsing with DAG support
#[derive(Debug, Clone)]
pub struct ObjectParsingContext {
    /// Parsing errors that are non-fatal
    pub parsing_errors: Vec<String>,
    /// Deferred validations to be processed later
    pub deferred_validations: Vec<DeferredValidation>,
    /// Variable references found during parsing
    pub variable_references: Vec<(String, String, String)>, // (source, target, context)
    /// Set references found during parsing
    pub set_references: Vec<(String, String, String)>, // (source, target, context)
    /// Filter state references found during parsing
    pub filter_state_references: Vec<(String, String, String)>, // (source, target, context)
}

impl ObjectParsingContext {
    pub fn new() -> Self {
        Self {
            parsing_errors: Vec::new(),
            deferred_validations: Vec::new(),
            variable_references: Vec::new(),
            set_references: Vec::new(),
            filter_state_references: Vec::new(),
        }
    }

    /// Add a parsing error (non-fatal)
    pub fn add_parsing_error(&mut self, error: String) {
        self.parsing_errors.push(error);
    }

    /// Add deferred variable reference validation
    pub fn defer_variable_reference(&mut self, source: String, target: String, context: String) {
        self.variable_references
            .push((source.clone(), target.clone(), context.clone()));
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::VariableReference,
            source_symbol: source,
            target_symbol: target,
            context,
        });
    }

    /// Add deferred set reference validation
    pub fn defer_set_reference(&mut self, source: String, target: String, context: String) {
        self.set_references
            .push((source.clone(), target.clone(), context.clone()));
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::SetReference,
            source_symbol: source,
            target_symbol: target,
            context,
        });
    }

    /// Add deferred filter state reference validation
    pub fn defer_filter_state_reference(
        &mut self,
        source: String,
        target: String,
        context: String,
    ) {
        self.filter_state_references
            .push((source.clone(), target.clone(), context.clone()));
        self.deferred_validations.push(DeferredValidation {
            validation_type: ValidationType::FilterStateReference,
            source_symbol: source,
            target_symbol: target,
            context,
        });
    }

    /// Apply collected data to ResolutionContext
    pub fn apply_to_resolution_context(self, resolution_context: &mut ResolutionContext) {
        // Add parsing errors
        for error in self.parsing_errors {
            resolution_context.add_parsing_error(error);
        }

        // Add deferred validations
        for validation in self.deferred_validations {
            resolution_context.add_deferred_validation(validation);
        }
    }
}

/// Extract objects from AST JSON, separating global and local objects
/// Returns (global_objects, local_objects_by_ctn_id, parsing_context)
pub fn extract_objects_from_json(
    ast_json: &serde_json::Value,
) -> Result<
    (
        Vec<ObjectDeclaration>,
        HashMap<usize, Vec<ObjectDeclaration>>,
        ObjectParsingContext,
    ),
    ResolutionError,
> {
    let _ = log_consumer_debug(
        "Starting object extraction from AST JSON",
        &[("ast_is_object", &ast_json.is_object().to_string())],
    );

    let mut parsing_context = ObjectParsingContext::new();

    let definition = ast_json.get("definition").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "No 'definition' key found in AST JSON",
            &[],
        );
        ResolutionError::InvalidInput {
            message: "No definition found in AST".to_string(),
        }
    })?;

    // Extract global objects from definition.objects
    let global_objects = extract_global_objects(definition, &mut parsing_context)?;

    // Extract local objects from criteria CTN blocks
    let local_objects = extract_local_objects_from_criteria(definition, &mut parsing_context)?;

    let _ = log_consumer_debug(
        "Object extraction completed",
        &[
            ("global_objects", &global_objects.len().to_string()),
            ("local_ctn_count", &local_objects.len().to_string()),
            (
                "total_local_objects",
                &local_objects
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>()
                    .to_string(),
            ),
            (
                "parsing_errors",
                &parsing_context.parsing_errors.len().to_string(),
            ),
            (
                "deferred_validations",
                &parsing_context.deferred_validations.len().to_string(),
            ),
        ],
    );

    Ok((global_objects, local_objects, parsing_context))
}

/// Extract global objects from definition.objects array
fn extract_global_objects(
    definition: &serde_json::Value,
    parsing_context: &mut ObjectParsingContext,
) -> Result<Vec<ObjectDeclaration>, ResolutionError> {
    let _ = log_consumer_debug("Extracting global objects from definition", &[]);

    let empty_vec = Vec::new();
    let objects_array = definition
        .get("objects")
        .and_then(|objs| objs.as_array())
        .unwrap_or(&empty_vec); // No objects is valid, return empty vec

    let _ = log_consumer_debug(
        "Found global objects array",
        &[("object_count", &objects_array.len().to_string())],
    );

    let mut global_objects = Vec::new();

    for (index, obj_json) in objects_array.iter().enumerate() {
        let _ = log_consumer_debug("Processing global object", &[("index", &index.to_string())]);

        match parse_object_from_json(obj_json, true, parsing_context) {
            Ok(object) => {
                let _ = log_consumer_debug(
                    "Successfully parsed global object",
                    &[
                        ("object_id", &object.identifier),
                        ("element_count", &object.elements.len().to_string()),
                        (
                            "has_variable_refs",
                            &object.has_variable_references().to_string(),
                        ),
                    ],
                );
                global_objects.push(object);
            }
            Err(e) => {
                let error_msg =
                    format!("Failed to parse global object at index {}: {:?}", index, e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[("index", &index.to_string())],
                );

                // For DAG resolution, we could continue parsing and collect this as a non-fatal error
                // But for now, we'll still return the error to maintain current behavior
                return Err(e);
            }
        }
    }

    let _ = log_consumer_debug(
        "Global object extraction completed",
        &[("total_extracted", &global_objects.len().to_string())],
    );

    Ok(global_objects)
}

/// Extract local objects from criteria blocks, organizing by CTN node ID
fn extract_local_objects_from_criteria(
    definition: &serde_json::Value,
    parsing_context: &mut ObjectParsingContext,
) -> Result<HashMap<usize, Vec<ObjectDeclaration>>, ResolutionError> {
    let _ = log_consumer_debug("Extracting local objects from criteria", &[]);

    let empty_vec = Vec::new();
    let criteria_array = definition
        .get("criteria")
        .and_then(|crit| crit.as_array())
        .unwrap_or(&empty_vec); // No criteria is valid for this extraction

    let mut local_objects: HashMap<usize, Vec<ObjectDeclaration>> = HashMap::new();
    let mut ctn_node_id_counter = 0usize;

    for (criteria_index, criteria_json) in criteria_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing criteria block",
            &[("criteria_index", &criteria_index.to_string())],
        );

        if let Some(content_array) = criteria_json.get("content").and_then(|c| c.as_array()) {
            for (content_index, content_json) in content_array.iter().enumerate() {
                if let Some(criterion_json) = content_json.get("Criterion") {
                    let ctn_type = criterion_json
                        .get("criterion_type")
                        .and_then(|ct| ct.as_str())
                        .unwrap_or("unknown");

                    let _ = log_consumer_debug(
                        "Processing criterion",
                        &[
                            ("ctn_node_id", &ctn_node_id_counter.to_string()),
                            ("ctn_type", ctn_type),
                            ("content_index", &content_index.to_string()),
                        ],
                    );

                    // Check for local_object (singular)
                    if let Some(local_object_json) = criterion_json.get("local_object") {
                        if !local_object_json.is_null() {
                            let _ = log_consumer_debug(
                                "Found local object in CTN",
                                &[
                                    ("ctn_node_id", &ctn_node_id_counter.to_string()),
                                    ("ctn_type", ctn_type),
                                ],
                            );

                            match parse_object_from_json(local_object_json, false, parsing_context)
                            {
                                Ok(object) => {
                                    let _ = log_consumer_debug(
                                        "Successfully parsed local object",
                                        &[
                                            ("object_id", &object.identifier),
                                            ("ctn_node_id", &ctn_node_id_counter.to_string()),
                                            (
                                                "has_variable_refs",
                                                &object.has_variable_references().to_string(),
                                            ),
                                        ],
                                    );

                                    local_objects
                                        .entry(ctn_node_id_counter)
                                        .or_insert_with(Vec::new)
                                        .push(object);
                                }
                                Err(e) => {
                                    let error_msg = format!(
                                        "Failed to parse local object in CTN {}: {:?}",
                                        ctn_node_id_counter, e
                                    );
                                    let _ = log_consumer_error(
                                        consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                                        &error_msg,
                                        &[
                                            ("ctn_node_id", &ctn_node_id_counter.to_string()),
                                            ("ctn_type", ctn_type),
                                        ],
                                    );

                                    // For DAG resolution, continue parsing and collect as non-fatal error
                                    parsing_context.add_parsing_error(error_msg);
                                }
                            }
                        }
                    }

                    ctn_node_id_counter += 1;
                }
            }
        }
    }

    let _ = log_consumer_debug(
        "Local object extraction completed",
        &[
            ("ctn_count", &local_objects.len().to_string()),
            (
                "total_local_objects",
                &local_objects
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>()
                    .to_string(),
            ),
        ],
    );

    Ok(local_objects)
}

/// Parse a single object from JSON with DAG-aware context collection
pub fn parse_object_from_json(
    obj_json: &serde_json::Value,
    is_global: bool,
    parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectDeclaration, ResolutionError> {
    let identifier = obj_json
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Object missing 'id' field",
                &[("is_global", &is_global.to_string())],
            );
            ResolutionError::InvalidInput {
                message: "Object missing id".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Parsing object",
        &[
            ("object_id", identifier),
            ("is_global", &is_global.to_string()),
        ],
    );

    let elements_array = obj_json
        .get("elements")
        .and_then(|elements| elements.as_array())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Object '{}' missing 'elements' array", identifier),
                &[("object_id", identifier)],
            );
            ResolutionError::InvalidInput {
                message: "Object missing elements".to_string(),
            }
        })?;

    if elements_array.is_empty() {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Object '{}' has no elements (empty objects not allowed)",
                identifier
            ),
            &[("object_id", identifier)],
        );
        return Err(ResolutionError::InvalidInput {
            message: format!("Object '{}' cannot be empty", identifier),
        });
    }

    let _ = log_consumer_debug(
        "Parsing object elements",
        &[
            ("object_id", identifier),
            ("element_count", &elements_array.len().to_string()),
        ],
    );

    let mut elements = Vec::new();

    for (element_index, element_json) in elements_array.iter().enumerate() {
        let _ = log_consumer_debug(
            "Processing object element",
            &[
                ("object_id", identifier),
                ("element_index", &element_index.to_string()),
            ],
        );

        match parse_object_element_from_json(element_json, identifier, parsing_context) {
            Ok(element) => {
                let _ = log_consumer_debug(
                    "Successfully parsed object element",
                    &[
                        ("object_id", identifier),
                        ("element_index", &element_index.to_string()),
                        ("element_type", element.element_type_name()),
                    ],
                );
                elements.push(element);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to parse element {} in object '{}': {:?}",
                    element_index, identifier, e
                );
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[
                        ("object_id", identifier),
                        ("element_index", &element_index.to_string()),
                    ],
                );

                // For DAG resolution, continue parsing and collect as non-fatal error
                parsing_context.add_parsing_error(error_msg);
            }
        }
    }

    let _ = log_consumer_debug(
        "Object parsing completed",
        &[
            ("object_id", identifier),
            ("element_count", &elements.len().to_string()),
            ("is_global", &is_global.to_string()),
        ],
    );

    Ok(ObjectDeclaration {
        identifier: identifier.to_string(),
        elements,
        is_global,
    })
}

/// Parse a single object element from JSON using tagged union format
fn parse_object_element_from_json(
    element_json: &serde_json::Value,
    object_id: &str,
    parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let _ = log_consumer_debug(
        "Parsing object element",
        &[
            ("object_id", object_id),
            (
                "element_keys",
                &element_json
                    .as_object()
                    .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_else(|| "none".to_string()),
            ),
        ],
    );

    // Handle Field element
    if let Some(field_json) = element_json.get("Field") {
        let _ = log_consumer_debug("Parsing Field element", &[("object_id", object_id)]);
        return parse_field_element(field_json, object_id, parsing_context);
    }

    // Handle Parameter element
    if let Some(parameter_json) = element_json.get("Parameter") {
        let _ = log_consumer_debug("Parsing Parameter element", &[("object_id", object_id)]);
        return parse_parameter_element(parameter_json, object_id, parsing_context);
    }

    // Handle Select element
    if let Some(select_json) = element_json.get("Select") {
        let _ = log_consumer_debug("Parsing Select element", &[("object_id", object_id)]);
        return parse_select_element(select_json, object_id, parsing_context);
    }

    // Handle Behavior element
    if let Some(behavior_json) = element_json.get("Behavior") {
        let _ = log_consumer_debug("Parsing Behavior element", &[("object_id", object_id)]);
        return parse_behavior_element(behavior_json, object_id, parsing_context);
    }

    // Handle Filter element (now properly implemented)
    if let Some(filter_json) = element_json.get("Filter") {
        let _ = log_consumer_debug("Parsing Filter element", &[("object_id", object_id)]);
        return parse_filter_element(filter_json, object_id, parsing_context);
    }

    // Handle SetRef element
    if let Some(setref_json) = element_json.get("SetRef") {
        let _ = log_consumer_debug("Parsing SetRef element", &[("object_id", object_id)]);
        return parse_setref_element(setref_json, object_id, parsing_context);
    }

    // Handle Module element
    if let Some(module_json) = element_json.get("Module") {
        let _ = log_consumer_debug("Parsing Module element", &[("object_id", object_id)]);
        return parse_module_element(module_json, object_id, parsing_context);
    }

    // Unknown element type
    let available_keys = element_json
        .as_object()
        .map(|obj| obj.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_else(Vec::new);

    let error_msg = format!("Unknown object element type in object '{}'", object_id);
    let _ = log_consumer_error(
        consumer_codes::CONSUMER_FORMAT_ERROR,
        &error_msg,
        &[
            ("object_id", object_id),
            ("available_keys", &available_keys.join(",")),
        ],
    );

    Err(ResolutionError::InvalidInput { message: error_msg })
}

/// Parse Field element with variable reference tracking
fn parse_field_element(
    field_json: &serde_json::Value,
    object_id: &str,
    parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let name = field_json
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Field element missing 'name' in object '{}'", object_id),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "Field missing name".to_string(),
            }
        })?;

    let value_json = field_json.get("value").ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            &format!("Field '{}' missing 'value' in object '{}'", name, object_id),
            &[("object_id", object_id), ("field_name", name)],
        );
        ResolutionError::InvalidInput {
            message: "Field missing value".to_string(),
        }
    })?;

    let value = parse_value_from_json(value_json, object_id, name, parsing_context)?;

    let _ = log_consumer_debug(
        "Parsed Field element",
        &[
            ("object_id", object_id),
            ("field_name", name),
            (
                "has_variable_ref",
                &value.has_variable_reference().to_string(),
            ),
        ],
    );

    Ok(ObjectElement::Field {
        name: name.to_string(),
        value,
    })
}

/// Parse Parameter element
fn parse_parameter_element(
    parameter_json: &serde_json::Value,
    object_id: &str,
    _parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let data_type_str = parameter_json
        .get("data_type")
        .and_then(|dt| dt.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Parameter element missing 'data_type' in object '{}'",
                    object_id
                ),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "Parameter missing data_type".to_string(),
            }
        })?;

    let data_type = DataType::from_str(data_type_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid data type '{}' for Parameter in object '{}'",
                data_type_str, object_id
            ),
            &[("object_id", object_id), ("data_type", data_type_str)],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid data type: {}", data_type_str),
        }
    })?;

    let empty_vec = Vec::new();
    let fields_array = parameter_json
        .get("fields")
        .and_then(|f| f.as_array())
        .unwrap_or(&empty_vec);

    let mut fields = Vec::new();
    for field_pair in fields_array {
        if let Some(pair_array) = field_pair.as_array() {
            if pair_array.len() == 2 {
                if let (Some(key), Some(value)) = (pair_array[0].as_str(), pair_array[1].as_str()) {
                    fields.push((key.to_string(), value.to_string()));
                }
            }
        }
    }

    let _ = log_consumer_debug(
        "Parsed Parameter element",
        &[
            ("object_id", object_id),
            ("data_type", data_type_str),
            ("field_count", &fields.len().to_string()),
        ],
    );

    Ok(ObjectElement::Parameter { data_type, fields })
}

/// Parse Select element
fn parse_select_element(
    select_json: &serde_json::Value,
    object_id: &str,
    _parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let data_type_str = select_json
        .get("data_type")
        .and_then(|dt| dt.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Select element missing 'data_type' in object '{}'",
                    object_id
                ),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "Select missing data_type".to_string(),
            }
        })?;

    let data_type = DataType::from_str(data_type_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid data type '{}' for Select in object '{}'",
                data_type_str, object_id
            ),
            &[("object_id", object_id), ("data_type", data_type_str)],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid data type: {}", data_type_str),
        }
    })?;

    let empty_vec = Vec::new();
    let fields_array = select_json
        .get("fields")
        .and_then(|f| f.as_array())
        .unwrap_or(&empty_vec);

    let mut fields = Vec::new();
    for field_pair in fields_array {
        if let Some(pair_array) = field_pair.as_array() {
            if pair_array.len() == 2 {
                if let (Some(key), Some(value)) = (pair_array[0].as_str(), pair_array[1].as_str()) {
                    fields.push((key.to_string(), value.to_string()));
                }
            }
        }
    }

    let _ = log_consumer_debug(
        "Parsed Select element",
        &[
            ("object_id", object_id),
            ("data_type", data_type_str),
            ("field_count", &fields.len().to_string()),
        ],
    );

    Ok(ObjectElement::Select { data_type, fields })
}

/// Parse Behavior element
fn parse_behavior_element(
    behavior_json: &serde_json::Value,
    object_id: &str,
    _parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let values_array = behavior_json
        .get("values")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!(
                    "Behavior element missing 'values' array in object '{}'",
                    object_id
                ),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "Behavior missing values".to_string(),
            }
        })?;

    let mut values = Vec::new();
    for value_json in values_array {
        if let Some(value_str) = value_json.as_str() {
            values.push(value_str.to_string());
        }
    }

    let _ = log_consumer_debug(
        "Parsed Behavior element",
        &[
            ("object_id", object_id),
            ("value_count", &values.len().to_string()),
        ],
    );

    Ok(ObjectElement::Behavior { values })
}

/// Parse Filter element (now properly implemented with state reference collection)
fn parse_filter_element(
    filter_json: &serde_json::Value,
    object_id: &str,
    parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let _ = log_consumer_debug("Parsing Filter element", &[("object_id", object_id)]);

    // Parse filter action (default to Include if missing)
    let action_str = filter_json
        .get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("Include");

    let action = FilterAction::from_str(action_str).unwrap_or_else(|| {
        let error_msg = format!(
            "Invalid filter action '{}' in object '{}', defaulting to Include",
            action_str, object_id
        );
        parsing_context.add_parsing_error(error_msg);
        FilterAction::Include
    });

    // Parse state references
    let mut state_refs = Vec::new();
    if let Some(state_refs_array) = filter_json.get("state_refs").and_then(|sr| sr.as_array()) {
        for state_ref_json in state_refs_array {
            if let Some(state_ref_obj) = state_ref_json.as_object() {
                if let Some(state_id) = state_ref_obj.get("state_id").and_then(|id| id.as_str()) {
                    let _ = log_consumer_debug(
                        "Found filter state reference",
                        &[("object_id", object_id), ("state_id", state_id)],
                    );

                    // DEFER state reference validation - don't check if state exists now
                    parsing_context.defer_filter_state_reference(
                        object_id.to_string(),
                        state_id.to_string(),
                        format!("filter in object '{}'", object_id),
                    );

                    state_refs.push(state_id.to_string());
                }
            }
        }
    }

    let filter_spec = FilterSpec { action, state_refs };

    let _ = log_consumer_debug(
        "Parsed Filter element",
        &[
            ("object_id", object_id),
            ("action", action.as_str()),
            ("state_ref_count", &filter_spec.state_refs.len().to_string()),
        ],
    );

    Ok(ObjectElement::Filter(filter_spec))
}

/// Parse SetRef element with deferred validation
fn parse_setref_element(
    setref_json: &serde_json::Value,
    object_id: &str,
    parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let set_id = setref_json
        .get("set_id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("SetRef element missing 'set_id' in object '{}'", object_id),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "SetRef missing set_id".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Parsed SetRef element",
        &[("object_id", object_id), ("set_id", set_id)],
    );

    // DEFER set reference validation - don't check if set exists now
    parsing_context.defer_set_reference(
        object_id.to_string(),
        set_id.to_string(),
        format!("SetRef in object '{}'", object_id),
    );

    Ok(ObjectElement::SetRef {
        set_id: set_id.to_string(),
    })
}

/// Parse Module element
fn parse_module_element(
    module_json: &serde_json::Value,
    object_id: &str,
    _parsing_context: &mut ObjectParsingContext,
) -> Result<ObjectElement, ResolutionError> {
    let field_str = module_json
        .get("field")
        .and_then(|f| f.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Module element missing 'field' in object '{}'", object_id),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "Module missing field".to_string(),
            }
        })?;

    let module_field = ModuleField::from_str(field_str).ok_or_else(|| {
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
            &format!(
                "Invalid module field '{}' in object '{}'",
                field_str, object_id
            ),
            &[("object_id", object_id), ("field", field_str)],
        );
        ResolutionError::InvalidInput {
            message: format!("Invalid module field: {}", field_str),
        }
    })?;

    let value = module_json
        .get("value")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                &format!("Module element missing 'value' in object '{}'", object_id),
                &[("object_id", object_id)],
            );
            ResolutionError::InvalidInput {
                message: "Module missing value".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Parsed Module element",
        &[
            ("object_id", object_id),
            ("field", field_str),
            ("value", value),
        ],
    );

    Ok(ObjectElement::Module {
        field: module_field,
        value: value.to_string(),
    })
}

/// Parse Value enum from JSON with variable reference tracking
fn parse_value_from_json(
    value_json: &serde_json::Value,
    object_id: &str,
    field_name: &str,
    parsing_context: &mut ObjectParsingContext,
) -> Result<Value, ResolutionError> {
    if let Some(string_val) = value_json.get("String").and_then(|s| s.as_str()) {
        Ok(Value::String(string_val.to_string()))
    } else if let Some(int_val) = value_json.get("Integer").and_then(|i| i.as_i64()) {
        Ok(Value::Integer(int_val))
    } else if let Some(float_val) = value_json.get("Float").and_then(|f| f.as_f64()) {
        Ok(Value::Float(float_val))
    } else if let Some(bool_val) = value_json.get("Boolean").and_then(|b| b.as_bool()) {
        Ok(Value::Boolean(bool_val))
    } else if let Some(var_name) = value_json.get("Variable").and_then(|v| v.as_str()) {
        let _ = log_consumer_debug(
            "Found variable reference in object field",
            &[
                ("object_id", object_id),
                ("field_name", field_name),
                ("variable_name", var_name),
            ],
        );

        // DEFER variable reference validation - don't check if variable exists now
        parsing_context.defer_variable_reference(
            object_id.to_string(),
            var_name.to_string(),
            format!("field '{}' in object '{}'", field_name, object_id),
        );

        Ok(Value::Variable(var_name.to_string()))
    } else {
        let available_keys = value_json
            .as_object()
            .map(|obj| obj.keys().cloned().collect::<Vec<_>>().join(","))
            .unwrap_or_else(|| "none".to_string());

        let _ = log_consumer_error(
            consumer_codes::CONSUMER_FORMAT_ERROR,
            "Unknown value type in JSON",
            &[
                ("object_id", object_id),
                ("field_name", field_name),
                ("available_keys", &available_keys),
            ],
        );
        Err(ResolutionError::InvalidInput {
            message: "Unknown value type in JSON".to_string(),
        })
    }
}

/// Check if an object has variable references in any of its elements (helper for existing code)
pub fn has_variable_references_in_object(object: &ObjectDeclaration) -> bool {
    object.has_variable_references()
}

/// Apply object parsing results to ResolutionContext
pub fn apply_object_parsing_to_context(
    global_objects: Vec<ObjectDeclaration>,
    local_objects: HashMap<usize, Vec<ObjectDeclaration>>,
    parsing_context: ObjectParsingContext,
    resolution_context: &mut ResolutionContext,
) {
    // Add global objects to resolution context
    resolution_context.global_objects = global_objects;

    // Add local objects to resolution context
    for (ctn_node_id, objects) in local_objects {
        // For now, take the first object (ICS allows max 1 local object per CTN)
        if let Some(object) = objects.into_iter().next() {
            resolution_context
                .ctn_local_objects
                .insert(ctn_node_id, object);
        }
    }

    // Apply parsing context (errors and deferred validations)
    parsing_context.apply_to_resolution_context(resolution_context);
}
