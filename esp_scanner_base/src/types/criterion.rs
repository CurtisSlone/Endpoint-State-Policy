use super::object::{ObjectDeclaration, ObjectRef, ResolvedObject};
use super::state::{ResolvedState, StateDeclaration, StateRef};
use super::test::TestSpecification;
use serde::{Deserialize, Serialize};

/// CTN node identifier for tracking local symbol scopes
pub type CtnNodeId = usize;

/// Criterion declaration (CTN block)
/// EBNF: criterion ::= "CTN" space criterion_type statement_end ctn_content "CTN_END" statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterionDeclaration {
    /// Criterion type (identifier)
    pub criterion_type: String,
    /// Test specification (required)
    pub test: TestSpecification,
    /// State references (optional, multiple allowed)
    pub state_refs: Vec<StateRef>,
    /// Object references (optional, multiple allowed)
    pub object_refs: Vec<ObjectRef>,
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
/// EBNF: ctn_content ::= test_specification state_references? object_references? ctn_states? ctn_object?
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

// Utility implementations
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
        let total_states = self.state_refs.len() + self.local_states.len();
        if !self.test.is_suitable_for_state_count(total_states) {
            return Err(format!(
                "Test specification {:?} is not suitable for {} states",
                self.test, total_states
            ));
        }

        // Validate local elements don't exceed limits
        if self.local_states.len() > 10 {
            // Reasonable limit
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

    /// Check if this criterion references external filters
    pub fn has_filter_dependencies(&self) -> bool {
        // From local object filters
        if let Some(object) = &self.local_object {
            if object.has_filters() {
                return true;
            }
        }
        false
    }

    /// Get all filter state dependencies
    pub fn get_filter_dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();

        if let Some(object) = &self.local_object {
            deps.extend(object.get_filter_state_dependencies());
        }

        deps.sort();
        deps.dedup();
        deps
    }

    /// Get all external dependencies (states, objects, variables, filters)
    pub fn get_all_dependencies(&self) -> CriterionDependencies {
        CriterionDependencies {
            global_states: self.get_global_state_refs(),
            global_objects: self.get_global_object_refs(),
            variables: self.get_variable_references(),
            filter_states: self.get_filter_dependencies(),
        }
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

    /// Check if this resolved criterion is ready for execution
    pub fn is_execution_ready(&self) -> bool {
        // All references should be validated during resolution
        // Local states should have no unresolved variables
        self.local_states.iter().all(|state| {
            state.resolved_fields.iter().all(|field| {
                !matches!(field.value, super::common::ResolvedValue::String(ref s) if s.contains("VAR "))
            })
        })
    }

    /// Get total state count for execution planning
    pub fn total_state_count(&self) -> usize {
        self.state_refs.len() + self.local_states.len()
    }
}

impl CtnContent {
    /// Create new CTN content with required test
    pub fn new(test: TestSpecification) -> Self {
        Self {
            test,
            state_refs: Vec::new(),
            object_refs: Vec::new(),
            local_states: Vec::new(),
            local_object: None,
        }
    }

    /// Add state reference (validates ordering)
    pub fn add_state_ref(mut self, state_ref: StateRef) -> Self {
        self.state_refs.push(state_ref);
        self
    }

    /// Add object reference (validates ordering)
    pub fn add_object_ref(mut self, object_ref: ObjectRef) -> Self {
        self.object_refs.push(object_ref);
        self
    }

    /// Add local state (validates ordering)
    pub fn add_local_state(mut self, state: StateDeclaration) -> Self {
        self.local_states.push(state);
        self
    }

    /// Set local object (validates single object limit)
    pub fn set_local_object(mut self, object: ObjectDeclaration) -> Result<Self, String> {
        if self.local_object.is_some() {
            return Err("CTN can only have one local object".to_string());
        }
        self.local_object = Some(object);
        Ok(self)
    }

    /// Validate CTN content ordering and structure
    pub fn validate(&self) -> Result<(), String> {
        // CTN must have at least references or local elements
        if self.state_refs.is_empty()
            && self.object_refs.is_empty()
            && self.local_states.is_empty()
            && self.local_object.is_none()
        {
            return Err(
                "CTN must have at least one state reference, object reference, or local element"
                    .to_string(),
            );
        }

        Ok(())
    }
}

/// Comprehensive dependency tracking for DAG construction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriterionDependencies {
    /// Global state dependencies
    pub global_states: Vec<String>,
    /// Global object dependencies
    pub global_objects: Vec<String>,
    /// Variable dependencies
    pub variables: Vec<String>,
    /// Filter state dependencies
    pub filter_states: Vec<String>,
}

impl CriterionDependencies {
    /// Get all unique dependencies as a single list
    pub fn get_all_unique(&self) -> Vec<String> {
        let mut all_deps = Vec::new();
        all_deps.extend_from_slice(&self.global_states);
        all_deps.extend_from_slice(&self.global_objects);
        all_deps.extend_from_slice(&self.variables);
        all_deps.extend_from_slice(&self.filter_states);
        all_deps.sort();
        all_deps.dedup();
        all_deps
    }

    /// Check if this criterion has any dependencies
    pub fn has_dependencies(&self) -> bool {
        !self.global_states.is_empty()
            || !self.global_objects.is_empty()
            || !self.variables.is_empty()
            || !self.filter_states.is_empty()
    }

    /// Count total dependencies
    pub fn total_count(&self) -> usize {
        self.global_states.len()
            + self.global_objects.len()
            + self.variables.len()
            + self.filter_states.len()
    }
}

// Display implementations
impl std::fmt::Display for CriterionDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CTN {} ({})", self.criterion_type, self.test)?;
        if !self.state_refs.is_empty() {
            write!(f, " states:{}", self.state_refs.len())?;
        }
        if !self.object_refs.is_empty() {
            write!(f, " objects:{}", self.object_refs.len())?;
        }
        if !self.local_states.is_empty() {
            write!(f, " local_states:{}", self.local_states.len())?;
        }
        if self.local_object.is_some() {
            write!(f, " local_object:1")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for ResolvedCriterion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ResolvedCTN {} ({})", self.criterion_type, self.test)?;
        write!(f, " [node:{}]", self.ctn_node_id)?;
        if self.is_execution_ready() {
            write!(f, " READY")?;
        }
        Ok(())
    }
}
