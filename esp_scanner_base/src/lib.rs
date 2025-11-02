//! # ICS SDK - Infrastructure Compliance Scanner

pub mod api;
pub mod execution;
pub mod ffi;
pub mod parser;
pub mod resolution;
pub mod results;
pub mod scanner;
pub mod strategies;
pub mod types;

#[cfg(feature = "cli")]
pub mod cli;

// Convenience re-exports
pub use api::*;
pub use scanner::{
    create_scanner_registry, FileContentExecutor, FileMetadataExecutor, FileSystemCollector,
};

pub mod prelude {
    pub use crate::api::{
        BatchScanResult, CommandError, CommandOutput, IcsProcessor, IcsScanner, ProcessResult,
        ProcessorConfig, ProcessorError, ScannerError, SystemCommandExecutor,
    };

    pub use crate::strategies::{
        CollectedData, ComplianceStatus, CtnContract, CtnDataCollector, CtnExecutionResult,
        CtnExecutor, CtnStrategyRegistry, RegistryBuilder,
    };

    pub use crate::execution::{ExecutionEngine, ExecutionError};
    pub use crate::results::{ComplianceFinding, FindingSeverity, ResultGenerator, ScanResult};

    pub use crate::types::{
        common::{DataType, Operation, ResolvedValue},
        execution_context::{
            ExecutableCriterion, ExecutableObject, ExecutableState, ExecutionContext,
        },
        test::{ExistenceCheck, ItemCheck, StateJoinOp},
    };

    pub use crate::scanner::{
        create_scanner_registry, FileContentExecutor, FileMetadataExecutor, FileSystemCollector,
    };
}
