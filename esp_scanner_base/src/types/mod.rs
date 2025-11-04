// ============================================================================
// SCANNER TYPE MODULES - Complete and Corrected
// ============================================================================

// Core type definitions
pub mod common;
pub mod error;
pub mod metadata;

// Declaration types
pub mod filter;
pub mod object;
pub mod runtime_operation;
pub mod set;
pub mod state;
pub mod variable;

// Criteria types
pub mod criteria;
pub mod criterion;

// Context types
pub mod execution_context;
pub mod resolution_context;

// Record traits (may be redundant with state.rs extensions)
pub mod record_traits;

// Add this line:
pub mod field_path_extensions;
pub use field_path_extensions::*;

// ============================================================================
// RE-EXPORTS FROM COMPILER - Types scanner uses directly
// ============================================================================

// Test specification types (used in CTN execution)
pub use esp_compiler::grammar::ast::nodes::{
    ExistenceCheck, ItemCheck, StateJoinOp, TestSpecification,
};

// Entity check types (used in state field validation)
pub use esp_compiler::grammar::ast::nodes::EntityCheck;

// Record types (used in state definitions)
pub use esp_compiler::grammar::ast::nodes::{RecordCheck, RecordContent, RecordField};

// Filter and object types
pub use esp_compiler::grammar::ast::nodes::FilterAction;
pub use esp_compiler::grammar::ast::nodes::FilterSpec;
pub use esp_compiler::grammar::ast::nodes::ObjectElement;
pub use esp_compiler::grammar::ast::nodes::ObjectField;
pub use esp_compiler::grammar::ast::nodes::ObjectRef;

// Module field type (for module specifications)
pub use esp_compiler::grammar::ast::nodes::FieldPath;
pub use esp_compiler::grammar::ast::nodes::ModuleField;

// ============================================================================
// RE-EXPORTS - Import what you need from types::*
// ============================================================================

// Core types and traits
pub use common::*; // DataType, ResolvedValue, RecordData, DataTypeExt, ValueExt, etc.
pub use error::*; // FieldResolutionError
pub use metadata::*; // MetaDataBlock

// Variable types
pub use variable::*; // VariableDeclaration, ResolvedVariable

// State types
pub use state::*; // StateDeclaration, ResolvedState, RecordCheck traits

// Object types
pub use object::*; // ObjectDeclaration, ResolvedObject, ObjectElementExt

// Filter types
pub use filter::*; // FilterSpecExt, ResolvedFilterSpec, FilterResult

// Runtime operation types
pub use runtime_operation::*; // RuntimeOperation, RunParameterExt

// Set types
pub use set::*; // SetOperation, ResolvedSetOperation, SetOperandExt

// Criteria types - be specific to avoid ambiguous glob re-exports
pub use criteria::{CriteriaRoot, CriteriaTree}; // NOT ExecutableCriteriaTree - that's in execution_context
pub use criterion::*; // CriterionDeclaration, ResolvedCriterion

// Context types
pub use execution_context::*; // ExecutionContext, ExecutableCriteriaTree (the actual one we want)
pub use resolution_context::*; // ResolutionContext

// Record traits
pub use record_traits::*;

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Node ID for criteria tree traversal
pub type CtnNodeId = usize;
