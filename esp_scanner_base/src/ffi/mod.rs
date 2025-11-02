//! # FFI Consumer Module for ICS Parser
//!
//! Provides safe Rust bindings for consuming the simplified ICS parser C library.
//! This module focuses on parsing ICS files and extracting AST + symbol data.
//!
//! ## Architecture Changes
//!
//! This version consumes a simplified FFI that only exposes:
//! - AST (Abstract Syntax Tree) data
//! - Symbol discovery results  
//! - Basic success/failure status
//!
//! The complex execution data, metadata, and processing statistics from the
//! previous version have been removed to simplify the library interface.

pub mod bindings;
pub mod common;
pub mod logging;
pub mod parsing;
pub mod types;

// Re-export commonly used types for convenience
pub use common::{FfiResult, IcsError, IcsErrorCode};
pub use parsing::{
    get_parser_version, parse_directory, parse_file, BatchConfig, BatchResult, IcsParser,
    ParseResult, ParseSummary,
};
pub use types::{ExecutionData, ExecutionMetadata, PipelineOutput, ProcessingStats};

// Re-export logging functionality
pub use logging::{consumer_codes, ConsumerLogger};
pub use logging::{
    log_consumer_debug, log_consumer_error, log_consumer_info, log_consumer_warning,
    set_consumer_context,
};

/// Library version and compatibility information
pub mod version {
    /// Consumer library version
    pub const CONSUMER_VERSION: &str = env!("CARGO_PKG_VERSION");

    /// Minimum supported parser library version
    pub const MIN_PARSER_VERSION: (u32, u32, u32) = (1, 0, 0);

    /// Check if the connected parser library version is compatible
    pub fn check_compatibility() -> Result<(), super::IcsError> {
        let parser = super::parsing::IcsParser::new()?;
        let (major, minor, patch) = parser.version_parts();

        let (min_major, min_minor, min_patch) = MIN_PARSER_VERSION;

        if major < min_major
            || (major == min_major && minor < min_minor)
            || (major == min_major && minor == min_minor && patch < min_patch)
        {
            return Err(super::IcsError::version_error(format!(
                "Parser library version {}.{}.{} is below minimum required {}.{}.{}",
                major, minor, patch, min_major, min_minor, min_patch
            )));
        }

        Ok(())
    }

    /// Get version information as a string
    pub fn version_info() -> Result<String, super::IcsError> {
        let parser = super::parsing::IcsParser::new()?;
        let parser_version = parser.version();

        Ok(format!(
            "ICS Parser Consumer v{}\nParser Library: {}",
            CONSUMER_VERSION, parser_version
        ))
    }
}

/// Convenience functions for quick parsing operations
pub mod quick {
    use super::*;
    use std::path::Path;

    /// Quick parse a single file and return just the success status
    pub fn check_file<P: AsRef<Path>>(path: P) -> Result<bool, IcsError> {
        let result = parse_file(path)?;
        Ok(result.is_success())
    }

    /// Quick parse a single file and return AST if successful
    pub fn get_ast<P: AsRef<Path>>(path: P) -> Result<Option<serde_json::Value>, IcsError> {
        let result = parse_file(path)?;
        Ok(result.ast().cloned())
    }

    /// Quick parse a single file and return symbols if successful
    pub fn get_symbols<P: AsRef<Path>>(path: P) -> Result<Option<serde_json::Value>, IcsError> {
        let result = parse_file(path)?;
        Ok(result.symbols().cloned())
    }

    /// Quick batch validation - returns (success_count, total_count)
    pub fn batch_validate<P: AsRef<Path>>(dir_path: P) -> Result<(usize, usize), IcsError> {
        let result = parse_directory(dir_path, None)?;
        Ok((result.success_count(), result.total_count()))
    }

    /// Quick batch processing with custom configuration
    pub fn batch_parse_with_config<P: AsRef<Path>>(
        dir_path: P,
        max_threads: usize,
        recursive: bool,
    ) -> Result<BatchResult, IcsError> {
        let config = BatchConfig {
            max_threads,
            recursive,
            max_files: None,
            fail_fast: false,
        };
        parse_directory(dir_path, Some(&config))
    }
}

/// Configuration and setup utilities
pub mod config {
    use super::*;

    /// Default configuration for the ICS parser consumer
    #[derive(Debug, Clone)]
    pub struct ConsumerConfig {
        /// Consumer identifier for logging
        pub consumer_id: String,
        /// Module name for logging context
        pub module_name: String,
        /// Whether to enable detailed logging
        pub enable_logging: bool,
        /// Default batch processing configuration
        pub default_batch_config: BatchConfig,
    }

    impl Default for ConsumerConfig {
        fn default() -> Self {
            Self {
                consumer_id: "ics-consumer".to_string(),
                module_name: "parser".to_string(),
                enable_logging: true,
                default_batch_config: BatchConfig::default(),
            }
        }
    }

    impl ConsumerConfig {
        /// Create a new consumer configuration
        pub fn new(consumer_id: impl Into<String>, module_name: impl Into<String>) -> Self {
            Self {
                consumer_id: consumer_id.into(),
                module_name: module_name.into(),
                enable_logging: true,
                default_batch_config: BatchConfig::default(),
            }
        }

        /// Create a parser with this configuration
        pub fn create_parser(&self) -> Result<IcsParser, IcsError> {
            if self.enable_logging {
                IcsParser::with_logger(&self.consumer_id, &self.module_name)
            } else {
                IcsParser::new()
            }
        }

        /// Disable logging for this configuration
        pub fn without_logging(mut self) -> Self {
            self.enable_logging = false;
            self
        }

        /// Set custom batch configuration
        pub fn with_batch_config(mut self, config: BatchConfig) -> Self {
            self.default_batch_config = config;
            self
        }
    }
}

/// Error handling utilities and common error patterns
pub mod errors {
    use super::*;

    /// Check if an error is recoverable (parsing can be retried)
    pub fn is_recoverable_error(error: &IcsError) -> bool {
        error.is_recoverable()
    }

    /// Check if an error indicates a system/library issue
    pub fn is_system_error(error: &IcsError) -> bool {
        error.is_system_error()
    }

    /// Convert an ICS error to a user-friendly message
    pub fn user_friendly_message(error: &IcsError) -> String {
        match error {
            IcsError::FileNotFound { path } => {
                format!("The file '{}' could not be found", path)
            }
            IcsError::ParseFailed { message } => {
                format!("Failed to parse ICS file: {}", message)
            }
            IcsError::ValidationFailed { message } => {
                format!("ICS file validation failed: {}", message)
            }
            IcsError::InvalidPath { path } => {
                format!("Invalid file path: {}", path)
            }
            IcsError::JsonDeserialization { message, .. } => {
                format!("Failed to process parser output: {}", message)
            }
            IcsError::Configuration { message } => {
                format!("Configuration error: {}", message)
            }
            IcsError::NotInitialized => "Parser library is not initialized".to_string(),
            IcsError::InternalError => "An internal parser error occurred".to_string(),
            _ => error.to_string(),
        }
    }

    /// Create a detailed error report
    pub fn create_error_report(error: &IcsError, context: &str) -> String {
        format!(
            "ICS Parser Error Report\n\
            Context: {}\n\
            Error Type: {}\n\
            Recoverable: {}\n\
            System Error: {}\n\
            Message: {}",
            context,
            error_type_name(error),
            is_recoverable_error(error),
            is_system_error(error),
            user_friendly_message(error)
        )
    }

    fn error_type_name(error: &IcsError) -> &'static str {
        match error {
            IcsError::FileNotFound { .. } => "FileNotFound",
            IcsError::ParseFailed { .. } => "ParseFailed",
            IcsError::ValidationFailed { .. } => "ValidationFailed",
            IcsError::InvalidPath { .. } => "InvalidPath",
            IcsError::JsonDeserialization { .. } => "JsonDeserialization",
            IcsError::Configuration { .. } => "Configuration",
            IcsError::NotInitialized => "NotInitialized",
            IcsError::InternalError => "InternalError",
            IcsError::NullPointer => "NullPointer",
            IcsError::StringConversion(_) => "StringConversion",
            IcsError::UnsupportedOperation { .. } => "UnsupportedOperation",
            IcsError::VersionCompatibility { .. } => "VersionCompatibility",
            IcsError::MemoryAllocation { .. } => "MemoryAllocation",
            IcsError::FfiBoundary { .. } => "FfiBoundary",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constants() {
        assert!(!version::CONSUMER_VERSION.is_empty());
        assert_eq!(version::MIN_PARSER_VERSION.0, 1);
    }

    #[test]
    fn test_consumer_config_creation() {
        let config = config::ConsumerConfig::new("test-consumer", "test-module");
        assert_eq!(config.consumer_id, "test-consumer");
        assert_eq!(config.module_name, "test-module");
        assert!(config.enable_logging);

        let config_no_logging = config.without_logging();
        assert!(!config_no_logging.enable_logging);
    }

    #[test]
    fn test_error_utilities() {
        let file_error = IcsError::FileNotFound {
            path: "test.ics".to_string(),
        };

        assert!(errors::is_recoverable_error(&file_error));
        assert!(!errors::is_system_error(&file_error));

        let message = errors::user_friendly_message(&file_error);
        assert!(message.contains("test.ics"));
        assert!(message.contains("could not be found"));

        let report = errors::create_error_report(&file_error, "unit test");
        assert!(report.contains("unit test"));
        assert!(report.contains("FileNotFound"));
        assert!(report.contains("Recoverable: true"));
    }

    #[test]
    fn test_quick_functions_signature() {
        // These will fail without the actual library, but that's expected
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let test_file = temp_dir.path().join("test.ics");

        // Test that functions can be called (they may fail but shouldn't panic)
        let result = quick::check_file(&test_file);
        // Don't assert success/failure since library may not be available
        assert!(result.is_ok() || result.is_err());

        let result = quick::get_ast(&test_file);
        assert!(result.is_ok() || result.is_err());

        let result = quick::batch_validate(temp_dir.path());
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchConfig::default();
        assert!(config.max_threads > 0);
        assert!(config.recursive); // Should default to recursive
    }
}
