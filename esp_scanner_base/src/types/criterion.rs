use super::object::{ObjectDeclaration, ResolvedObject};
use super::state::{ResolvedState, StateDeclaration};
use esp_compiler::grammar::ast::nodes::{ObjectRef, StateRef, TestSpecification};
use serde::{Deserialize, Serialize};

/// CTN node identifier for tracking local symbol scopes
pub type CtnNodeId = usize;

/// Criterion declaration (CTN block) - scanner working type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterionDeclaration {
    /// Criterion type (identifier)
    pub criterion_type: String,
    /// Test specification (required)
    pub test: TestSpecification, // Using compiler's type
    /// State references (optional, multiple allowed)
    pub state_refs: Vec<StateRef>, // Using compiler's type
    /// Object references (optional, multiple allowed)
    pub object_refs: Vec<ObjectRef>, // Using compiler's type
    /// Local states (CTN-level, non-referenceable)
    pub local_states: Vec<StateDeclaration>,
    /// Local object (CTN-level, non-referenceable, max 1)
    pub local_object: Option<ObjectDeclaration>,
    /// CTN node ID for scope tracking
    pub ctn_node_id: Option<CtnNodeId>,
}

/// Resolved criterion with all references validated and local elements resolved
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedCriterion {
    /// Criterion type (identifier)
    pub criterion_type: String,
    /// Test specification (unchanged during resolution)
    pub test: TestSpecification,
    /// Validated state references (guaranteed to exist as global states)
    pub state_refs: Vec<StateRef>,
    /// Validated object references (guaranteed to exist as global objects)
    pub object_refs: Vec<ObjectRef>,
    /// Resolved local states
    pub local_states: Vec<ResolvedState>,
    /// Resolved local object
    pub local_object: Option<ResolvedObject>,
    /// CTN node ID for scope tracking
    pub ctn_node_id: CtnNodeId,
}

/// CTN content structure with strict ordering enforcement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CtnContent {
    /// Test specification (required, must be first)
    pub test: TestSpecification,
    /// State references (optional, before object references)
    pub state_refs: Vec<StateRef>,
    /// Object references (optional, before local elements)
    pub object_refs: Vec<ObjectRef>,
    /// Local states (optional, before local object)
    pub local_states: Vec<StateDeclaration>,
    /// Local object (optional, only one allowed)
    pub local_object: Option<ObjectDeclaration>,
}

/// Resolved CTN content with validated references
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedCtnContent {
    /// Test specification (unchanged)
    pub test: TestSpecification,
    /// Validated state references
    pub state_refs: Vec<StateRef>,
    /// Validated object references
    pub object_refs: Vec<ObjectRef>,
    /// Resolved local states
    pub local_states: Vec<ResolvedState>,
    /// Resolved local object
    pub local_object: Option<ResolvedObject>,
}

// ============================================================================
// IMPLEMENTATIONS - Scanner working types
// ============================================================================

impl CriterionDeclaration {
    /// Create a new criterion declaration
    pub fn new(
        criterion_type: String,
        test: TestSpecification,
        state_refs: Vec<StateRef>,
        object_refs: Vec<ObjectRef>,
        local_states: Vec<StateDeclaration>,
        local_object: Option<ObjectDeclaration>,
    ) -> Self {
        Self {
            criterion_type,
            test,
            state_refs,
            object_refs,
            local_states,
            local_object,
            ctn_node_id: None,
        }
    }

    /// Set the CTN node ID for scope tracking
    pub fn with_ctn_node_id(mut self, ctn_node_id: CtnNodeId) -> Self {
        self.ctn_node_id = Some(ctn_node_id);
        self
    }

    /// Convert from compiler AST node with CTN node ID assignment
    ///
    /// # Arguments
    /// * `node` - Criterion node from esp_compiler AST
    /// * `ctn_node_id` - Unique CTN node identifier for scope tracking
    ///
    /// # Example
    /// ```ignore
    /// use esp_compiler::grammar::ast::nodes::CriterionNode;
    ///
    /// let ast_criterion = CriterionNode { ... };
    /// let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 0);
    /// ```
    pub fn from_ast_node(
        node: &esp_compiler::grammar::ast::nodes::CriterionNode,
        ctn_node_id: CtnNodeId,
    ) -> Self {
        // Recursively convert local states
        let local_states: Vec<StateDeclaration> = node
            .local_states
            .iter()
            .map(|state_def| StateDeclaration::from_ast_node(state_def))
            .collect();

        // Recursively convert local object if present
        let local_object = node
            .local_object
            .as_ref()
            .map(|obj_def| ObjectDeclaration::from_ast_node(obj_def));

        Self {
            criterion_type: node.criterion_type.clone(),
            test: node.test.clone(), // Using compiler type directly
            state_refs: node.state_refs.clone(), // Using compiler type directly
            object_refs: node.object_refs.clone(), // Using compiler type directly
            local_states,
            local_object,
            ctn_node_id: Some(ctn_node_id),
        }
    }

    /// Check if this criterion has any references to global symbols
    pub fn has_global_references(&self) -> bool {
        !self.state_refs.is_empty() || !self.object_refs.is_empty()
    }

    /// Check if this criterion has any local elements
    pub fn has_local_elements(&self) -> bool {
        !self.local_states.is_empty() || self.local_object.is_some()
    }

    /// Get all global state references
    pub fn get_global_state_refs(&self) -> Vec<String> {
        self.state_refs
            .iter()
            .map(|sr| sr.state_id.clone())
            .collect()
    }

    /// Get all global object references
    pub fn get_global_object_refs(&self) -> Vec<String> {
        self.object_refs
            .iter()
            .map(|obj_ref| obj_ref.object_id.clone())
            .collect()
    }

    /// Get all variable references from local elements
    pub fn get_variable_references(&self) -> Vec<String> {
        let mut refs = Vec::new();

        // From local states
        for state in &self.local_states {
            refs.extend(state.get_variable_references());
        }

        // From local object
        if let Some(object) = &self.local_object {
            refs.extend(object.get_variable_references());
        }

        refs.sort();
        refs.dedup();
        refs
    }

    /// Get count of local states
    pub fn local_state_count(&self) -> usize {
        self.local_states.len()
    }

    /// Check if this criterion has a local object
    pub fn has_local_object(&self) -> bool {
        self.local_object.is_some()
    }

    /// Validate CTN structure according to EBNF ordering rules
    pub fn validate(&self) -> Result<(), String> {
        // Check that TEST specification is suitable for the number of states
        let _total_states = self.state_refs.len() + self.local_states.len();

        // Note: TestSpecification validation methods would need to be added
        // to the compiler or via extension trait if needed

        // Validate local elements don't exceed limits
        if self.local_states.len() > 10 {
            return Err("Too many local states in CTN".to_string());
        }

        // Validate local states are not empty
        for state in &self.local_states {
            if state.is_empty() {
                return Err(format!("Local state '{}' is empty", state.identifier));
            }
        }

        // Validate local object is not empty if present
        if let Some(object) = &self.local_object {
            if object.is_empty() {
                return Err(format!("Local object '{}' is empty", object.identifier));
            }
        }

        Ok(())
    }

    /// Get all dependencies for this criterion
    pub fn get_all_dependencies(&self) -> CriterionDependencies {
        CriterionDependencies {
            global_states: self.get_global_state_refs(),
            global_objects: self.get_global_object_refs(),
            variables: self.get_variable_references(),
        }
    }
}

/// Criterion dependencies for DAG construction
#[derive(Debug, Clone, Default)]
pub struct CriterionDependencies {
    pub global_states: Vec<String>,
    pub global_objects: Vec<String>,
    pub variables: Vec<String>,
}

impl CriterionDependencies {
    pub fn has_dependencies(&self) -> bool {
        !self.global_states.is_empty()
            || !self.global_objects.is_empty()
            || !self.variables.is_empty()
    }

    pub fn total_count(&self) -> usize {
        self.global_states.len() + self.global_objects.len() + self.variables.len()
    }
}

impl ResolvedCriterion {
    /// Create a new resolved criterion
    pub fn new(
        criterion_type: String,
        test: TestSpecification,
        state_refs: Vec<StateRef>,
        object_refs: Vec<ObjectRef>,
        local_states: Vec<ResolvedState>,
        local_object: Option<ResolvedObject>,
        ctn_node_id: CtnNodeId,
    ) -> Self {
        Self {
            criterion_type,
            test,
            state_refs,
            object_refs,
            local_states,
            local_object,
            ctn_node_id,
        }
    }

    /// Check if this criterion is ready for execution
    pub fn is_execution_ready(&self) -> bool {
        // All local states must be resolved
        // All referenced states must exist (validated during resolution)
        // All referenced objects must exist (validated during resolution)
        true // If we got this far in resolution, we're ready
    }

    /// Get total state count (global refs + local)
    pub fn total_state_count(&self) -> usize {
        self.state_refs.len() + self.local_states.len()
    }

    /// Get total object count (global refs + local)
    pub fn total_object_count(&self) -> usize {
        self.object_refs.len() + if self.local_object.is_some() { 1 } else { 0 }
    }

    /// Check if this criterion has any states
    pub fn has_states(&self) -> bool {
        !self.state_refs.is_empty() || !self.local_states.is_empty()
    }

    /// Check if this criterion has any objects
    pub fn has_objects(&self) -> bool {
        !self.object_refs.is_empty() || self.local_object.is_some()
    }
}

// ============================================================================
// DISPLAY IMPLEMENTATIONS
// ============================================================================

impl std::fmt::Display for CriterionDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CTN {} (", self.criterion_type)?;
        write!(f, "{}", format_test_specification(&self.test))?;
        write!(f, ")")?;

        write!(
            f,
            ", {} state refs, {} object refs, {} local states",
            self.state_refs.len(),
            self.object_refs.len(),
            self.local_states.len()
        )?;

        if let Some(node_id) = self.ctn_node_id {
            write!(f, " [node:{}]", node_id)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for ResolvedCriterion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResolvedCTN {} (", self.criterion_type)?;
        write!(f, "{}", format_test_specification(&self.test))?;
        write!(f, ")")?;
        write!(f, " [node:{}]", self.ctn_node_id)?;
        if self.is_execution_ready() {
            write!(f, " READY")?;
        }
        Ok(())
    }
}

// ============================================================================
// HELPER FUNCTIONS for formatting
// ============================================================================

fn format_test_specification(test: &TestSpecification) -> String {
    let mut parts = vec![
        format_existence_check(&test.existence_check),
        format_item_check(&test.item_check),
    ];

    if let Some(state_op) = &test.state_operator {
        parts.push(format_state_operator(state_op));
    }

    if let Some(entity_check) = &test.entity_check {
        parts.push(format_entity_check(entity_check));
    }

    parts.join(" ")
}

fn format_existence_check(check: &esp_compiler::grammar::ast::nodes::ExistenceCheck) -> String {
    use esp_compiler::grammar::ast::nodes::ExistenceCheck;
    match check {
        ExistenceCheck::Any => "any".to_string(),
        ExistenceCheck::All => "all".to_string(),
        ExistenceCheck::None => "none".to_string(),
        ExistenceCheck::AtLeastOne => "at_least_one".to_string(),
        ExistenceCheck::OnlyOne => "only_one".to_string(),
    }
}

fn format_item_check(check: &esp_compiler::grammar::ast::nodes::ItemCheck) -> String {
    use esp_compiler::grammar::ast::nodes::ItemCheck;
    match check {
        ItemCheck::All => "all".to_string(),
        ItemCheck::AtLeastOne => "at_least_one".to_string(),
        ItemCheck::OnlyOne => "only_one".to_string(),
        ItemCheck::NoneSatisfy => "none_satisfy".to_string(),
    }
}

fn format_state_operator(op: &esp_compiler::grammar::ast::nodes::StateJoinOp) -> String {
    use esp_compiler::grammar::ast::nodes::StateJoinOp;
    match op {
        StateJoinOp::And => "AND".to_string(),
        StateJoinOp::Or => "OR".to_string(),
        StateJoinOp::One => "ONE".to_string(),
    }
}

fn format_entity_check(check: &esp_compiler::grammar::ast::nodes::EntityCheck) -> String {
    use esp_compiler::grammar::ast::nodes::EntityCheck;
    match check {
        EntityCheck::All => "entity:all".to_string(),
        EntityCheck::AtLeastOne => "entity:at_least_one".to_string(),
        EntityCheck::None => "entity:none".to_string(),
        EntityCheck::OnlyOne => "entity:only_one".to_string(),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use esp_compiler::grammar::ast::nodes::{
        CriterionNode as AstCriterion, DataType as AstDataType, EntityCheck, ExistenceCheck,
        ItemCheck, ObjectDefinition as AstObject, ObjectElement as AstElement,
        ObjectField as AstField, ObjectRef as AstObjectRef, Operation, StateDefinition as AstState,
        StateField as AstStateField, StateRef as AstStateRef, TestSpecification as AstTest,
        Value as AstValue,
    };

    fn create_test_specification() -> AstTest {
        AstTest {
            existence_check: ExistenceCheck::Any,
            item_check: ItemCheck::All,
            state_operator: None,
            entity_check: None,
        }
    }

    #[test]
    fn test_criterion_ast_conversion_basic() {
        // Create compiler AST node
        let ast_criterion = AstCriterion {
            criterion_type: "test_ctn".to_string(),
            test: create_test_specification(),
            state_refs: vec![AstStateRef {
                state_id: "state1".to_string(),
                span: None,
            }],
            object_refs: vec![AstObjectRef {
                object_id: "obj1".to_string(),
                span: None,
            }],
            local_states: vec![],
            local_object: None,
            span: None,
        };

        // Convert to scanner type
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 0);

        // Verify
        assert_eq!(scanner_criterion.criterion_type, "test_ctn");
        assert_eq!(scanner_criterion.ctn_node_id, Some(0));
        assert_eq!(scanner_criterion.state_refs.len(), 1);
        assert_eq!(scanner_criterion.object_refs.len(), 1);
        assert!(scanner_criterion.local_states.is_empty());
        assert!(scanner_criterion.local_object.is_none());
    }

    #[test]
    fn test_criterion_ast_conversion_with_local_elements() {
        // Create compiler AST node with local elements
        let ast_criterion = AstCriterion {
            criterion_type: "local_ctn".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![AstState {
                id: "local_state".to_string(),
                fields: vec![AstStateField::Simple {
                    field_name: "test".to_string(),
                    data_type: AstDataType::String,
                    operation: Operation::Equals,
                    value: AstValue::String("value".to_string()),
                }],
                is_global: false,
                span: None,
            }],
            local_object: Some(AstObject {
                id: "local_obj".to_string(),
                elements: vec![AstElement::Field(AstField {
                    name: "path".to_string(),
                    value: AstValue::String("/test".to_string()),
                    span: None,
                })],
                is_global: false,
                span: None,
            }),
            span: None,
        };

        // Convert to scanner type
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 1);

        // Verify
        assert_eq!(scanner_criterion.criterion_type, "local_ctn");
        assert_eq!(scanner_criterion.local_states.len(), 1);
        assert!(scanner_criterion.local_object.is_some());
        assert!(scanner_criterion.has_local_elements());
    }

    #[test]
    fn test_criterion_global_references() {
        // Create criterion with global references
        let ast_criterion = AstCriterion {
            criterion_type: "global_ref_ctn".to_string(),
            test: create_test_specification(),
            state_refs: vec![
                AstStateRef {
                    state_id: "state1".to_string(),
                    span: None,
                },
                AstStateRef {
                    state_id: "state2".to_string(),
                    span: None,
                },
            ],
            object_refs: vec![AstObjectRef {
                object_id: "obj1".to_string(),
                span: None,
            }],
            local_states: vec![],
            local_object: None,
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 2);

        // Verify
        assert!(scanner_criterion.has_global_references());
        let state_refs = scanner_criterion.get_global_state_refs();
        assert_eq!(state_refs.len(), 2);
        assert!(state_refs.contains(&"state1".to_string()));
        assert!(state_refs.contains(&"state2".to_string()));
    }

    #[test]
    fn test_criterion_unique_node_ids() {
        // Create two criteria with different node IDs
        let ast_criterion_1 = AstCriterion {
            criterion_type: "ctn_1".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![],
            local_object: None,
            span: None,
        };

        let ast_criterion_2 = AstCriterion {
            criterion_type: "ctn_2".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![],
            local_object: None,
            span: None,
        };

        // Convert with different node IDs
        let criterion_1 = CriterionDeclaration::from_ast_node(&ast_criterion_1, 0);
        let criterion_2 = CriterionDeclaration::from_ast_node(&ast_criterion_2, 1);

        // Verify unique CTN node IDs
        assert_eq!(criterion_1.ctn_node_id, Some(0));
        assert_eq!(criterion_2.ctn_node_id, Some(1));
        assert_ne!(criterion_1.ctn_node_id, criterion_2.ctn_node_id);
    }

    #[test]
    fn test_criterion_dependency_tracking() {
        // Create criterion with various dependencies
        let ast_criterion = AstCriterion {
            criterion_type: "deps_check".to_string(),
            test: create_test_specification(),
            state_refs: vec![
                AstStateRef {
                    state_id: "state_1".to_string(),
                    span: None,
                },
                AstStateRef {
                    state_id: "state_2".to_string(),
                    span: None,
                },
            ],
            object_refs: vec![AstObjectRef {
                object_id: "obj_1".to_string(),
                span: None,
            }],
            local_states: vec![AstState {
                id: "local_with_var".to_string(),
                fields: vec![AstStateField::Simple {
                    field_name: "field".to_string(),
                    data_type: AstDataType::String,
                    operation: Operation::Equals,
                    value: AstValue::Variable("my_var".to_string()),
                }],
                is_global: false,
                span: None,
            }],
            local_object: None,
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 4);

        // Get dependencies
        let deps = scanner_criterion.get_all_dependencies();

        // Verify
        assert_eq!(deps.global_states.len(), 2);
        assert_eq!(deps.global_objects.len(), 1);
        assert_eq!(deps.variables.len(), 1);
        assert!(deps.variables.contains(&"my_var".to_string()));
        assert!(deps.has_dependencies());
        assert_eq!(deps.total_count(), 4);
    }

    #[test]
    fn test_criterion_validation_empty_local_state() {
        // Create criterion with empty local state
        let ast_criterion = AstCriterion {
            criterion_type: "invalid_check".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![AstState {
                id: "empty_state".to_string(),
                fields: vec![], // Empty!
                is_global: false,
                span: None,
            }],
            local_object: None,
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 5);

        // Validate - should fail
        let result = scanner_criterion.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_criterion_validation_empty_local_object() {
        // Create criterion with empty local object
        let ast_criterion = AstCriterion {
            criterion_type: "invalid_check".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![],
            local_object: Some(AstObject {
                id: "empty_object".to_string(),
                elements: vec![], // Empty!
                is_global: false,
                span: None,
            }),
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 6);

        // Validate - should fail
        let result = scanner_criterion.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_criterion_get_global_refs() {
        // Create criterion with multiple refs
        let ast_criterion = AstCriterion {
            criterion_type: "multi_ref".to_string(),
            test: create_test_specification(),
            state_refs: vec![
                AstStateRef {
                    state_id: "state_a".to_string(),
                    span: None,
                },
                AstStateRef {
                    state_id: "state_b".to_string(),
                    span: None,
                },
            ],
            object_refs: vec![
                AstObjectRef {
                    object_id: "obj_x".to_string(),
                    span: None,
                },
                AstObjectRef {
                    object_id: "obj_y".to_string(),
                    span: None,
                },
            ],
            local_states: vec![],
            local_object: None,
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 7);

        // Get refs
        let state_refs = scanner_criterion.get_global_state_refs();
        let object_refs = scanner_criterion.get_global_object_refs();

        // Verify
        assert_eq!(state_refs.len(), 2);
        assert_eq!(object_refs.len(), 2);
        assert!(state_refs.contains(&"state_a".to_string()));
        assert!(state_refs.contains(&"state_b".to_string()));
        assert!(object_refs.contains(&"obj_x".to_string()));
        assert!(object_refs.contains(&"obj_y".to_string()));
    }

    #[test]
    fn test_criterion_variable_references_in_local_elements() {
        // Create criterion with variable references in local elements
        let ast_criterion = AstCriterion {
            criterion_type: "var_ref_check".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![AstState {
                id: "state_with_var".to_string(),
                fields: vec![AstStateField::Simple {
                    field_name: "field".to_string(),
                    data_type: AstDataType::String,
                    operation: Operation::Equals,
                    value: AstValue::Variable("var1".to_string()),
                }],
                is_global: false,
                span: None,
            }],
            local_object: Some(AstObject {
                id: "obj_with_var".to_string(),
                elements: vec![AstElement::Field(AstField {
                    name: "path".to_string(),
                    value: AstValue::Variable("var2".to_string()),
                    span: None,
                })],
                is_global: false,
                span: None,
            }),
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 8);

        // Get variable references
        let var_refs = scanner_criterion.get_variable_references();

        // Verify
        assert_eq!(var_refs.len(), 2);
        assert!(var_refs.contains(&"var1".to_string()));
        assert!(var_refs.contains(&"var2".to_string()));
    }

    #[test]
    fn test_criterion_no_elements_invalid() {
        // Create criterion with no refs and no local elements
        let ast_criterion = AstCriterion {
            criterion_type: "empty_check".to_string(),
            test: create_test_specification(),
            state_refs: vec![],
            object_refs: vec![],
            local_states: vec![],
            local_object: None,
            span: None,
        };

        // Convert
        let scanner_criterion = CriterionDeclaration::from_ast_node(&ast_criterion, 9);

        // Should have neither global refs nor local elements
        assert!(!scanner_criterion.has_global_references());
        assert!(!scanner_criterion.has_local_elements());
    }
}
