use super::common::{DataType, ResolvedValue};
use super::criterion::{CriterionDeclaration, CtnNodeId, ResolvedCriterion};
use super::object::{ObjectDeclaration, ResolvedObject};
use super::runtime_operation::{RunParameter, RuntimeOperation};
use super::set::{ResolvedSetOperation, SetOperation};
use super::state::{ResolvedState, StateDeclaration};
use super::variable::{ResolvedVariable, VariableDeclaration};
use crate::types::metadata::MetaDataBlock;
use crate::types::CriteriaRoot;
use crate::types::RuntimeOperationType;
use crate::LogicalOp;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Complete resolution context for the DAG resolution engine
#[derive(Debug)]
pub struct ResolutionContext {
    // Source data from JSON parser
    pub variables: Vec<VariableDeclaration>,
    pub global_objects: Vec<ObjectDeclaration>,
    pub global_states: Vec<StateDeclaration>,
    pub set_operations: Vec<SetOperation>,
    pub runtime_operations: Vec<RuntimeOperation>,
    pub criteria_root: CriteriaRoot,
    pub relationships: Vec<SymbolRelationship>,

    // Resolution storage - populated during DAG traversal
    pub resolved_variables: HashMap<String, ResolvedVariable>,
    pub resolved_global_objects: HashMap<String, ResolvedObject>,
    pub resolved_global_states: HashMap<String, ResolvedState>,
    pub resolved_sets: HashMap<String, ResolvedSetOperation>,
    pub resolved_criteria: HashMap<CtnNodeId, ResolvedCriterion>,

    // Local symbol storage by CTN node ID
    pub ctn_local_objects: HashMap<CtnNodeId, ObjectDeclaration>,
    pub ctn_local_states: HashMap<CtnNodeId, Vec<StateDeclaration>>,
    pub resolved_local_objects: HashMap<CtnNodeId, ResolvedObject>,
    pub resolved_local_states: HashMap<CtnNodeId, Vec<ResolvedState>>,

    // NEW: Computed variable tracking for DAG resolution
    pub computed_variables: HashSet<String>, // Variables without initial values
    pub literal_variables: HashSet<String>,  // Variables with literal initial values
    pub reference_variables: HashMap<String, String>, // var_name -> referenced_var_name

    // NEW: Deferred validation for DAG resolution
    pub parsing_errors: Vec<String>, // Non-fatal parsing issues
    pub deferred_validations: Vec<DeferredValidation>, // Validations to do later

    // NEW: Dependency tracking for DAG
    pub hard_dependencies: HashMap<String, Vec<String>>,
    pub soft_dependencies: HashMap<String, Vec<String>>,

    // DAG resolution state
    pub resolution_order: Vec<ResolutionNode>,
    pub dependency_graph: DependencyGraph,
    pub resolution_status: HashMap<String, ResolutionStatus>,

    // Enhanced memoization cache for performance
    pub memoization_cache: HashMap<String, ResolvedValue>,

    // Error collection
    pub errors: Vec<ResolutionError>,
    pub warnings: Vec<ResolutionWarning>,

    // ICS Metadata
    pub metadata: Option<MetaDataBlock>,

    // RUN Operation Type tracking
    pub resolution_time_operations: Vec<RuntimeOperation>,
    pub scan_time_operations: Vec<DeferredOperation>,
}

/// NEW: Deferred validation for DAG resolution phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredValidation {
    pub validation_type: ValidationType,
    pub source_symbol: String,
    pub target_symbol: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredOperation {
    pub target_variable: String,
    pub operation_type: RuntimeOperationType,
    pub source_object_id: Option<String>, // For EXTRACT operations
    pub parameters: Vec<RunParameter>,
}

/// NEW: Types of validation that are deferred to DAG resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidationType {
    VariableReference,     // VAR x references VAR y
    StateReference,        // STATE_REF references global state
    ObjectReference,       // OBJECT_REF references global object
    SetReference,          // SET_REF references set
    FilterStateReference,  // FILTER references global state
    ObjectFieldExtraction, // RUN operation extracts from object
    RunOperationTarget,    // RUN operation targets valid variable
}

impl ValidationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VariableReference => "VariableReference",
            Self::StateReference => "StateReference",
            Self::ObjectReference => "ObjectReference",
            Self::SetReference => "SetReference",
            Self::FilterStateReference => "FilterStateReference",
            Self::ObjectFieldExtraction => "ObjectFieldExtraction",
            Self::RunOperationTarget => "RunOperationTarget",
        }
    }
}

/// NEW: Helper struct for variable categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableCategories {
    /// Variables with no initial value (populated by RUN operations)
    pub computed: HashSet<String>,
    /// Variables with literal initial values
    pub literal: HashSet<String>,
    /// Variables initialized with references to other variables
    pub reference: HashMap<String, String>, // var_name -> referenced_var_name
}

impl VariableCategories {
    /// Get total variable count
    pub fn total_count(&self) -> usize {
        self.computed.len() + self.literal.len() + self.reference.len()
    }

    /// Check if a variable exists in any category
    pub fn contains_variable(&self, name: &str) -> bool {
        self.computed.contains(name)
            || self.literal.contains(name)
            || self.reference.contains_key(name)
    }

    /// Get all variable names across categories
    pub fn get_all_variable_names(&self) -> HashSet<String> {
        let mut all_names = HashSet::new();
        all_names.extend(self.computed.iter().cloned());
        all_names.extend(self.literal.iter().cloned());
        all_names.extend(self.reference.keys().cloned());
        all_names
    }
}

/// DAG node representing a symbol that needs resolution
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResolutionNode {
    pub symbol_id: String,
    pub symbol_type: SymbolType,
    pub dependencies: Vec<String>,
    pub ctn_context: Option<CtnNodeId>, // For local symbols
}

/// Types of symbols in the resolution graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolType {
    Variable,
    GlobalState,
    GlobalObject,
    SetOperation,
    RuntimeOperation,
    LocalState,
    LocalObject,
    FilterDependency,
}

/// Dependency graph for DAG traversal
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub nodes: HashMap<String, ResolutionNode>,
    pub edges: HashMap<String, Vec<String>>, // adjacency list: symbol -> dependencies
    pub reverse_edges: HashMap<String, Vec<String>>, // reverse: symbol -> dependents
}

/// Resolution status tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResolutionStatus {
    Pending,
    InProgress,
    Resolved,
    Failed,
    Skipped, // For circular dependencies
}

/// Enhanced error types for resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionError {
    UndefinedVariable {
        name: String,
        context: String,
    },
    UndefinedGlobalState {
        name: String,
        context: String,
    },
    UndefinedGlobalObject {
        name: String,
        context: String,
    },
    UndefinedSet {
        name: String,
        context: String,
    },
    TypeMismatch {
        expected: DataType,
        found: DataType,
        symbol: String,
    },
    CircularDependency {
        cycle: Vec<String>,
    },
    RuntimeOperationFailed {
        operation: String,
        reason: String,
    },
    FilterValidationFailed {
        filter_context: String,
        reason: String,
    },
    SetOperationFailed {
        set_id: String,
        reason: String,
    },
    LocalSymbolConflict {
        symbol: String,
        ctn_id: CtnNodeId,
    },
    InvalidInput {
        message: String,
    },
    DependencyGraphCorrupted {
        details: String,
    },
    MemoizationError {
        key: String,
        reason: String,
    },
    /// NEW: Computed variable specific errors
    ComputedVariableNotPopulated {
        variable_name: String,
        context: String,
    },
    RunOperationTargetConflict {
        variable_name: String,
        operation1: String,
        operation2: String,
    },
    DeferredValidationFailed {
        validation: DeferredValidation,
        reason: String,
    },
}

/// Warning types for non-fatal issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolutionWarning {
    UnusedVariable {
        name: String,
    },
    UnusedGlobalState {
        name: String,
    },
    UnusedGlobalObject {
        name: String,
    },
    RedundantDependency {
        source: String,
        target: String,
    },
    PerformanceWarning {
        message: String,
    },
    /// NEW: Computed variable warnings
    ComputedVariableNeverPopulated {
        name: String,
    },
    MultipleRunOperationsForVariable {
        name: String,
        count: usize,
    },
}

/// Symbol relationships for dependency graph construction
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolRelationship {
    pub source: String,
    pub target: String,
    pub relationship_type: RelationshipType,
    pub ctn_context: Option<CtnNodeId>, // For local symbol relationships
}

/// Relationship types between symbols
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum RelationshipType {
    VariableInitialization,
    VariableUsage,
    ObjectFieldExtraction,
    StateReference,
    ObjectReference,
    SetReference,
    FilterDependency,
    RunOperationInput,
    RunOperationTarget, // Added - marks computed variable targets
    SetOperandDependency,
    LocalStateDependency,
    LocalObjectDependency,
}

impl RelationshipType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "VariableInitialization" => Some(Self::VariableInitialization),
            "VariableUsage" => Some(Self::VariableUsage),
            "ObjectFieldExtraction" => Some(Self::ObjectFieldExtraction),
            "StateReference" => Some(Self::StateReference),
            "ObjectReference" => Some(Self::ObjectReference),
            "SetReference" => Some(Self::SetReference),
            "FilterDependency" => Some(Self::FilterDependency),
            "RunOperationInput" => Some(Self::RunOperationInput),
            "RunOperationTarget" => Some(Self::RunOperationTarget),
            "SetOperandDependency" => Some(Self::SetOperandDependency),
            "LocalStateDependency" => Some(Self::LocalStateDependency),
            "LocalObjectDependency" => Some(Self::LocalObjectDependency),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VariableInitialization => "VariableInitialization",
            Self::VariableUsage => "VariableUsage",
            Self::ObjectFieldExtraction => "ObjectFieldExtraction",
            Self::StateReference => "StateReference",
            Self::ObjectReference => "ObjectReference",
            Self::SetReference => "SetReference",
            Self::FilterDependency => "FilterDependency",
            Self::RunOperationInput => "RunOperationInput",
            Self::RunOperationTarget => "RunOperationTarget",
            Self::SetOperandDependency => "SetOperandDependency",
            Self::LocalStateDependency => "LocalStateDependency",
            Self::LocalObjectDependency => "LocalObjectDependency",
        }
    }

    pub fn is_hard_dependency(&self) -> bool {
        match self {
            // Hard dependencies that affect DAG ordering
            Self::VariableInitialization
            | Self::RunOperationInput
            | Self::ObjectFieldExtraction
            | Self::SetOperandDependency => true,
            // Not used for DAG - we detect computed variables by checking
            // if they have initial_value.is_none() instead
            Self::RunOperationTarget
            // Soft dependencies - don't affect resolution order
            | Self::VariableUsage
            | Self::StateReference
            | Self::ObjectReference
            | Self::SetReference
            | Self::FilterDependency
            | Self::LocalStateDependency
            | Self::LocalObjectDependency => false,
        }
    }

    /// Check if this relationship type is within CTN scope
    pub fn is_local_scope(&self) -> bool {
        matches!(
            self,
            Self::LocalStateDependency | Self::LocalObjectDependency
        )
    }
}

impl ResolutionContext {
    /// Create new resolution context from parser output
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            global_objects: Vec::new(),
            global_states: Vec::new(),
            set_operations: Vec::new(),
            runtime_operations: Vec::new(),
            criteria_root: CriteriaRoot {
                // Add this
                trees: Vec::new(),
                root_logical_op: LogicalOp::And,
            },
            relationships: Vec::new(),
            resolved_variables: HashMap::new(),
            resolved_global_objects: HashMap::new(),
            resolved_global_states: HashMap::new(),
            resolved_sets: HashMap::new(),
            resolved_criteria: HashMap::new(),
            ctn_local_objects: HashMap::new(),
            ctn_local_states: HashMap::new(),
            resolved_local_objects: HashMap::new(),
            resolved_local_states: HashMap::new(),
            // NEW: Initialize computed variable tracking
            computed_variables: HashSet::new(),
            literal_variables: HashSet::new(),
            reference_variables: HashMap::new(),
            // NEW: Initialize deferred validation
            parsing_errors: Vec::new(),
            deferred_validations: Vec::new(),
            resolution_order: Vec::new(),
            dependency_graph: DependencyGraph::new(),
            resolution_status: HashMap::new(),
            memoization_cache: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            soft_dependencies: HashMap::new(),
            hard_dependencies: HashMap::new(),
            metadata: None,
            resolution_time_operations: Vec::new(),
            scan_time_operations: Vec::new(),
        }
    }

    // =============================================================================
    // NEW: Computed Variable Tracking Methods
    // =============================================================================

    /// Add computed variable to tracking
    pub fn add_computed_variable(&mut self, name: String) {
        self.computed_variables.insert(name);
    }

    /// Add literal variable to tracking
    pub fn add_literal_variable(&mut self, name: String) {
        self.literal_variables.insert(name);
    }

    /// Add reference variable to tracking
    pub fn add_reference_variable(&mut self, name: String, referenced_name: String) {
        self.reference_variables.insert(name, referenced_name);
    }

    /// Check if a variable is computed (no initial value)
    pub fn is_computed_variable(&self, name: &str) -> bool {
        self.computed_variables.contains(name)
    }

    /// Check if a variable has a literal initial value
    pub fn is_literal_variable(&self, name: &str) -> bool {
        self.literal_variables.contains(name)
    }

    /// Check if a variable is initialized with a reference
    pub fn is_reference_variable(&self, name: &str) -> bool {
        self.reference_variables.contains_key(name)
    }

    /// Get all computed variables
    pub fn get_computed_variables(&self) -> Vec<&String> {
        self.computed_variables.iter().collect()
    }

    /// Get initialization dependencies for reference variables
    pub fn get_variable_initialization_dependencies(&self) -> &HashMap<String, String> {
        &self.reference_variables
    }

    /// Categorize all variables by initialization type
    pub fn categorize_variables(&self) -> VariableCategories {
        VariableCategories {
            computed: self.computed_variables.clone(),
            literal: self.literal_variables.clone(),
            reference: self.reference_variables.clone(),
        }
    }

    /// Find RUN operations that target a specific computed variable
    pub fn find_run_operations_for_variable(&self, var_name: &str) -> Vec<&RuntimeOperation> {
        self.runtime_operations
            .iter()
            .filter(|op| op.target_variable == var_name)
            .collect()
    }

    /// Validate that all computed variables have corresponding RUN operations
    pub fn validate_computed_variables_have_run_operations(&mut self) {
        // Clone the computed_variables set to avoid borrow conflicts
        let computed_vars: Vec<String> = self.computed_variables.iter().cloned().collect();

        for computed_var in computed_vars {
            let run_ops = self.find_run_operations_for_variable(&computed_var);
            if run_ops.is_empty() {
                self.add_warning(ResolutionWarning::ComputedVariableNeverPopulated {
                    name: computed_var,
                });
            } else if run_ops.len() > 1 {
                self.add_warning(ResolutionWarning::MultipleRunOperationsForVariable {
                    name: computed_var,
                    count: run_ops.len(),
                });
            }
        }
    }

    // =============================================================================
    // NEW: Deferred Validation Methods
    // =============================================================================

    /// Add deferred validation to be processed during DAG resolution
    pub fn add_deferred_validation(&mut self, validation: DeferredValidation) {
        self.deferred_validations.push(validation);
    }

    /// Add deferred variable reference validation
    pub fn defer_variable_reference_validation(
        &mut self,
        source_symbol: String,
        target_variable: String,
        context: String,
    ) {
        self.add_deferred_validation(DeferredValidation {
            validation_type: ValidationType::VariableReference,
            source_symbol,
            target_symbol: target_variable,
            context,
        });
    }

    /// Add deferred state reference validation
    pub fn defer_state_reference_validation(
        &mut self,
        source_symbol: String,
        target_state: String,
        context: String,
    ) {
        self.add_deferred_validation(DeferredValidation {
            validation_type: ValidationType::StateReference,
            source_symbol,
            target_symbol: target_state,
            context,
        });
    }

    /// Add deferred object reference validation
    pub fn defer_object_reference_validation(
        &mut self,
        source_symbol: String,
        target_object: String,
        context: String,
    ) {
        self.add_deferred_validation(DeferredValidation {
            validation_type: ValidationType::ObjectReference,
            source_symbol,
            target_symbol: target_object,
            context,
        });
    }

    /// Get all deferred validations of a specific type
    pub fn get_deferred_validations_of_type(
        &self,
        validation_type: ValidationType,
    ) -> Vec<&DeferredValidation> {
        self.deferred_validations
            .iter()
            .filter(|v| v.validation_type == validation_type)
            .collect()
    }

    /// Process all deferred validations during DAG resolution
    pub fn process_deferred_validations(&mut self) -> Result<(), Vec<ResolutionError>> {
        let mut validation_errors = Vec::new();

        for validation in self.deferred_validations.clone() {
            match self.validate_deferred(&validation) {
                Ok(_) => {} // Validation passed
                Err(error) => validation_errors.push(error),
            }
        }

        if validation_errors.is_empty() {
            Ok(())
        } else {
            Err(validation_errors)
        }
    }

    /// Validate a single deferred validation
    fn validate_deferred(&self, validation: &DeferredValidation) -> Result<(), ResolutionError> {
        match validation.validation_type {
            ValidationType::VariableReference => {
                if !self.has_variable(&validation.target_symbol) {
                    return Err(ResolutionError::UndefinedVariable {
                        name: validation.target_symbol.clone(),
                        context: validation.context.clone(),
                    });
                }
            }
            ValidationType::StateReference => {
                if !self.has_global_state(&validation.target_symbol) {
                    return Err(ResolutionError::UndefinedGlobalState {
                        name: validation.target_symbol.clone(),
                        context: validation.context.clone(),
                    });
                }
            }
            ValidationType::ObjectReference => {
                if !self.has_global_object(&validation.target_symbol) {
                    return Err(ResolutionError::UndefinedGlobalObject {
                        name: validation.target_symbol.clone(),
                        context: validation.context.clone(),
                    });
                }
            }
            ValidationType::SetReference => {
                if !self.has_set(&validation.target_symbol) {
                    return Err(ResolutionError::UndefinedSet {
                        name: validation.target_symbol.clone(),
                        context: validation.context.clone(),
                    });
                }
            }
            ValidationType::FilterStateReference => {
                if !self.has_global_state(&validation.target_symbol) {
                    return Err(ResolutionError::FilterValidationFailed {
                        filter_context: validation.context.clone(),
                        reason: format!(
                            "Referenced state '{}' does not exist",
                            validation.target_symbol
                        ),
                    });
                }
            }
            ValidationType::ObjectFieldExtraction => {
                if !self.has_global_object(&validation.target_symbol) {
                    return Err(ResolutionError::UndefinedGlobalObject {
                        name: validation.target_symbol.clone(),
                        context: validation.context.clone(),
                    });
                }
            }
            ValidationType::RunOperationTarget => {
                if !self.has_variable(&validation.target_symbol) {
                    return Err(ResolutionError::RuntimeOperationFailed {
                        operation: validation.source_symbol.clone(),
                        reason: format!(
                            "Target variable '{}' does not exist",
                            validation.target_symbol
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    // =============================================================================
    // NEW: Enhanced Memoization Methods
    // =============================================================================

    /// Generate memoization key for variable resolution
    pub fn variable_memoization_key(&self, variable_name: &str, context_info: &str) -> String {
        format!("var:{}:{}", variable_name, context_info)
    }

    /// Generate memoization key for RUN operation result
    pub fn run_operation_memoization_key(&self, target_var: &str, operation_hash: &str) -> String {
        format!("run:{}:{}", target_var, operation_hash)
    }

    /// Check if a computed variable has been resolved and cached
    pub fn is_computed_variable_resolved(&self, variable_name: &str) -> bool {
        let key = format!("computed_var:{}", variable_name);
        self.memoization_cache.contains_key(&key)
    }

    /// Cache a resolved computed variable
    pub fn cache_computed_variable(&mut self, variable_name: String, value: ResolvedValue) {
        let key = format!("computed_var:{}", variable_name);
        self.memoization_cache.insert(key, value);
    }

    /// Get cached computed variable value
    pub fn get_cached_computed_variable(&self, variable_name: &str) -> Option<&ResolvedValue> {
        let key = format!("computed_var:{}", variable_name);
        self.memoization_cache.get(&key)
    }

    /// Generate hash for RUN operation parameters (for memoization)
    pub fn generate_run_operation_hash(&self, operation: &RuntimeOperation) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        operation.operation_type.hash(&mut hasher);
        for param in &operation.parameters {
            // Hash parameter type and content
            std::mem::discriminant(param).hash(&mut hasher);
            // Note: Would need to implement Hash for RunParameter for full hashing
        }
        format!("{:x}", hasher.finish())
    }

    // =============================================================================
    // Enhanced Existing Methods
    // =============================================================================

    /// Check if a variable exists (any category)
    pub fn has_variable(&self, name: &str) -> bool {
        self.variables.iter().any(|v| v.name == name)
    }

    /// Check if a global state exists
    pub fn has_global_state(&self, name: &str) -> bool {
        self.global_states.iter().any(|s| s.identifier == name)
    }

    /// Check if a global object exists
    pub fn has_global_object(&self, name: &str) -> bool {
        self.global_objects.iter().any(|o| o.identifier == name)
    }

    /// Check if a set exists
    pub fn has_set(&self, name: &str) -> bool {
        self.set_operations.iter().any(|s| s.set_id == name)
    }

    /// Add error to context
    pub fn add_error(&mut self, error: ResolutionError) {
        self.errors.push(error);
    }

    /// Add warning to context
    pub fn add_warning(&mut self, warning: ResolutionWarning) {
        self.warnings.push(warning);
    }

    /// Add parsing error (non-fatal)
    pub fn add_parsing_error(&mut self, error: String) {
        self.parsing_errors.push(error);
    }

    /// Check if resolution was successful
    pub fn is_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get resolution statistics
    pub fn get_stats(&self) -> ResolutionStats {
        ResolutionStats {
            total_symbols: self.count_total_symbols(),
            resolved_symbols: self.count_resolved_symbols(),
            errors: self.errors.len(),
            warnings: self.warnings.len(),
            cache_hits: self.memoization_cache.len(),
            computed_variables: self.computed_variables.len(),
            literal_variables: self.literal_variables.len(),
            reference_variables: self.reference_variables.len(),
            deferred_validations: self.deferred_validations.len(),
        }
    }

    /// Count total symbols to be resolved
    pub fn count_total_symbols(&self) -> usize {
        self.variables.len()
            + self.global_objects.len()
            + self.global_states.len()
            + self.set_operations.len()
    }

    /// Count successfully resolved symbols
    pub fn count_resolved_symbols(&self) -> usize {
        self.resolved_variables.len()
            + self.resolved_global_objects.len()
            + self.resolved_global_states.len()
            + self.resolved_sets.len()
            + self.resolved_criteria.len()
    }

    /// Check if a symbol exists in global scope
    pub fn has_global_symbol(&self, name: &str) -> bool {
        self.has_variable(name)
            || self.has_global_object(name)
            || self.has_global_state(name)
            || self.has_set(name)
    }

    /// Check if a symbol is resolved
    pub fn is_resolved(&self, name: &str) -> bool {
        matches!(
            self.resolution_status.get(name),
            Some(ResolutionStatus::Resolved)
        )
    }

    /// Get memoized value if available
    pub fn get_memoized(&self, key: &str) -> Option<&ResolvedValue> {
        self.memoization_cache.get(key)
    }

    /// Store memoized value
    pub fn memoize(&mut self, key: String, value: ResolvedValue) {
        self.memoization_cache.insert(key, value);
    }

    /// Mark symbol as in progress (for cycle detection)
    pub fn mark_in_progress(&mut self, symbol: &str) {
        self.resolution_status
            .insert(symbol.to_string(), ResolutionStatus::InProgress);
    }

    /// Mark symbol as resolved
    pub fn mark_resolved(&mut self, symbol: &str) {
        self.resolution_status
            .insert(symbol.to_string(), ResolutionStatus::Resolved);
    }

    /// Mark symbol as failed
    pub fn mark_failed(&mut self, symbol: &str) {
        self.resolution_status
            .insert(symbol.to_string(), ResolutionStatus::Failed);
    }

    /// Check if symbol is currently being resolved (cycle detection)
    pub fn is_in_progress(&self, symbol: &str) -> bool {
        matches!(
            self.resolution_status.get(symbol),
            Some(ResolutionStatus::InProgress)
        )
    }
}

impl DependencyGraph {
    /// Create new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    /// Add node to graph
    pub fn add_node(&mut self, node: ResolutionNode) {
        let node_id = node.symbol_id.clone();
        self.nodes.insert(node_id.clone(), node);
        self.edges.entry(node_id.clone()).or_insert_with(Vec::new);
        self.reverse_edges.entry(node_id).or_insert_with(Vec::new);
    }

    /// Add dependency edge
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.edges
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
        self.reverse_edges
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());
    }

    /// Get dependencies for a symbol
    pub fn get_dependencies(&self, symbol: &str) -> Vec<String> {
        self.edges.get(symbol).cloned().unwrap_or_default()
    }

    /// Get dependents for a symbol
    pub fn get_dependents(&self, symbol: &str) -> Vec<String> {
        self.reverse_edges.get(symbol).cloned().unwrap_or_default()
    }

    /// Perform topological sort for resolution order
    pub fn topological_sort(&self) -> Result<Vec<String>, Vec<String>> {
        let mut visited = HashSet::new();
        let mut temp_mark = HashSet::new();
        let mut result = Vec::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                if let Err(cycle) =
                    self.visit_node(node_id, &mut visited, &mut temp_mark, &mut result)
                {
                    return Err(cycle);
                }
            }
        }

        result.reverse(); // Topological order
        Ok(result)
    }

    /// Visit node for topological sort (with cycle detection)
    fn visit_node(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        temp_mark: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), Vec<String>> {
        if temp_mark.contains(node) {
            // Cycle detected - reconstruct cycle path
            return Err(vec![node.to_string()]);
        }

        if visited.contains(node) {
            return Ok(());
        }

        temp_mark.insert(node.to_string());

        for dependency in self.get_dependencies(node) {
            if let Err(mut cycle) = self.visit_node(&dependency, visited, temp_mark, result) {
                cycle.push(node.to_string());
                return Err(cycle);
            }
        }

        temp_mark.remove(node);
        visited.insert(node.to_string());
        result.push(node.to_string());

        Ok(())
    }
}

/// Enhanced resolution statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionStats {
    pub total_symbols: usize,
    pub resolved_symbols: usize,
    pub errors: usize,
    pub warnings: usize,
    pub cache_hits: usize,
    // NEW: Computed variable statistics
    pub computed_variables: usize,
    pub literal_variables: usize,
    pub reference_variables: usize,
    pub deferred_validations: usize,
}

impl ResolutionStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_symbols == 0 {
            1.0
        } else {
            self.resolved_symbols as f64 / self.total_symbols as f64
        }
    }

    /// Check if resolution is complete
    pub fn is_complete(&self) -> bool {
        self.resolved_symbols == self.total_symbols && self.errors == 0
    }

    /// Get variable distribution summary
    pub fn variable_distribution_summary(&self) -> String {
        format!(
            "Variables: {} literal, {} reference, {} computed",
            self.literal_variables, self.reference_variables, self.computed_variables
        )
    }
}

// Display implementations
impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionError::UndefinedVariable { name, context } => {
                write!(f, "Undefined variable '{}' in context: {}", name, context)
            }
            ResolutionError::UndefinedGlobalState { name, context } => {
                write!(
                    f,
                    "Undefined global state '{}' in context: {}",
                    name, context
                )
            }
            ResolutionError::UndefinedGlobalObject { name, context } => {
                write!(
                    f,
                    "Undefined global object '{}' in context: {}",
                    name, context
                )
            }
            ResolutionError::UndefinedSet { name, context } => {
                write!(f, "Undefined set '{}' in context: {}", name, context)
            }
            ResolutionError::TypeMismatch {
                expected,
                found,
                symbol,
            } => {
                write!(
                    f,
                    "Type mismatch in '{}': expected {:?}, found {:?}",
                    symbol, expected, found
                )
            }
            ResolutionError::CircularDependency { cycle } => {
                write!(f, "Circular dependency detected: {}", cycle.join(" -> "))
            }
            ResolutionError::RuntimeOperationFailed { operation, reason } => {
                write!(f, "Runtime operation '{}' failed: {}", operation, reason)
            }
            ResolutionError::FilterValidationFailed {
                filter_context,
                reason,
            } => {
                write!(
                    f,
                    "Filter validation failed in '{}': {}",
                    filter_context, reason
                )
            }
            ResolutionError::SetOperationFailed { set_id, reason } => {
                write!(f, "Set operation '{}' failed: {}", set_id, reason)
            }
            ResolutionError::LocalSymbolConflict { symbol, ctn_id } => {
                write!(f, "Local symbol conflict: '{}' in CTN {}", symbol, ctn_id)
            }
            ResolutionError::InvalidInput { message } => {
                write!(f, "Invalid input: {}", message)
            }
            ResolutionError::DependencyGraphCorrupted { details } => {
                write!(f, "Dependency graph corrupted: {}", details)
            }
            ResolutionError::MemoizationError { key, reason } => {
                write!(f, "Memoization error for '{}': {}", key, reason)
            }
            ResolutionError::ComputedVariableNotPopulated {
                variable_name,
                context,
            } => {
                write!(
                    f,
                    "Computed variable '{}' not populated by any RUN operation in context: {}",
                    variable_name, context
                )
            }
            ResolutionError::RunOperationTargetConflict {
                variable_name,
                operation1,
                operation2,
            } => {
                write!(
                    f,
                    "Multiple RUN operations target variable '{}': {} and {}",
                    variable_name, operation1, operation2
                )
            }
            ResolutionError::DeferredValidationFailed { validation, reason } => {
                write!(
                    f,
                    "Deferred validation failed: {} -> {} ({}): {}",
                    validation.source_symbol,
                    validation.target_symbol,
                    validation.validation_type.as_str(),
                    reason
                )
            }
        }
    }
}

impl Default for ResolutionContext {
    fn default() -> Self {
        Self::new()
    }
}
