//! # Processor Errors

use crate::ffi::common::IcsError;
use crate::resolution::error::ResolutionError;

/// Comprehensive error type for ICS processing
#[derive(Debug, thiserror::Error)]
pub enum ProcessorError {
    /// Error from FFI layer
    #[error("FFI error: {0}")]
    FfiError(#[from] IcsError),
    /// Error from resolution engine
    #[error("Resolution error: {0}")]
    ResolutionError(#[from] ResolutionError),

    /// REMOVED: ExecutionContextError - use String instead
    /// Error from execution context creation
    #[error("Execution context error: {reason}")]
    ExecutionContextError { reason: String },

    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    /// Processing failed with reason
    #[error("Processing failed: {reason}")]
    ProcessingFailed { reason: String },

    /// Configuration error
    #[error("Configuration error: {reason}")]
    ConfigurationError { reason: String },

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl ProcessorError {
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ProcessorError::FileNotFound { .. }
                | ProcessorError::ProcessingFailed { .. }
                | ProcessorError::ConfigurationError { .. }
        )
    }
    /// Check if this error indicates a system/library issue
    pub fn is_system_error(&self) -> bool {
        matches!(
            self,
            ProcessorError::FfiError(_) | ProcessorError::IoError(_)
        )
    }

    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            ProcessorError::FfiError(e) => format!("Parser library error: {}", e),
            ProcessorError::ResolutionError(e) => format!("Resolution failed: {}", e),
            ProcessorError::ExecutionContextError { reason } => {
                format!("Failed to create execution context: {}", reason)
            }
            ProcessorError::FileNotFound { path } => {
                format!("File not found: {}", path)
            }
            ProcessorError::ProcessingFailed { reason } => {
                format!("Processing failed: {}", reason)
            }
            ProcessorError::ConfigurationError { reason } => {
                format!("Configuration error: {}", reason)
            }
            ProcessorError::IoError(e) => format!("I/O error: {}", e),
        }
    }
}
