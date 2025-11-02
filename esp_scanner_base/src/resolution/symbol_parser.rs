//! Symbol parser for converting JSON symbol table relationships to internal DAG types
//! Handles parsing of symbol relationships from pipeline output for dependency graph construction

use crate::ffi::logging::{
    consumer_codes, log_consumer_debug, log_consumer_error, log_consumer_warning,
};
use crate::resolution::error::ResolutionError;
use crate::types::criterion::CtnNodeId;
use crate::types::resolution_context::{RelationshipType, SymbolRelationship};
use serde_json::Value;
use std::collections::HashMap;

/// Parse symbol relationships from JSON symbol table
pub fn parse_relationships_from_json(
    symbols_json: &Value,
) -> Result<Vec<SymbolRelationship>, ResolutionError> {
    let _ = log_consumer_debug(
        "Starting JSON symbol relationships parsing",
        &[("has_symbols_data", &(!symbols_json.is_null()).to_string())],
    );

    let relationships_array = symbols_json
        .get("relationships")
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Missing 'relationships' field in symbols JSON",
                &[],
            );
            ResolutionError::InvalidInput {
                message: "Symbol table missing relationships array".to_string(),
            }
        })?
        .as_array()
        .ok_or_else(|| {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_FORMAT_ERROR,
                "Relationships field is not an array",
                &[],
            );
            ResolutionError::InvalidInput {
                message: "Relationships must be an array".to_string(),
            }
        })?;

    let _ = log_consumer_debug(
        "Found relationships array",
        &[("relationship_count", &relationships_array.len().to_string())],
    );

    let mut parsed_relationships = Vec::new();
    let mut parse_errors = Vec::new();

    for (index, relationship_json) in relationships_array.iter().enumerate() {
        match parse_single_relationship(relationship_json, index) {
            Ok(relationship) => {
                let _ = log_consumer_debug(
                    "Successfully parsed relationship",
                    &[
                        ("index", &index.to_string()),
                        ("source", &relationship.source),
                        ("target", &relationship.target),
                        ("type", relationship.relationship_type.as_str()),
                    ],
                );
                parsed_relationships.push(relationship);
            }
            Err(error) => {
                let _ = log_consumer_warning(
                    &format!("Failed to parse relationship at index {}: {}", index, error),
                    &[("index", &index.to_string()), ("error", &error.to_string())],
                );
                parse_errors.push((index, error));
            }
        }
    }

    // Log parsing summary
    let _ = log_consumer_debug(
        "Symbol relationships parsing completed",
        &[
            (
                "total_relationships",
                &relationships_array.len().to_string(),
            ),
            (
                "successfully_parsed",
                &parsed_relationships.len().to_string(),
            ),
            ("parse_errors", &parse_errors.len().to_string()),
        ],
    );

    // For now, continue with successfully parsed relationships even if some failed
    // This provides resilience against malformed individual relationships
    if !parse_errors.is_empty() {
        let _ = log_consumer_warning(
            &format!(
                "Proceeding with {} valid relationships, {} parse errors ignored",
                parsed_relationships.len(),
                parse_errors.len()
            ),
            &[
                ("valid_count", &parsed_relationships.len().to_string()),
                ("error_count", &parse_errors.len().to_string()),
            ],
        );
    }

    Ok(parsed_relationships)
}

/// Parse a single relationship from JSON object
fn parse_single_relationship(
    relationship_json: &Value,
    index: usize,
) -> Result<SymbolRelationship, ResolutionError> {
    let obj = relationship_json
        .as_object()
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Relationship at index {} is not a JSON object", index),
        })?;

    // Extract required fields
    let source = obj
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Missing or invalid 'source' field at index {}", index),
        })?
        .to_string();

    let target = obj
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Missing or invalid 'target' field at index {}", index),
        })?
        .to_string();

    let relationship_type_str = obj
        .get("relationship_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!(
                "Missing or invalid 'relationship_type' field at index {}",
                index
            ),
        })?;

    // Parse relationship type
    let relationship_type = parse_relationship_type(relationship_type_str).ok_or_else(|| {
        ResolutionError::InvalidInput {
            message: format!(
                "Unknown relationship type '{}' at index {}",
                relationship_type_str, index
            ),
        }
    })?;

    // Extract optional CTN context
    let ctn_context = obj
        .get("ctn_context")
        .and_then(|v| v.as_u64())
        .map(|id| id as CtnNodeId);

    let _ = log_consumer_debug(
        "Parsed relationship fields",
        &[
            ("index", &index.to_string()),
            ("source", &source),
            ("target", &target),
            ("relationship_type", relationship_type_str),
            ("has_ctn_context", &ctn_context.is_some().to_string()),
        ],
    );

    Ok(SymbolRelationship {
        source,
        target,
        relationship_type,
        ctn_context,
    })
}

/// Parse relationship type string to enum
fn parse_relationship_type(type_str: &str) -> Option<RelationshipType> {
    let relationship_type = match type_str {
        "VariableInitialization" => RelationshipType::VariableInitialization,
        "VariableUsage" => RelationshipType::VariableUsage,
        "ObjectFieldExtraction" => RelationshipType::ObjectFieldExtraction,
        "StateReference" => RelationshipType::StateReference,
        "ObjectReference" => RelationshipType::ObjectReference,
        "SetReference" => RelationshipType::SetReference,
        "FilterDependency" => RelationshipType::FilterDependency,
        "RunOperationInput" => RelationshipType::RunOperationInput,
        "RunOperationTarget" => RelationshipType::RunOperationTarget,
        "SetOperandDependency" => RelationshipType::SetOperandDependency,
        "LocalStateDependency" => RelationshipType::LocalStateDependency,
        "LocalObjectDependency" => RelationshipType::LocalObjectDependency,
        _ => {
            let _ = log_consumer_warning(
                &format!("Unknown relationship type encountered: {}", type_str),
                &[("type_str", type_str)],
            );
            return None;
        }
    };

    let _ = log_consumer_debug(
        "Successfully parsed relationship type",
        &[
            ("type_str", type_str),
            ("parsed_type", relationship_type.as_str()),
            (
                "is_hard_dependency",
                &relationship_type.is_hard_dependency().to_string(),
            ),
        ],
    );

    Some(relationship_type)
}

/// Extract symbol metadata from JSON for additional context
pub fn extract_symbol_metadata(symbols_json: &Value) -> Result<SymbolMetadata, ResolutionError> {
    let _ = log_consumer_debug("Extracting symbol metadata from JSON", &[]);

    let global_symbols = symbols_json
        .get("global_symbols")
        .and_then(|v| v.as_object())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "Missing or invalid global_symbols in symbol table".to_string(),
        })?;

    // Extract variables metadata
    let variables = extract_symbol_collection(global_symbols, "variables")?;
    let states = extract_symbol_collection(global_symbols, "states")?;
    let objects = extract_symbol_collection(global_symbols, "objects")?;
    let sets = extract_symbol_collection(global_symbols, "sets")?;

    // Extract local symbol metadata
    let local_symbols = extract_local_symbol_metadata(symbols_json)?;

    let metadata = SymbolMetadata {
        variables,
        global_states: states,
        global_objects: objects,
        sets,
        local_symbols,
    };

    let _ = log_consumer_debug(
        "Symbol metadata extraction completed",
        &[
            ("variables_count", &metadata.variables.len().to_string()),
            (
                "global_states_count",
                &metadata.global_states.len().to_string(),
            ),
            (
                "global_objects_count",
                &metadata.global_objects.len().to_string(),
            ),
            ("sets_count", &metadata.sets.len().to_string()),
            (
                "local_ctns_count",
                &metadata.local_symbols.len().to_string(),
            ),
        ],
    );

    Ok(metadata)
}

/// Extract symbol collection from global_symbols object
fn extract_symbol_collection(
    global_symbols: &serde_json::Map<String, Value>,
    collection_name: &str,
) -> Result<HashMap<String, SymbolInfo>, ResolutionError> {
    let _ = log_consumer_debug(
        "Extracting symbol collection",
        &[("collection_name", collection_name)],
    );

    let collection_obj = global_symbols
        .get(collection_name)
        .and_then(|v| v.as_object())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!(
                "Missing or invalid {} collection in global_symbols",
                collection_name
            ),
        })?;

    let mut symbols = HashMap::new();

    for (symbol_id, symbol_data) in collection_obj {
        let symbol_info = parse_symbol_info(symbol_data, symbol_id)?;
        symbols.insert(symbol_id.clone(), symbol_info);
    }

    let _ = log_consumer_debug(
        "Symbol collection extracted",
        &[
            ("collection_name", collection_name),
            ("symbol_count", &symbols.len().to_string()),
        ],
    );

    Ok(symbols)
}

/// Parse individual symbol info from JSON
fn parse_symbol_info(symbol_data: &Value, symbol_id: &str) -> Result<SymbolInfo, ResolutionError> {
    let obj = symbol_data
        .as_object()
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Symbol '{}' data is not a JSON object", symbol_id),
        })?;

    let identifier = obj
        .get("identifier")
        .and_then(|v| v.as_str())
        .unwrap_or(symbol_id)
        .to_string();

    // Extract additional metadata based on symbol type
    let element_count = obj
        .get("element_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let data_type = obj
        .get("data_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let initial_value = obj.get("initial_value").cloned();

    Ok(SymbolInfo {
        identifier,
        element_count,
        data_type,
        initial_value,
    })
}

/// Extract local symbol metadata by CTN
fn extract_local_symbol_metadata(
    symbols_json: &Value,
) -> Result<HashMap<CtnNodeId, LocalSymbolInfo>, ResolutionError> {
    let _ = log_consumer_debug("Extracting local symbol metadata", &[]);

    let local_symbol_tables = symbols_json
        .get("local_symbol_tables")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: "Missing or invalid local_symbol_tables in symbol table".to_string(),
        })?;

    let mut local_symbols = HashMap::new();

    for (index, table_json) in local_symbol_tables.iter().enumerate() {
        match parse_local_symbol_table(table_json, index) {
            Ok((ctn_id, local_info)) => {
                local_symbols.insert(ctn_id, local_info);
            }
            Err(error) => {
                let _ = log_consumer_warning(
                    &format!(
                        "Failed to parse local symbol table at index {}: {}",
                        index, error
                    ),
                    &[("index", &index.to_string())],
                );
                // Continue processing other tables
            }
        }
    }

    let _ = log_consumer_debug(
        "Local symbol metadata extraction completed",
        &[("local_ctn_count", &local_symbols.len().to_string())],
    );

    Ok(local_symbols)
}

/// Parse individual local symbol table
fn parse_local_symbol_table(
    table_json: &Value,
    index: usize,
) -> Result<(CtnNodeId, LocalSymbolInfo), ResolutionError> {
    let obj = table_json
        .as_object()
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Local symbol table at index {} is not a JSON object", index),
        })?;

    let ctn_node_id = obj
        .get("ctn_node_id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Missing or invalid ctn_node_id at index {}", index),
        })? as CtnNodeId;

    let ctn_type = obj
        .get("ctn_type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ResolutionError::InvalidInput {
            message: format!("Missing or invalid ctn_type at index {}", index),
        })?
        .to_string();

    // Extract local states
    let states_count = obj
        .get("states")
        .and_then(|v| v.as_object())
        .map(|states_obj| states_obj.len())
        .unwrap_or(0);

    // Extract local object info
    let has_object = obj
        .get("object")
        .map(|obj_data| !obj_data.is_null())
        .unwrap_or(false);

    let local_info = LocalSymbolInfo {
        ctn_type,
        local_states_count: states_count,
        has_local_object: has_object,
    };

    Ok((ctn_node_id, local_info))
}

/// Metadata about symbols extracted from JSON
#[derive(Debug, Clone)]
pub struct SymbolMetadata {
    pub variables: HashMap<String, SymbolInfo>,
    pub global_states: HashMap<String, SymbolInfo>,
    pub global_objects: HashMap<String, SymbolInfo>,
    pub sets: HashMap<String, SymbolInfo>,
    pub local_symbols: HashMap<CtnNodeId, LocalSymbolInfo>,
}

/// Information about individual symbols
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub identifier: String,
    pub element_count: usize,
    pub data_type: Option<String>,
    pub initial_value: Option<Value>,
}

/// Information about local symbols within a CTN
#[derive(Debug, Clone)]
pub struct LocalSymbolInfo {
    pub ctn_type: String,
    pub local_states_count: usize,
    pub has_local_object: bool,
}

impl SymbolMetadata {
    /// Get total symbol count across all types
    pub fn total_symbol_count(&self) -> usize {
        self.variables.len()
            + self.global_states.len()
            + self.global_objects.len()
            + self.sets.len()
            + self
                .local_symbols
                .values()
                .map(|ls| ls.local_states_count + if ls.has_local_object { 1 } else { 0 })
                .sum::<usize>()
    }

    /// Check if a global symbol exists
    pub fn has_global_symbol(&self, symbol_id: &str) -> bool {
        self.variables.contains_key(symbol_id)
            || self.global_states.contains_key(symbol_id)
            || self.global_objects.contains_key(symbol_id)
            || self.sets.contains_key(symbol_id)
    }

    /// Get symbol type if it exists
    pub fn get_symbol_type(&self, symbol_id: &str) -> Option<&str> {
        if self.variables.contains_key(symbol_id) {
            Some("Variable")
        } else if self.global_states.contains_key(symbol_id) {
            Some("GlobalState")
        } else if self.global_objects.contains_key(symbol_id) {
            Some("GlobalObject")
        } else if self.sets.contains_key(symbol_id) {
            Some("SetOperation")
        } else {
            None
        }
    }
}

/// Validate parsed relationships against symbol metadata
pub fn validate_relationships(
    relationships: &[SymbolRelationship],
    metadata: &SymbolMetadata,
) -> Result<(), ResolutionError> {
    let _ = log_consumer_debug(
        "Validating parsed relationships against symbol metadata",
        &[
            ("relationship_count", &relationships.len().to_string()),
            ("total_symbols", &metadata.total_symbol_count().to_string()),
        ],
    );

    let mut validation_errors = Vec::new();

    for (index, relationship) in relationships.iter().enumerate() {
        // Validate source symbol exists
        if !metadata.has_global_symbol(&relationship.source) {
            validation_errors.push(format!(
                "Relationship {}: source symbol '{}' not found in metadata",
                index, relationship.source
            ));
        }

        // Validate target symbol exists
        if !metadata.has_global_symbol(&relationship.target) {
            validation_errors.push(format!(
                "Relationship {}: target symbol '{}' not found in metadata",
                index, relationship.target
            ));
        }

        // Validate relationship type makes sense for symbols involved
        if let Err(type_error) = validate_relationship_type_compatibility(relationship, metadata) {
            validation_errors.push(format!("Relationship {}: {}", index, type_error));
        }
    }

    if !validation_errors.is_empty() {
        let error_summary = validation_errors.join("; ");
        let _ = log_consumer_error(
            consumer_codes::CONSUMER_VALIDATION_ERROR,
            &format!("Relationship validation failed: {}", error_summary),
            &[("error_count", &validation_errors.len().to_string())],
        );

        return Err(ResolutionError::InvalidInput {
            message: format!("Relationship validation failed: {}", error_summary),
        });
    }

    let _ = log_consumer_debug("Relationship validation completed successfully", &[]);
    Ok(())
}

/// Validate that relationship type is compatible with involved symbols
fn validate_relationship_type_compatibility(
    relationship: &SymbolRelationship,
    metadata: &SymbolMetadata,
) -> Result<(), String> {
    let source_type = metadata.get_symbol_type(&relationship.source);
    let target_type = metadata.get_symbol_type(&relationship.target);

    match (&relationship.relationship_type, source_type, target_type) {
        // RunOperationInput: runtime operation depends on variable/object
        (RelationshipType::RunOperationInput, Some("RuntimeOperation"), Some("Variable")) => Ok(()),
        (RelationshipType::RunOperationInput, Some("RuntimeOperation"), Some("GlobalObject")) => {
            Ok(())
        }

        // VariableUsage: state/object field uses variable
        (RelationshipType::VariableUsage, Some("GlobalState"), Some("Variable")) => Ok(()),
        (RelationshipType::VariableUsage, Some("GlobalObject"), Some("Variable")) => Ok(()),

        // ObjectFieldExtraction: runtime operation extracts from object
        (
            RelationshipType::ObjectFieldExtraction,
            Some("RuntimeOperation"),
            Some("GlobalObject"),
        ) => Ok(()),

        // SetReference: set operation references another set
        (RelationshipType::SetReference, Some("SetOperation"), Some("SetOperation")) => Ok(()),

        // ObjectReference: set operation references object
        (RelationshipType::ObjectReference, Some("SetOperation"), Some("GlobalObject")) => Ok(()),

        // Allow unknown types to pass validation (defensive)
        (_, None, _) | (_, _, None) => Ok(()),

        // Invalid combinations
        _ => Err(format!(
            "Invalid relationship type '{}' between source type '{:?}' and target type '{:?}'",
            relationship.relationship_type.as_str(),
            source_type,
            target_type
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_simple_relationship() {
        let json = json!({
            "source": "var_a",
            "target": "var_b",
            "relationship_type": "VariableUsage"
        });

        let result = parse_single_relationship(&json, 0).unwrap();

        assert_eq!(result.source, "var_a");
        assert_eq!(result.target, "var_b");
        assert_eq!(result.relationship_type, RelationshipType::VariableUsage);
        assert_eq!(result.ctn_context, None);
    }

    #[test]
    fn test_parse_relationship_with_ctn_context() {
        let json = json!({
            "source": "local_state",
            "target": "global_var",
            "relationship_type": "LocalStateDependency",
            "ctn_context": 42
        });

        let result = parse_single_relationship(&json, 0).unwrap();

        assert_eq!(result.ctn_context, Some(42));
    }

    #[test]
    fn test_parse_relationships_array() {
        let json = json!({
            "relationships": [
                {
                    "source": "op1",
                    "target": "var1",
                    "relationship_type": "RunOperationInput"
                },
                {
                    "source": "set1",
                    "target": "obj1",
                    "relationship_type": "ObjectReference"
                }
            ]
        });

        let result = parse_relationships_from_json(&json).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].relationship_type,
            RelationshipType::RunOperationInput
        );
        assert_eq!(
            result[1].relationship_type,
            RelationshipType::ObjectReference
        );
    }

    #[test]
    fn test_parse_invalid_relationship_type() {
        let json = json!({
            "source": "a",
            "target": "b",
            "relationship_type": "InvalidType"
        });

        let result = parse_single_relationship(&json, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_symbol_metadata() {
        let json = json!({
            "global_symbols": {
                "variables": {
                    "var1": {
                        "identifier": "var1",
                        "data_type": "String",
                        "initial_value": "test"
                    }
                },
                "states": {
                    "state1": {
                        "identifier": "state1",
                        "element_count": 3
                    }
                },
                "objects": {},
                "sets": {}
            },
            "local_symbol_tables": []
        });

        let metadata = extract_symbol_metadata(&json).unwrap();

        assert_eq!(metadata.variables.len(), 1);
        assert_eq!(metadata.global_states.len(), 1);
        assert!(metadata.has_global_symbol("var1"));
        assert!(metadata.has_global_symbol("state1"));
        assert!(!metadata.has_global_symbol("nonexistent"));
    }
}
