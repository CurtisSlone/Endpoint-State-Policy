//! # Public API for ICS SDK
//!
//! This module provides the high-level interfaces for scanner implementation.pub mod config;
pub mod config;
pub mod errors;
pub mod processor;
pub mod scanner; // Re-export for convenience

pub use crate::execution::{
    evaluate_entity_check, evaluate_existence_check, evaluate_item_check, evaluate_state_operator,
    ExecutionEngine, ExecutionError, extract_behavior_hints, BehaviorHints
}; // Re-export strategy components
pub use crate::results::{
    ComplianceFinding, ComplianceResults, ComplianceStatus as ResultComplianceStatus,
    FindingSeverity, HostContext, IcsMetadata, ResultGenerationError, ResultGenerator,
    ScanMetadata, ScanResult, UserContext,
};
pub use crate::strategies::{
    // Data structures
    CollectedData,
    CollectionError,
    CollectionMetadata,
    CollectionMode,
    CollectionStrategy,
    ComplianceStatus, // USE FROM STRATEGIES
    // Contracts
    CtnContract,
    // Errors
    CtnContractError,
    // Core traits
    CtnDataCollector,
    CtnExecutionError,
    CtnExecutionResult,
    CtnExecutor,
    CtnFieldMappings,
    CtnMetadata,
    // Registry
    CtnStrategyRegistry,
    DefaultTestProcessor,
    // Test results
    ExistenceResult,
    FieldValidationResult,
    ItemCheckResult,
    ObjectFieldSpec,
    ObjectRequirements,
    PerformanceHints,
    RegistryBuilder,
    RegistryHealth,
    RegistryStatistics,
    StateFieldSpec,
    StateRequirements,
    StateValidationResult,
    StrategyError,
    TestPhase,
    TestProcessor,
    ValidationReport,
}; // Re-export result types
pub use config::ProcessorConfig;
pub use errors::ProcessorError;
pub use processor::{IcsProcessor, ProcessResult};
pub use scanner::{BatchScanResult, BatchStatistics, IcsScanner, ScannerError}; // Re-export execution components // REMOVED: Optional features not yet implemented
                                                                               // pub use crate::execution::behavior::{extract_behavior_hints, BehaviorHints};
                                                                               // pub use crate::execution::entity_check::{
                                                                               //     get_collection_strategy, wrap_for_entity_check, EntityCheckAnalyzer,
                                                                               // };
                                                                               // pub use crate::execution::module_version::{
                                                                               //     extract_module_spec, ModuleSpec, SemanticVersion, VersionCompatibility,
                                                                               // };pub use crate::execution::structured_params::{parse_parameters, validate_parameter_depth};// Re-export commonly used types
pub use crate::types::{
    common::{DataType, LogicalOp, Operation, ResolvedValue},
    execution_context::{
        ExecutableCriteriaTree, ExecutableCriterion, ExecutableObject, ExecutableState,
        ExecutionContext,
    },
    state::EntityCheck, // Type definition still exists, just not the analyzer
    test::{ExistenceCheck, ItemCheck, StateJoinOp, TestSpecification},
};

pub use crate::strategies::{CommandError, CommandOutput, SystemCommandExecutor};
