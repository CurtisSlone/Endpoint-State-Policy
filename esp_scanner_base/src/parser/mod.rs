// parser/mod.rs

pub mod criterion;
pub mod filter;
pub mod metadata;
pub mod object;
pub mod runtime_operation;
pub mod set;
pub mod state;
pub mod variable;

use crate::ffi::logging::{consumer_codes, log_consumer_debug, log_consumer_error};
use crate::ffi::types::PipelineOutput;
use crate::types::criterion::CtnNodeId;
use crate::types::resolution_context::{RelationshipType, ResolutionContext, SymbolRelationship};

impl ResolutionContext {
    /// Single entry point for parsing pipeline output with DAG relationship support
    /// This method coordinates all parser modules to build the complete ResolutionContext
    pub fn from_pipeline_output(output: &PipelineOutput) -> Result<Self, String> {
        let _ = log_consumer_debug(
            "Starting pipeline output parsing with DAG support",
            &[
                ("has_ast", &(!output.ast_tree.is_null()).to_string()),
                ("has_symbols", &(!output.symbols.is_null()).to_string()),
            ],
        );

        // Initialize empty context
        let mut context = Self::new();

        // =====================================================================
        // Phase 1: Parse Metadata (Required)
        // =====================================================================

        let _ = log_consumer_debug("Phase 1: Parsing metadata", &[]);

        match metadata::extract_metadata_from_json(&output.ast_tree) {
            Ok(metadata_block) => {
                context.metadata = Some(metadata_block);
                let _ = log_consumer_debug(
                    "Metadata parsing completed",
                    &[(
                        "field_count",
                        &context.metadata.as_ref().unwrap().fields.len().to_string(),
                    )],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse metadata: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // =====================================================================
        // Phase 2: Parse Variables (with computed variable support)
        // =====================================================================

        let _ = log_consumer_debug("Phase 2: Parsing variables with computed support", &[]);

        match variable::extract_and_categorize_variables_from_json(&output.ast_tree, &mut context) {
            Ok(variables) => {
                context.variables = variables;
                let _ = log_consumer_debug(
                    "Variable parsing completed",
                    &[
                        ("total_variables", &context.variables.len().to_string()),
                        (
                            "computed_variables",
                            &context.computed_variables.len().to_string(),
                        ),
                        (
                            "literal_variables",
                            &context.literal_variables.len().to_string(),
                        ),
                        (
                            "reference_variables",
                            &context.reference_variables.len().to_string(),
                        ),
                    ],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse variables: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // Phase 3: Parse Global States (with deferred validation)
        let _ = log_consumer_debug("Phase 3: Parsing global states", &[]);

        match state::extract_states_from_json(&output.ast_tree, &mut context) {
            Ok((global_states, local_states_by_ctn)) => {
                context.global_states = global_states;

                // Store local states in context
                for (ctn_id, states) in local_states_by_ctn {
                    context.ctn_local_states.insert(ctn_id, states);
                }

                let _ = log_consumer_debug(
                    "Global and local state parsing completed",
                    &[
                        ("global_states", &context.global_states.len().to_string()),
                        (
                            "local_states_ctns",
                            &context.ctn_local_states.len().to_string(),
                        ),
                    ],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse states: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // =====================================================================
        // Phase 4: Parse Global Objects (with deferred validation)
        // =====================================================================

        let _ = log_consumer_debug("Phase 4: Parsing global objects", &[]);

        match object::extract_objects_from_json(&output.ast_tree) {
            Ok((global_objects, local_objects_by_ctn, parsing_context)) => {
                context.global_objects = global_objects;

                // Store local objects - ICS allows max 1 per CTN
                for (ctn_id, mut objects) in local_objects_by_ctn {
                    if let Some(obj) = objects.pop() {
                        context.ctn_local_objects.insert(ctn_id, obj);
                    }
                }

                // Apply parsing context (deferred validations, errors)
                parsing_context.apply_to_resolution_context(&mut context);

                let _ = log_consumer_debug(
                    "Object parsing completed",
                    &[
                        ("global_objects", &context.global_objects.len().to_string()),
                        (
                            "local_objects",
                            &context.ctn_local_objects.len().to_string(),
                        ),
                        (
                            "deferred_validations",
                            &context.deferred_validations.len().to_string(),
                        ),
                    ],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse objects: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // =====================================================================
        // Phase 5: Parse Runtime Operations (with deferred validation)
        // =====================================================================

        let _ = log_consumer_debug("Phase 5: Parsing runtime operations", &[]);

        match runtime_operation::extract_runtime_operations_from_json_with_context(&output.ast_tree)
        {
            Ok(parsing_result) => {
                context.runtime_operations = parsing_result.runtime_operations;

                // Apply parsing context
                parsing_result
                    .context
                    .apply_to_resolution_context(&mut context);

                let _ = log_consumer_debug(
                    "Runtime operation parsing completed",
                    &[
                        (
                            "runtime_operations",
                            &context.runtime_operations.len().to_string(),
                        ),
                        (
                            "deferred_validations",
                            &context.deferred_validations.len().to_string(),
                        ),
                    ],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse runtime operations: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // =====================================================================
        // Phase 6: Parse Set Operations (with deferred validation)
        // =====================================================================

        let _ = log_consumer_debug("Phase 6: Parsing set operations", &[]);

        match set::extract_set_operations_from_json(&output.ast_tree, &mut context) {
            Ok(set_operations) => {
                context.set_operations = set_operations;
                let _ = log_consumer_debug(
                    "Set operation parsing completed",
                    &[
                        ("set_operations", &context.set_operations.len().to_string()),
                        (
                            "deferred_validations",
                            &context.deferred_validations.len().to_string(),
                        ),
                    ],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse set operations: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // =====================================================================
        // Phase 7: Parse Criteria (flattened, with local symbols)
        // =====================================================================

        let _ = log_consumer_debug("Phase 7: Parsing criteria with local symbols", &[]);

        match criterion::extract_criteria_from_json(&output.ast_tree, &mut context) {
            Ok(criteria_root) => {
                context.criteria_root = criteria_root; // Changed from context.criteria
                let _ = log_consumer_debug(
                    "Criteria parsing completed",
                    &[
                        (
                            "criteria_trees",
                            &context.criteria_root.trees.len().to_string(),
                        ), // Changed
                        (
                            "total_criteria",
                            &context.criteria_root.total_criteria_count().to_string(),
                        ), // Changed
                        (
                            "local_states_ctns",
                            &context.ctn_local_states.len().to_string(),
                        ),
                        (
                            "local_objects_ctns",
                            &context.ctn_local_objects.len().to_string(),
                        ),
                        (
                            "deferred_validations",
                            &context.deferred_validations.len().to_string(),
                        ),
                    ],
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to parse criteria: {:?}", e);
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_DATA_VALIDATION_ERROR,
                    &error_msg,
                    &[],
                );
                return Err(error_msg);
            }
        }

        // =====================================================================
        // Phase 8: Structural Validation Only (No Cross-Reference Checks)
        // =====================================================================

        let _ = log_consumer_debug("Phase 8: Structural validation", &[]);

        if let Err(validation_error) = Self::validate_structure_only(&context) {
            let _ = log_consumer_error(
                consumer_codes::CONSUMER_VALIDATION_ERROR,
                &format!("Structural validation failed: {}", validation_error),
                &[],
            );
            return Err(validation_error);
        }

        // =====================================================================
        // Phase 9: Extract Relationships from Symbols JSON (if available)
        // =====================================================================

        let _ = log_consumer_debug("Phase 9: Extracting relationships from symbols", &[]);

        if !output.symbols.is_null() {
            if let Some(relationships) = output.symbols.get("relationships") {
                if let Some(relationships_array) = relationships.as_array() {
                    for rel in relationships_array {
                        if let Ok(relationship) = Self::parse_relationship_from_json(rel) {
                            context.relationships.push(relationship);
                        }
                    }

                    let _ = log_consumer_debug(
                        "Relationships extracted",
                        &[(
                            "relationship_count",
                            &context.relationships.len().to_string(),
                        )],
                    );
                }
            }
        } else {
            let _ = log_consumer_debug(
                "No symbols JSON provided - relationships will be empty",
                &[],
            );
            context.relationships = Vec::new();
        }

        // =====================================================================
        // Summary Logging
        // =====================================================================

        let _ = log_consumer_debug(
            "Pipeline parsing completed successfully",
            &[
                ("variables", &context.variables.len().to_string()),
                (
                    "computed_variables",
                    &context.computed_variables.len().to_string(),
                ),
                ("global_states", &context.global_states.len().to_string()),
                ("global_objects", &context.global_objects.len().to_string()),
                (
                    "runtime_operations",
                    &context.runtime_operations.len().to_string(),
                ),
                ("set_operations", &context.set_operations.len().to_string()),
                (
                    "criteria_trees",
                    &context.criteria_root.trees.len().to_string(),
                ),
                ("relationships", &context.relationships.len().to_string()),
                (
                    "deferred_validations",
                    &context.deferred_validations.len().to_string(),
                ),
                ("parsing_errors", &context.parsing_errors.len().to_string()),
            ],
        );

        Ok(context)
    }

    /// Structural validation only - NO cross-reference validation
    /// This validates:
    /// - Identifier uniqueness within scopes
    /// - Empty names/identifiers
    /// - Required fields present
    /// - Duplicate field names within elements
    /// - Self-references (variable = itself)
    ///
    /// This does NOT validate:
    /// - Whether referenced symbols exist (deferred to DAG)
    /// - Whether variable references are valid (deferred to DAG)
    /// - Whether state/object refs point to existing symbols (deferred to DAG)
    fn validate_structure_only(context: &ResolutionContext) -> Result<(), String> {
        let _ = log_consumer_debug("Starting structural validation (no cross-references)", &[]);

        let mut errors = Vec::new();

        // Validate identifier uniqueness across GLOBAL scopes
        let mut global_identifiers = std::collections::HashSet::new();

        // Variables (all global)
        for variable in &context.variables {
            if variable.name.is_empty() {
                errors.push("Variable with empty name found".to_string());
            }
            if !global_identifiers.insert(variable.name.clone()) {
                errors.push(format!("Duplicate global identifier: '{}'", variable.name));
            }

            // Check self-reference (structural issue)
            if let Some(crate::types::common::Value::Variable(ref_name)) = &variable.initial_value {
                if ref_name == &variable.name {
                    errors.push(format!("Variable '{}' references itself", variable.name));
                }
            }
        }

        // Global states
        for state in &context.global_states {
            if state.identifier.is_empty() {
                errors.push("State with empty identifier found".to_string());
            }
            if !global_identifiers.insert(state.identifier.clone()) {
                errors.push(format!(
                    "Duplicate global identifier: '{}'",
                    state.identifier
                ));
            }
            if state.fields.is_empty() && state.record_checks.is_empty() {
                errors.push(format!(
                    "State '{}' has no fields or record checks",
                    state.identifier
                ));
            }
        }

        // Global objects
        for object in &context.global_objects {
            if object.identifier.is_empty() {
                errors.push("Object with empty identifier found".to_string());
            }
            if !global_identifiers.insert(object.identifier.clone()) {
                errors.push(format!(
                    "Duplicate global identifier: '{}'",
                    object.identifier
                ));
            }
            if object.elements.is_empty() {
                errors.push(format!("Object '{}' has no elements", object.identifier));
            }

            // Check duplicate field names within object
            let mut field_names = std::collections::HashSet::new();
            for element in &object.elements {
                if let crate::types::object::ObjectElement::Field { name, .. } = element {
                    if !field_names.insert(name) {
                        errors.push(format!(
                            "Object '{}' has duplicate field name '{}'",
                            object.identifier, name
                        ));
                    }
                }
            }
        }

        // Set operations
        for set_op in &context.set_operations {
            if set_op.set_id.is_empty() {
                errors.push("Set operation with empty ID found".to_string());
            }
            if !global_identifiers.insert(set_op.set_id.clone()) {
                errors.push(format!("Duplicate global identifier: '{}'", set_op.set_id));
            }

            // Validate operand count (structural constraint)
            if let Err(validation_error) = set_op.validate() {
                errors.push(format!(
                    "Set operation '{}' structural validation failed: {}",
                    set_op.set_id, validation_error
                ));
            }

            // Check self-reference (structural issue)
            for operand in &set_op.operands {
                if let crate::types::set::SetOperand::SetRef(ref_set_id) = operand {
                    if ref_set_id == &set_op.set_id {
                        errors.push(format!(
                            "Set operation '{}' references itself",
                            set_op.set_id
                        ));
                    }
                }
            }
        }

        // Runtime operations
        for runtime_op in &context.runtime_operations {
            if runtime_op.target_variable.is_empty() {
                errors.push("Runtime operation with empty target variable found".to_string());
            }
            if !global_identifiers.insert(runtime_op.target_variable.clone()) {
                errors.push(format!(
                    "Duplicate global identifier: '{}'",
                    runtime_op.target_variable
                ));
            }

            // Validate operation structure (not references)
            if let Err(validation_error) = runtime_op.validate() {
                errors.push(format!(
                    "Runtime operation '{}' structural validation failed: {}",
                    runtime_op.target_variable, validation_error
                ));
            }

            // Check self-reference in parameters (structural issue)
            for param in &runtime_op.parameters {
                if let crate::types::runtime_operation::RunParameter::Variable(var_name) = param {
                    if var_name == &runtime_op.target_variable {
                        errors.push(format!(
                            "Runtime operation '{}' references itself in parameters",
                            runtime_op.target_variable
                        ));
                    }
                }
            }
        }

        // Criteria
        // Extract all criteria from the tree for validation
        let all_criteria = context.criteria_root.get_all_criteria();

        for criterion in all_criteria {
            if criterion.criterion_type.is_empty() {
                errors.push("Criterion with empty type found".to_string());
            }
            if criterion.ctn_node_id.is_none() {
                errors.push(format!(
                    "Criterion '{}' missing CTN node ID",
                    criterion.criterion_type
                ));
            }

            // Validate criterion has at least some content (structural requirement)
            let has_content = !criterion.state_refs.is_empty()
                || !criterion.object_refs.is_empty()
                || !criterion.local_states.is_empty()
                || criterion.local_object.is_some();

            if !has_content {
                errors.push(format!("Criterion '{}' has no content (state refs, object refs, or local elements required)", criterion.criterion_type));
            }
        }

        // Validate local identifier uniqueness within each CTN scope
        for (ctn_id, local_states) in &context.ctn_local_states {
            let mut local_identifiers = std::collections::HashSet::new();

            for state in local_states {
                if state.identifier.is_empty() {
                    errors.push(format!(
                        "Local state with empty identifier in CTN {}",
                        ctn_id
                    ));
                }
                if !local_identifiers.insert(state.identifier.clone()) {
                    errors.push(format!(
                        "Duplicate local identifier '{}' in CTN {}",
                        state.identifier, ctn_id
                    ));
                }
                if state.fields.is_empty() && state.record_checks.is_empty() {
                    errors.push(format!(
                        "Local state '{}' in CTN {} has no fields",
                        state.identifier, ctn_id
                    ));
                }
            }
        }

        for (ctn_id, local_object) in &context.ctn_local_objects {
            if local_object.identifier.is_empty() {
                errors.push(format!(
                    "Local object with empty identifier in CTN {}",
                    ctn_id
                ));
            }
            if local_object.elements.is_empty() {
                errors.push(format!(
                    "Local object '{}' in CTN {} has no elements",
                    local_object.identifier, ctn_id
                ));
            }
        }

        // Check reserved keywords not used as identifiers
        let reserved_keywords = [
            "DEF",
            "VAR",
            "STATE",
            "OBJECT",
            "CTN",
            "CRI",
            "SET",
            "RUN",
            "TEST",
            "FILTER",
            "META",
            "DEF_END",
            "STATE_END",
            "OBJECT_END",
            "CTN_END",
            "CRI_END",
            "SET_END",
            "RUN_END",
            "FILTER_END",
            "META_END",
            "STATE_REF",
            "OBJECT_REF",
            "SET_REF",
            "AND",
            "OR",
            "true",
            "false",
        ];

        for identifier in &global_identifiers {
            if reserved_keywords.contains(&identifier.as_str()) {
                errors.push(format!(
                    "Reserved keyword '{}' used as identifier",
                    identifier
                ));
            }
        }

        if !errors.is_empty() {
            let error_summary = errors.join("; ");
            return Err(error_summary);
        }

        let _ = log_consumer_debug(
            "Structural validation passed",
            &[("global_identifiers", &global_identifiers.len().to_string())],
        );

        Ok(())
    }

    /// Parse relationship from symbols JSON
    fn parse_relationship_from_json(
        rel_json: &serde_json::Value,
    ) -> Result<SymbolRelationship, String> {
        let rel_obj = rel_json
            .as_object()
            .ok_or("Relationship must be an object")?;

        let relationship_type_str = rel_obj
            .get("relationship_type")
            .and_then(|v| v.as_str())
            .ok_or("Missing relationship type")?;

        let relationship_type = RelationshipType::from_str(relationship_type_str)
            .ok_or_else(|| format!("Invalid relationship type: {}", relationship_type_str))?;

        let source = rel_obj
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or("Missing relationship source")?
            .to_string();

        let target = rel_obj
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or("Missing relationship target")?
            .to_string();

        // Extract optional CTN context for local symbol relationships
        let ctn_context = rel_obj
            .get("ctn_context")
            .and_then(|v| v.as_u64())
            .map(|id| id as CtnNodeId);

        Ok(SymbolRelationship {
            relationship_type,
            source,
            target,
            ctn_context,
        })
    }
}
