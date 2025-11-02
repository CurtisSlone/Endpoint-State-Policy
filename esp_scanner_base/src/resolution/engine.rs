use crate::ffi::logging::{
    consumer_codes, log_consumer_debug, log_consumer_error, log_consumer_info,
};
use crate::ffi::types::PipelineOutput;
use crate::resolution::dag::{DependencyGraph, SymbolType};
use crate::resolution::error::ResolutionError;
use crate::resolution::field_resolver::FieldResolver;
use crate::resolution::symbol_parser;
use crate::types::execution_context::ExecutionContext;
use crate::types::object::{ObjectDeclaration, ObjectElement, ResolvedObject};
use crate::types::resolution_context::{DeferredOperation, ResolutionContext};
use crate::types::runtime_operation::{OperationCategory, RunParameter};
use crate::types::set::SetOperand;
use crate::types::state::{ResolvedState, StateDeclaration};
use crate::types::variable::{ResolvedVariable, VariableDeclaration};
use crate::types::CriterionDeclaration;
use crate::types::{
    RecordCheck, RecordContent, ResolvedRecordCheck, ResolvedRecordContent, ResolvedRecordField,
};
use std::collections::HashMap;

pub struct ResolutionEngine {
    field_resolver: FieldResolver,
}

impl ResolutionEngine {
    pub fn new() -> Self {
        let _ = log_consumer_debug("Creating DAG-based Resolution Engine", &[]);
        Self {
            field_resolver: FieldResolver::new(),
        }
    }

    pub fn process_pipeline_output(
    &mut self,
    output: &PipelineOutput,
) -> Result<ExecutionContext, ResolutionError> {
    let _ = log_consumer_info(
        "Starting DAG resolution pipeline",
        &[
            ("has_ast", &(!output.ast_tree.is_null()).to_string()),
            ("has_symbols", &(!output.symbols.is_null()).to_string()),
        ],
    );

    // Parse pipeline output with relationships
    let mut context = self.parse_pipeline_output(output)?;

    // Perform DAG-based resolution
    self.resolve_dag(&mut context)?;

    // ✅ ADD THIS LINE HERE (between resolve_dag and ExecutionContext creation):
    crate::resolution::set_expansion::expand_sets_in_resolution_context(&mut context)?;

    // Create ExecutionContext
    let execution_context = ExecutionContext::from_resolution_context(&context)
        .map_err(|e| ResolutionError::ContextError(e.to_string()))?;

    execution_context
        .validate()
        .map_err(|e| ResolutionError::ContextError(e.to_string()))?;

    let _ = log_consumer_info(
        "DAG resolution pipeline completed",
        &[
            (
                "criteria_count",
                &execution_context.criteria_tree.count_criteria().to_string(),
            ),
            (
                "variables_count",
                &execution_context.global_variables.len().to_string(),
            ),
            (
                "deferred_operations",
                &execution_context.deferred_operations.len().to_string(),
            ),
        ],
    );

    Ok(execution_context)
}

    /// Parse pipeline output with integrated relationship extraction
    fn parse_pipeline_output(
        &self,
        output: &PipelineOutput,
    ) -> Result<ResolutionContext, ResolutionError> {
        let _ = log_consumer_debug("Parsing pipeline output with relationship integration", &[]);

        if output.ast_tree.is_null() {
            return Err(ResolutionError::InvalidInput {
                message: "Pipeline output contains null AST".to_string(),
            });
        }

        // Parse base context from AST
        let mut context = ResolutionContext::from_pipeline_output(output)
            .map_err(|e| ResolutionError::InvalidInput { message: e })?;

        // Parse symbol relationships
        match symbol_parser::parse_relationships_from_json(&output.symbols) {
            Ok(relationships) => {
                let _ = log_consumer_debug(
                    "Parsed symbol relationships",
                    &[("count", &relationships.len().to_string())],
                );
                context.relationships = relationships;
            }
            Err(e) => {
                let _ = log_consumer_error(
                    consumer_codes::CONSUMER_PIPELINE_ERROR,
                    &format!("Failed to parse relationships: {}", e),
                    &[],
                );
                return Err(e);
            }
        }

        Ok(context)
    }

    /// Main DAG resolution method
    fn resolve_dag(&mut self, context: &mut ResolutionContext) -> Result<(), ResolutionError> {
        let _ = log_consumer_info(
            "Starting DAG resolution",
            &[
                ("symbols", &self.count_symbols(context).to_string()),
                ("relationships", &context.relationships.len().to_string()),
            ],
        );

        // Build dependency graph (only includes resolution-time operations)
        let graph = self.build_dependency_graph(context)?;

        // Get resolution order via topological sort
        let resolution_order = graph.topological_sort()?;

        let _ = log_consumer_debug(
            "Topological sort completed",
            &[
                (
                    "resolution_order_length",
                    &resolution_order.len().to_string(),
                ),
                (
                    "first_symbols",
                    &resolution_order
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            ],
        );

        // Resolve symbols in dependency order
        self.resolve_symbols_in_order(&resolution_order, context)?;

        let _ = log_consumer_info(
            "Before resolving local symbols",
            &[
                (
                    "resolved_variables_count",
                    &context.resolved_variables.len().to_string(),
                ),
                (
                    "resolved_variables",
                    &context
                        .resolved_variables
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            ],
        );

        // Resolve local symbols (CTN-scoped)
        self.resolve_local_symbols(context)?;

        let _ = log_consumer_info(
            "DAG resolution completed",
            &[
                (
                    "resolved_variables",
                    &context.resolved_variables.len().to_string(),
                ),
                ("resolved_sets", &context.resolved_sets.len().to_string()),
                (
                    "deferred_operations",
                    &context.scan_time_operations.len().to_string(),
                ),
            ],
        );

        Ok(())
    }

    /// Build dependency graph with runtime operation categorization
    fn build_dependency_graph(
        &self,
        context: &ResolutionContext,
    ) -> Result<DependencyGraph, ResolutionError> {
        let _ = log_consumer_debug(
            "Building dependency graph with operation categorization",
            &[],
        );

        let mut graph = DependencyGraph::new();

        // Step 1: Add all declared variables as nodes
        for variable in &context.variables {
            graph.add_node(variable.name.clone(), SymbolType::Variable)?;
        }

        // Step 2: Add RUN operation target variables as nodes (computed variables)
        // This ensures all computed variables exist before edges reference them
        for runtime_op in &context.runtime_operations {
            if !graph.nodes.contains_key(&runtime_op.target_variable) {
                graph.add_node(runtime_op.target_variable.clone(), SymbolType::Variable)?;

                let _ = log_consumer_debug(
                    "Added computed variable node to DAG",
                    &[
                        ("variable", &runtime_op.target_variable),
                        ("operation", runtime_op.operation_type.as_str()),
                    ],
                );
            }
        }

        // Step 3: Add states, objects, sets as nodes
        for state in &context.global_states {
            graph.add_node(state.identifier.clone(), SymbolType::GlobalState)?;
        }

        for object in &context.global_objects {
            graph.add_node(object.identifier.clone(), SymbolType::GlobalObject)?;
        }

        for set_op in &context.set_operations {
            graph.add_node(set_op.set_id.clone(), SymbolType::SetOperation)?;
        }

        // Step 4: Add edges from parsed relationships (hard dependencies only)
        // All nodes now exist, so this won't fail
        for relationship in &context.relationships {
            if relationship.relationship_type.is_hard_dependency() {
                graph.add_dependency(&relationship.source, &relationship.target)?;
            }
        }

        // Step 5: Add runtime operation dependencies (ONLY resolution-time operations)
        for runtime_op in &context.runtime_operations {
            match runtime_op.categorize(context) {
                OperationCategory::ResolutionTime => {
                    // Add to DAG: target variable depends on input parameters
                    for param in &runtime_op.parameters {
                        match param {
                            RunParameter::Variable(input_var) => {
                                graph.add_dependency(&runtime_op.target_variable, input_var)?;
                            }
                            RunParameter::ArithmeticOp { operand, .. } => {
                                if let Some(var_name) = operand.get_variable_name() {
                                    graph.add_dependency(&runtime_op.target_variable, var_name)?;
                                }
                            }
                            RunParameter::Literal(value) => {
                                // Check if literal contains variable reference
                                if let Some(var_name) = value.get_variable_name() {
                                    graph.add_dependency(&runtime_op.target_variable, var_name)?;
                                }
                            }
                            _ => {} // Other parameters don't create dependencies
                        }
                    }

                    let _ = log_consumer_debug(
                        "Added resolution-time RUN operation to DAG",
                        &[
                            ("target", &runtime_op.target_variable),
                            ("operation", runtime_op.operation_type.as_str()),
                        ],
                    );
                }
                OperationCategory::ScanTime => {
                    // Don't add to DAG - will be deferred
                    let _ = log_consumer_debug(
                        "Runtime operation deferred to scan-time (not in DAG)",
                        &[
                            ("target", &runtime_op.target_variable),
                            ("operation", runtime_op.operation_type.as_str()),
                            (
                                "has_object_dep",
                                &runtime_op.has_object_dependency().to_string(),
                            ),
                        ],
                    );
                }
            }
        }

        // Step 6: Add variable initialization dependencies
        for variable in &context.variables {
            if let Some(var_ref) = variable.get_variable_reference() {
                graph.add_dependency(&variable.name, var_ref)?;
            }
        }

        // Step 7: Add cross-type dependencies (states, objects, sets)
        self.add_cross_type_dependencies(&mut graph, context)?;

        // Step 8: Validate graph integrity
        graph.validate()?;

        let stats = graph.get_stats();
        let _ = log_consumer_debug(
            "Dependency graph built successfully",
            &[
                ("nodes", &stats.total_nodes.to_string()),
                ("edges", &stats.total_edges.to_string()),
                (
                    "independent_nodes",
                    &graph.get_independent_symbols().len().to_string(),
                ),
            ],
        );

        Ok(graph)
    }

    fn extract_all_criteria_from_tree(context: &ResolutionContext) -> Vec<&CriterionDeclaration> {
        context.criteria_root.get_all_criteria()
    }
    /// Add cross-type dependencies (SET operations, states, objects)
    fn add_cross_type_dependencies(
        &self,
        graph: &mut DependencyGraph,
        context: &ResolutionContext,
    ) -> Result<(), ResolutionError> {
        // SET operations depending on variables/objects
        for set_op in &context.set_operations {
            for operand in &set_op.operands {
                match operand {
                    SetOperand::ObjectRef(obj_id) => {
                        graph.add_dependency(&set_op.set_id, obj_id)?;
                    }
                    SetOperand::SetRef(other_set_id) => {
                        graph.add_dependency(&set_op.set_id, other_set_id)?;
                    }
                    SetOperand::InlineObject(obj) => {
                        for var_ref in obj.get_variable_references() {
                            graph.add_dependency(&set_op.set_id, &var_ref)?;
                        }
                    }
                }
            }

            // SET filter dependencies
            if let Some(filter) = &set_op.filter {
                for state_ref in &filter.state_refs {
                    graph.add_dependency(&set_op.set_id, state_ref)?;
                }
            }
        }

        // State dependencies on variables
        for state in &context.global_states {
            for field in &state.fields {
                if let Some(var_name) = field.value.get_variable_name() {
                    graph.add_dependency(&state.identifier, var_name)?;
                }
            }
        }

        for object in &context.global_objects {
            for var_ref in object.get_variable_references() {
                graph.add_dependency(&object.identifier, &var_ref)?;
            }
        }

        Ok(())
    }

    /// Resolve symbols in topological order
    fn resolve_symbols_in_order(
        &mut self,
        resolution_order: &[String],
        context: &mut ResolutionContext,
    ) -> Result<(), ResolutionError> {
        let _ = log_consumer_debug(
            "Resolution order",
            &[("order", &resolution_order.join(" -> "))],
        );

        for symbol_id in resolution_order {
            let symbol_type = self.determine_symbol_type(symbol_id, context);

            match symbol_type {
                Some(SymbolType::Variable) => {
                    self.resolve_variable(symbol_id, context)?;
                }
                Some(SymbolType::GlobalState) => {
                    self.resolve_global_state(symbol_id, context)?;
                }
                Some(SymbolType::GlobalObject) => {
                    self.resolve_global_object(symbol_id, context)?;
                }
                Some(SymbolType::SetOperation) => {
                    self.resolve_set_operation(symbol_id, context)?;
                }
                _ => {
                    let _ = log_consumer_debug("Skipping unknown symbol", &[("id", symbol_id)]);
                }
            }
        }
        Ok(())
    }

    /// Resolve variable (handles both initialized and computed variables)
    fn resolve_variable(
        &mut self,
        variable_name: &str,
        context: &mut ResolutionContext,
    ) -> Result<(), ResolutionError> {
        // Try to find declared variable
        let variable = context
            .variables
            .iter()
            .find(|v| v.name == variable_name)
            .cloned();

        if let Some(var) = variable {
            // CASE 1: Variable is declared
            if var.has_initial_value() {
                let resolved = self.resolve_variable_with_initial_value(&var, context)?;
                context
                    .resolved_variables
                    .insert(variable_name.to_string(), resolved);
            } else {
                // Declared computed variable (no initial value)

                if let Some(run_op) = context
                    .runtime_operations
                    .iter()
                    .find(|op| op.target_variable == variable_name)
                {
                    match run_op.categorize(context) {
                        OperationCategory::ResolutionTime => {
                            let result =
                                crate::resolution::runtime_operations::execute_runtime_operation(
                                    run_op,
                                    &context.resolved_variables,
                                )?;

                            let resolved_var = ResolvedVariable {
                                identifier: variable_name.to_string(),
                                data_type: var.data_type.clone(),
                                value: result.clone(),
                            };

                            context
                                .resolved_variables
                                .insert(variable_name.to_string(), resolved_var);
                        }
                        OperationCategory::ScanTime => {
                            let deferred = DeferredOperation {
                                target_variable: variable_name.to_string(),
                                operation_type: run_op.operation_type,
                                parameters: run_op.parameters.clone(),
                                source_object_id: run_op.extract_object_id(),
                            };

                            context.scan_time_operations.push(deferred);
                        }
                    }
                } else {
                    return Err(ResolutionError::UndefinedVariable {
                        name: variable_name.to_string(),
                        context: "Computed variable has no RUN operation".to_string(),
                    });
                }
            }
        } else {
            // CASE 2: Variable is NOT declared - check RUN operations

            if let Some(run_op) = context
                .runtime_operations
                .iter()
                .find(|op| op.target_variable == variable_name)
            {
                match run_op.categorize(context) {
                    OperationCategory::ResolutionTime => {
                        let result =
                            crate::resolution::runtime_operations::execute_runtime_operation(
                                run_op,
                                &context.resolved_variables,
                            )?;

                        let resolved_var = ResolvedVariable {
                            identifier: variable_name.to_string(),
                            data_type: run_op.operation_type.output_type(),
                            value: result.clone(),
                        };

                        context
                            .resolved_variables
                            .insert(variable_name.to_string(), resolved_var);
                    }
                    OperationCategory::ScanTime => {
                        let deferred = DeferredOperation {
                            target_variable: variable_name.to_string(),
                            operation_type: run_op.operation_type,
                            parameters: run_op.parameters.clone(),
                            source_object_id: run_op.extract_object_id(),
                        };

                        context.scan_time_operations.push(deferred);
                    }
                }
            } else {
                return Err(ResolutionError::UndefinedVariable {
                    name: variable_name.to_string(),
                    context: "Variable not declared and has no RUN operation".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Resolve variable with initial value
    fn resolve_variable_with_initial_value(
        &self,
        variable: &VariableDeclaration,
        context: &ResolutionContext,
    ) -> Result<ResolvedVariable, ResolutionError> {
        let initial_value =
            variable
                .initial_value
                .as_ref()
                .ok_or_else(|| ResolutionError::UndefinedVariable {
                    name: variable.name.clone(),
                    context: "Variable missing initial value".to_string(),
                })?;

        let resolved_value = self.field_resolver.resolve_value(
            initial_value,
            &format!("variable '{}'", variable.name),
            &context.resolved_variables,
        )?;

        Ok(ResolvedVariable {
            identifier: variable.name.clone(),
            data_type: variable.data_type,
            value: resolved_value,
        })
    }

    /// Resolve global state
    fn resolve_global_state(
        &mut self,
        state_id: &str,
        context: &mut ResolutionContext,
    ) -> Result<(), ResolutionError> {
        let state = context
            .global_states
            .iter()
            .find(|s| s.identifier == state_id)
            .ok_or_else(|| ResolutionError::UndefinedGlobalState {
                name: state_id.to_string(),
                context: "DAG resolution".to_string(),
            })?
            .clone();

        let resolved_state = self.resolve_state_fields(&state, &context.resolved_variables)?;
        context
            .resolved_global_states
            .insert(state_id.to_string(), resolved_state);
        Ok(())
    }

    /// Resolve global object
    fn resolve_global_object(
        &mut self,
        object_id: &str,
        context: &mut ResolutionContext,
    ) -> Result<(), ResolutionError> {
        let object = context
            .global_objects
            .iter()
            .find(|o| o.identifier == object_id)
            .ok_or_else(|| ResolutionError::UndefinedGlobalObject {
                name: object_id.to_string(),
                context: "DAG resolution".to_string(),
            })?
            .clone();

        let resolved_object = self.resolve_object_fields(&object, &context.resolved_variables)?;
        context
            .resolved_global_objects
            .insert(object_id.to_string(), resolved_object);
        Ok(())
    }

    /// Resolve set operation
    fn resolve_set_operation(
        &mut self,
        set_id: &str,
        context: &mut ResolutionContext,
    ) -> Result<(), ResolutionError> {
        let set_operation = context
            .set_operations
            .iter()
            .find(|s| s.set_id == set_id)
            .ok_or_else(|| ResolutionError::SetOperationFailed {
                set_id: set_id.to_string(),
                reason: "Set operation not found".to_string(),
            })?
            .clone();

        let resolved_set =
            crate::resolution::set_operations::execute_set_operation(&set_operation, context)?;

        context
            .resolved_sets
            .insert(set_id.to_string(), resolved_set);
        Ok(())
    }

    /// Resolve local symbols (CTN-scoped states and objects)
    fn resolve_local_symbols(
        &mut self,
        context: &mut ResolutionContext,
    ) -> Result<(), ResolutionError> {
        // Resolve local objects
        for (ctn_id, local_object) in context.ctn_local_objects.clone() {
            let resolved_object =
                self.resolve_object_fields(&local_object, &context.resolved_variables)?;
            context
                .resolved_local_objects
                .insert(ctn_id, resolved_object);
        }

        // Resolve local states
        for (ctn_id, local_states) in context.ctn_local_states.clone() {
            let mut resolved_states = Vec::new();
            for state in local_states {
                let resolved_state =
                    self.resolve_state_fields(&state, &context.resolved_variables)?;
                resolved_states.push(resolved_state);
            }
            context
                .resolved_local_states
                .insert(ctn_id, resolved_states);
        }

        Ok(())
    }

    // ========== Helper Methods ==========

    fn determine_symbol_type(
        &self,
        symbol_id: &str,
        context: &ResolutionContext,
    ) -> Option<SymbolType> {
        // Check variables first (includes computed variables)
        if context.variables.iter().any(|v| v.name == symbol_id) {
            return Some(SymbolType::Variable);
        }

        if context
            .runtime_operations
            .iter()
            .any(|op| op.target_variable == symbol_id)
        {
            return Some(SymbolType::Variable);
        }

        if context.set_operations.iter().any(|s| s.set_id == symbol_id) {
            return Some(SymbolType::SetOperation);
        }

        if context
            .global_states
            .iter()
            .any(|s| s.identifier == symbol_id)
        {
            return Some(SymbolType::GlobalState);
        }

        if context
            .global_objects
            .iter()
            .any(|o| o.identifier == symbol_id)
        {
            return Some(SymbolType::GlobalObject);
        }

        None
    }

    fn count_symbols(&self, context: &ResolutionContext) -> usize {
        context.variables.len()
            + context.runtime_operations.len()
            + context.set_operations.len()
            + context.global_states.len()
            + context.global_objects.len()
    }

    fn resolve_state_fields(
        &self,
        state: &StateDeclaration,
        resolved_variables: &HashMap<String, ResolvedVariable>,
    ) -> Result<ResolvedState, ResolutionError> {
        let mut resolved_fields = Vec::new();
        let mut resolved_record_checks = Vec::new();

        for field in &state.fields {
            let resolved_field = self.resolve_single_state_field(field, resolved_variables)?;
            resolved_fields.push(resolved_field);
        }

        for record_check in &state.record_checks {
            let resolved_record_check =
                self.resolve_record_check(record_check, resolved_variables)?;
            resolved_record_checks.push(resolved_record_check);
        }

        Ok(ResolvedState {
            identifier: state.identifier.clone(),
            resolved_fields,
            resolved_record_checks,
            is_global: state.is_global,
        })
    }

    fn resolve_single_state_field(
        &self,
        field: &crate::types::StateField,
        resolved_variables: &HashMap<String, ResolvedVariable>,
    ) -> Result<crate::types::ResolvedStateField, ResolutionError> {
        let resolved_value = self.field_resolver.resolve_value(
            &field.value,
            &format!("state field '{}'", field.name),
            resolved_variables,
        )?;

        Ok(crate::types::ResolvedStateField {
            name: field.name.clone(),
            data_type: field.data_type,
            operation: field.operation,
            value: resolved_value,
            entity_check: field.entity_check,
        })
    }

    fn resolve_record_check(
        &self,
        record_check: &RecordCheck,
        resolved_variables: &HashMap<String, ResolvedVariable>,
    ) -> Result<ResolvedRecordCheck, ResolutionError> {
        let resolved_content = match &record_check.content {
            RecordContent::Direct { operation, value } => {
                let resolved_value = self.field_resolver.resolve_value(
                    value,
                    "record direct operation",
                    resolved_variables,
                )?;
                ResolvedRecordContent::Direct {
                    operation: *operation,
                    value: resolved_value,
                }
            }
            RecordContent::Nested { fields } => {
                let mut resolved_fields = Vec::new();
                for field in fields {
                    let resolved_value = self.field_resolver.resolve_value(
                        &field.value,
                        &format!("record field '{}'", field.path.to_dot_notation()),
                        resolved_variables,
                    )?;

                    resolved_fields.push(ResolvedRecordField {
                        path: field.path.clone(),
                        data_type: field.data_type,
                        operation: field.operation,
                        value: resolved_value,
                        entity_check: field.entity_check,
                    });
                }
                ResolvedRecordContent::Nested {
                    fields: resolved_fields,
                }
            }
        };

        Ok(ResolvedRecordCheck {
            data_type: record_check.data_type,
            content: resolved_content,
        })
    }

    fn resolve_object_fields(
        &self,
        object: &ObjectDeclaration,
        resolved_variables: &HashMap<String, ResolvedVariable>,
    ) -> Result<ResolvedObject, ResolutionError> {
        let mut resolved_elements = Vec::new();

        for element in &object.elements {
    let resolved_element = match element {
        ObjectElement::Field { name, value } => {
            let resolved_value = self.field_resolver.resolve_value(
                value,
                &format!("object field '{}'", name),
                resolved_variables,
            )?;
            crate::types::object::ResolvedObjectElement::Field {
                name: name.clone(),
                value: resolved_value,
            }
        }
        ObjectElement::SetRef { set_id } => {
            crate::types::object::ResolvedObjectElement::SetRef {
                set_id: set_id.clone(),
            }
        }
        // NEW: Preserve Filter during resolution
        ObjectElement::Filter(filter_spec) => {
            crate::types::object::ResolvedObjectElement::Filter(
                crate::types::filter::ResolvedFilterSpec::new(
                    filter_spec.action,
                    filter_spec.state_refs.clone(),
                )
            )
        }
        // Metadata/configuration elements - preserve as-is
        ObjectElement::Module { field, value } => {
            crate::types::object::ResolvedObjectElement::Module {
                field: *field,
                value: value.clone(),
            }
        }
        ObjectElement::Parameter { data_type, fields } => {
            // Convert to RecordData
            let record = crate::types::common::RecordData::from_field_pairs(fields);
            
            crate::types::object::ResolvedObjectElement::Parameter {
                data_type: *data_type,
                data: record,
            }
        }
        ObjectElement::Select { data_type, fields } => {
            // Convert to RecordData
            let record = crate::types::common::RecordData::from_field_pairs(fields);
          
            crate::types::object::ResolvedObjectElement::Select {
                data_type: *data_type,
                data: record,
            }
        }
        ObjectElement::Behavior { values } => {
            crate::types::object::ResolvedObjectElement::Behavior {
                values: values.clone(),
            }
        }
    };
    resolved_elements.push(resolved_element);
}

        Ok(ResolvedObject {
            identifier: object.identifier.clone(),
            resolved_elements,
            is_global: object.is_global,
        })
    }
}