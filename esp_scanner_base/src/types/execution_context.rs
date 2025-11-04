// src/types/execution_context.rs
//! Execution context with resolved symbols ready for compliance validation
use crate::types::common::{DataType, LogicalOp, Operation, RecordData, ResolvedValue};
use crate::types::criteria::CriteriaTree;
use crate::types::criterion::{CriterionDeclaration, CtnNodeId};
use crate::types::filter::ResolvedFilterSpec;
use crate::types::metadata::MetaDataBlock;
use crate::types::object::{ResolvedObject, ResolvedObjectElement};
use crate::types::resolution_context::{DeferredOperation, ResolutionContext};
use crate::types::variable::ResolvedVariable;
use crate::types::FieldPath;
use crate::types::ResolvedSetOperation;
use crate::types::TestSpecification;
use crate::types::{EntityCheck, ResolvedState};
use esp_compiler::grammar::ModuleField;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// ============================================================================
// NEW: Tree Structure for Execution
// ============================================================================
/// Executable criteria tree preserving logical structure from parser/resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableCriteriaTree {
    /// Leaf node: single CTN to execute
    Criterion(ExecutableCriterion),
    /// Branch node: logical block of multiple criteria
    Block {
        logical_op: LogicalOp,
        negate: bool,
        children: Vec<ExecutableCriteriaTree>,
    },
}
impl ExecutableCriteriaTree {
    /// Count total criteria in tree (recursive)
    pub fn count_criteria(&self) -> usize {
        match self {
            Self::Criterion(_) => 1,
            Self::Block { children, .. } => {
                children.iter().map(|child| child.count_criteria()).sum()
            }
        }
    }
    /// Get maximum depth of tree
    pub fn max_depth(&self) -> usize {
        match self {
            Self::Criterion(_) => 1,
            Self::Block { children, .. } => {
                1 + children
                    .iter()
                    .map(|child| child.max_depth())
                    .max()
                    .unwrap_or(0)
            }
        }
    }

    /// Get all criteria as flat list (for iteration)
    pub fn get_all_criteria(&self) -> Vec<&ExecutableCriterion> {
        match self {
            Self::Criterion(criterion) => vec![criterion],
            Self::Block { children, .. } => children
                .iter()
                .flat_map(|child| child.get_all_criteria())
                .collect(),
        }
    }

    /// Get mutable references to all criteria
    pub fn get_all_criteria_mut(&mut self) -> Vec<&mut ExecutableCriterion> {
        match self {
            Self::Criterion(criterion) => vec![criterion],
            Self::Block { children, .. } => children
                .iter_mut()
                .flat_map(|child| child.get_all_criteria_mut())
                .collect(),
        }
    }

    /// Convert from CriteriaTree (from resolution phase)
    pub fn from_criteria_tree(
        tree: &CriteriaTree,
        context: &ResolutionContext,
    ) -> Result<Self, String> {
        match tree {
            CriteriaTree::Criterion {
                declaration,
                node_id,
            } => {
                let executable =
                    ExecutableCriterion::from_declaration(declaration, *node_id, context)?;
                Ok(Self::Criterion(executable))
            }
            CriteriaTree::Block {
                logical_op,
                negate,
                children,
            } => {
                let mut executable_children = Vec::new();
                for child in children {
                    let executable_child = Self::from_criteria_tree(child, context)?;
                    executable_children.push(executable_child);
                }
                Ok(Self::Block {
                    logical_op: *logical_op,
                    negate: *negate,
                    children: executable_children,
                })
            }
        }
    }

    /// Validate tree structure
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Criterion(criterion) => {
                if criterion.ctn_node_id == 0 {
                    return Err("Criterion missing CTN node ID".to_string());
                }
                if criterion.criterion_type.is_empty() {
                    return Err("Criterion missing type".to_string());
                }
                Ok(())
            }
            Self::Block { children, .. } => {
                if children.is_empty() {
                    return Err("Block has no children".to_string());
                }
                for child in children {
                    child.validate()?;
                }
                Ok(())
            }
        }
    }
}
// ============================================================================
// Execution Context (Modified to use tree)
// ============================================================================
/// Execution context with resolved symbols and executable criteria tree
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Metadata from ESP definition
    pub metadata: Option<MetaDataBlock>,
    /// CHANGED: Executable criteria tree (not flat list)
    pub criteria_tree: ExecutableCriteriaTree,

    /// Global variables resolved during resolution phase
    pub global_variables: HashMap<String, ResolvedVariable>,

    /// Global states
    pub global_states: HashMap<String, ResolvedState>,

    /// Global objects
    pub global_objects: HashMap<String, ResolvedObject>,

    /// Resolved SET operations
    pub global_sets: HashMap<String, ResolvedSetOperation>,

    /// Deferred operations to execute at scan-time
    pub deferred_operations: Vec<DeferredOperation>,

    /// Local states by CTN node ID
    pub local_states: HashMap<CtnNodeId, Vec<ExecutableState>>,

    /// Local objects by CTN node ID (max 1 per CTN)
    pub local_objects: HashMap<CtnNodeId, ExecutableObject>,
}
impl ExecutionContext {
    /// Create execution context from resolution context
    pub fn from_resolution_context(resolution_context: &ResolutionContext) -> Result<Self, String> {
        // Convert CriteriaRoot to ExecutableCriteriaTree
        let criteria_tree = Self::build_executable_tree(resolution_context)?;
        // Convert resolved variables
        let global_variables = resolution_context.resolved_variables.clone();

        // Convert global states
        let global_states = resolution_context.resolved_global_states.clone();

        // Convert global objects
        let global_objects = resolution_context.resolved_global_objects.clone();

        // Convert sets
        let global_sets = resolution_context.resolved_sets.clone();

        // Extract deferred operations
        let deferred_operations = resolution_context.scan_time_operations.clone();

        // Convert local states
        let mut local_states = HashMap::new();
        for (ctn_id, states) in &resolution_context.resolved_local_states {
            let executable_states: Vec<ExecutableState> = states
                .iter()
                .map(|state| ExecutableState::from_resolved_state(state))
                .collect();
            local_states.insert(*ctn_id, executable_states);
        }

        // Convert local objects
        let mut local_objects = HashMap::new();
        for (ctn_id, obj) in &resolution_context.resolved_local_objects {
            let executable_obj = ExecutableObject::from_resolved_object(obj);
            local_objects.insert(*ctn_id, executable_obj);
        }

        Ok(Self {
            metadata: Some(resolution_context.metadata.clone()),
            criteria_tree,
            global_variables,
            global_states,
            global_objects,
            global_sets,
            deferred_operations,
            local_states,
            local_objects,
        })
    }

    /// Build executable tree from CriteriaRoot
    fn build_executable_tree(
        context: &ResolutionContext,
    ) -> Result<ExecutableCriteriaTree, String> {
        let root = &context.criteria_root;

        // Handle single tree case
        if root.trees.len() == 1 {
            return ExecutableCriteriaTree::from_criteria_tree(&root.trees[0], context);
        }

        // Multiple trees: wrap in root-level logical op
        let mut executable_children = Vec::new();
        for tree in &root.trees {
            let executable_child = ExecutableCriteriaTree::from_criteria_tree(tree, context)?;
            executable_children.push(executable_child);
        }

        Ok(ExecutableCriteriaTree::Block {
            logical_op: root.root_logical_op,
            negate: false,
            children: executable_children,
        })
    }

    /// Get all criteria as flat list (compatibility method)
    pub fn get_all_criteria(&self) -> Vec<&ExecutableCriterion> {
        self.criteria_tree.get_all_criteria()
    }

    /// Count total criteria
    pub fn count_criteria(&self) -> usize {
        self.criteria_tree.count_criteria()
    }

    /// Validate execution context
    pub fn validate(&self) -> Result<(), String> {
        // Validate tree structure
        self.criteria_tree.validate()?;

        // Validate criteria count
        let count = self.count_criteria();
        if count == 0 {
            return Err("Execution context has no criteria".to_string());
        }

        // Validate all CTN node IDs are unique
        let all_criteria = self.get_all_criteria();
        let mut seen_ids = std::collections::HashSet::new();
        for criterion in all_criteria {
            if !seen_ids.insert(criterion.ctn_node_id) {
                return Err(format!("Duplicate CTN node ID: {}", criterion.ctn_node_id));
            }
        }

        Ok(())
    }
}
// ============================================================================
// Executable Criterion (Leaf node)
// ============================================================================
/// Single executable criterion (CTN) with resolved symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableCriterion {
    pub ctn_node_id: CtnNodeId,
    pub criterion_type: String,
    pub test: TestSpecification,
    pub objects: Vec<ExecutableObject>,
    pub states: Vec<ExecutableState>,
}
impl ExecutableCriterion {
    /// Convert from CriterionDeclaration
    pub fn from_declaration(
        declaration: &CriterionDeclaration,
        ctn_node_id: CtnNodeId,
        context: &ResolutionContext,
    ) -> Result<Self, String> {
        // Convert objects
        let mut objects = Vec::new();
        for obj_ref in &declaration.object_refs {
            let resolved_obj = context
                .resolved_global_objects
                .get(&obj_ref.object_id)
                .ok_or_else(|| {
                    format!(
                        "Object '{}' not found in resolution context",
                        obj_ref.object_id
                    )
                })?;
            objects.push(ExecutableObject::from_resolved_object(resolved_obj));
        }
        // Add local object if present
        if declaration.local_object.is_some() {
            if let Some(resolved_local) = context.resolved_local_objects.get(&ctn_node_id) {
                objects.push(ExecutableObject::from_resolved_object(resolved_local));
            }
        }

        // Convert states
        let mut states = Vec::new();
        for state_ref in &declaration.state_refs {
            let resolved_state = context
                .resolved_global_states
                .get(&state_ref.state_id)
                .ok_or_else(|| {
                    format!(
                        "State '{}' not found in resolution context",
                        state_ref.state_id
                    )
                })?;
            states.push(ExecutableState::from_resolved_state(resolved_state));
        }

        // Add local states if present
        if let Some(local_states) = context.resolved_local_states.get(&ctn_node_id) {
            for local_state in local_states {
                states.push(ExecutableState::from_resolved_state(local_state));
            }
        }

        Ok(Self {
            ctn_node_id,
            criterion_type: declaration.criterion_type.clone(),
            test: declaration.test.clone(),
            objects,
            states,
        })
    }
}
// ============================================================================
// ExecutableObject
// ============================================================================
/// Executable object with resolved field values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableObject {
    pub identifier: String,
    pub elements: Vec<ExecutableObjectElement>,
    pub is_global: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableObjectElement {
    Module {
        field: ModuleField, // Compiler's enum
        value: String,      // Actual value
    },
    Parameter {
        data_type: DataType,
        data: RecordData,
    },
    Select {
        data_type: DataType,
        data: RecordData,
    },
    Behavior {
        values: Vec<String>,
    },
    Filter {
        filter: ResolvedFilterSpec,
    },
    SetRef {
        set_id: String,
    },
    Field {
        name: String,
        value: ResolvedValue,
    },
}
impl ExecutableObject {
    pub fn from_resolved_object(resolved: &ResolvedObject) -> Self {
        let elements = resolved
            .resolved_elements
            .iter()
            .map(|elem| match elem {
                ResolvedObjectElement::Module { field, value } => {
                    // Parse string field name to ModuleField enum
                    let module_field = match field.as_str() {
                        "module_name" | "name" => ModuleField::ModuleName,
                        "module_version" | "version" => ModuleField::ModuleVersion,
                        "module_type" | "type" => ModuleField::ModuleType,
                        "module_path" | "path" => ModuleField::ModulePath,
                        _ => ModuleField::ModuleName, // Default fallback
                    };
                    ExecutableObjectElement::Module {
                        field: module_field,
                        value: value.clone(),
                    }
                }
                ResolvedObjectElement::Parameter { data_type, data } => {
                    ExecutableObjectElement::Parameter {
                        data_type: *data_type,
                        data: data.clone(),
                    }
                }
                ResolvedObjectElement::Select { data_type, data } => {
                    ExecutableObjectElement::Select {
                        data_type: *data_type,
                        data: data.clone(),
                    }
                }
                ResolvedObjectElement::Behavior { values } => ExecutableObjectElement::Behavior {
                    values: values.clone(),
                },
                ResolvedObjectElement::Filter(filter) => ExecutableObjectElement::Filter {
                    filter: filter.clone(),
                },
                ResolvedObjectElement::SetRef { set_id } => ExecutableObjectElement::SetRef {
                    set_id: set_id.clone(),
                },
                ResolvedObjectElement::Field { name, value } => ExecutableObjectElement::Field {
                    name: name.clone(),
                    value: value.clone(),
                },
            })
            .collect();
        Self {
            identifier: resolved.identifier.clone(),
            elements,
            is_global: resolved.is_global,
        }
    }

    pub fn has_field(&self, field_name: &str) -> bool {
        self.elements.iter().any(|elem| match elem {
            ExecutableObjectElement::Field { name, .. } => name == field_name,
            _ => false,
        })
    }

    pub fn get_field(&self, field_name: &str) -> Option<&ResolvedValue> {
        self.elements.iter().find_map(|elem| match elem {
            ExecutableObjectElement::Field { name, value } if name == field_name => Some(value),
            _ => None,
        })
    }

    pub fn get_all_fields(&self) -> Vec<crate::strategies::ctn_contract::FieldInfo> {
        self.elements
            .iter()
            .filter_map(|elem| match elem {
                ExecutableObjectElement::Field { name, value } => {
                    Some(crate::strategies::ctn_contract::FieldInfo {
                        name: name.clone(),
                        data_type: value.infer_type(),
                    })
                }
                _ => None,
            })
            .collect()
    }

    pub fn get_filters(&self) -> Vec<&ResolvedFilterSpec> {
        self.elements
            .iter()
            .filter_map(|elem| match elem {
                ExecutableObjectElement::Filter { filter } => Some(filter),
                _ => None,
            })
            .collect()
    }
}
// ============================================================================
// ExecutableState
// ============================================================================
/// Executable state with resolved field values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableState {
    pub identifier: String,
    pub fields: Vec<ExecutableStateField>,
    pub record_checks: Vec<ExecutableRecordCheck>,
    pub is_global: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableStateField {
    pub name: String,
    pub data_type: DataType,
    pub operation: crate::types::common::Operation,
    pub value: ResolvedValue,
    pub entity_check: Option<EntityCheck>,
}

// NEW: Add these three structures for record checks
/// Executable record check with resolved values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableRecordCheck {
    pub data_type: Option<DataType>,
    pub content: ExecutableRecordContent,
}

/// Record content for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableRecordContent {
    /// Direct operation on entire record
    Direct {
        operation: Operation,
        value: ResolvedValue,
    },
    /// Nested field validation
    Nested { fields: Vec<ExecutableRecordField> },
}

/// Individual field within a record check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableRecordField {
    pub path: FieldPath,
    pub data_type: DataType,
    pub operation: Operation,
    pub value: ResolvedValue,
    pub entity_check: Option<EntityCheck>,
}

impl ExecutableState {
    pub fn from_resolved_state(resolved: &ResolvedState) -> Self {
        // Convert regular fields
        let fields = resolved
            .resolved_fields
            .iter()
            .map(|field| ExecutableStateField {
                name: field.name.clone(),
                data_type: field.data_type,
                operation: field.operation,
                value: field.value.clone(),
                entity_check: field.entity_check,
            })
            .collect();

        // NEW: Convert record checks
        let record_checks = resolved
            .resolved_record_checks
            .iter()
            .map(|check| {
                let content = match &check.content {
                    crate::types::state::ResolvedRecordContent::Direct { operation, value } => {
                        ExecutableRecordContent::Direct {
                            operation: *operation,
                            value: value.clone(),
                        }
                    }
                    crate::types::state::ResolvedRecordContent::Nested { fields } => {
                        let executable_fields = fields
                            .iter()
                            .map(|f| ExecutableRecordField {
                                path: f.path.clone(),
                                data_type: f.data_type,
                                operation: f.operation,
                                value: f.value.clone(),
                                entity_check: f.entity_check,
                            })
                            .collect();

                        ExecutableRecordContent::Nested {
                            fields: executable_fields,
                        }
                    }
                };

                ExecutableRecordCheck {
                    data_type: check.data_type,
                    content,
                }
            })
            .collect();

        Self {
            identifier: resolved.identifier.clone(),
            fields,
            record_checks, // NEW: Add this
            is_global: resolved.is_global,
        }
    }
}

// ============================================================================
// Helper implementations for ResolvedValue
// ============================================================================
impl ResolvedValue {
    pub fn infer_type(&self) -> DataType {
        match self {
            Self::String(_) => DataType::String,
            Self::Integer(_) => DataType::Int,
            Self::Float(_) => DataType::Float,
            Self::Boolean(_) => DataType::Boolean,
            Self::Binary(_) => DataType::Binary,
            Self::Version(_) => DataType::Version,
            Self::EvrString(_) => DataType::EvrString,
            Self::RecordData(_) => DataType::RecordData,
            Self::Collection(_) => DataType::String, // Default for collections
        }
    }
}
