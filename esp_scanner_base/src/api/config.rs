//! # Processor Configuration

use crate::ffi::parsing::BatchConfig;

/// Configuration for the ICS Processor
///
/// Controls logging, batch processing, and other processor behaviors.
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    /// Consumer ID for logging context
    pub consumer_id: String,

    /// Module name for logging context
    pub module_name: String,

    /// Enable debug logging
    pub debug_logging: bool,

    /// Batch processing configuration (for directory operations)
    pub batch_config: BatchConfig,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            consumer_id: "ics-processor".to_string(),
            module_name: "processor".to_string(),
            debug_logging: false,
            batch_config: BatchConfig::default(),
        }
    }
}

impl ProcessorConfig {
    /// Create a new configuration with custom consumer ID and module name
    pub fn new(consumer_id: impl Into<String>, module_name: impl Into<String>) -> Self {
        Self {
            consumer_id: consumer_id.into(),
            module_name: module_name.into(),
            debug_logging: false,
            batch_config: BatchConfig::default(),
        }
    }

    /// Enable debug logging
    pub fn with_debug_logging(mut self) -> Self {
        self.debug_logging = true;
        self
    }

    /// Disable logging
    pub fn without_logging(mut self) -> Self {
        self.debug_logging = false;
        self
    }

    /// Set custom batch configuration
    pub fn with_batch_config(mut self, config: BatchConfig) -> Self {
        self.batch_config = config;
        self
    }

    /// Set maximum threads for batch processing
    pub fn with_max_threads(mut self, max_threads: usize) -> Self {
        self.batch_config.max_threads = max_threads;
        self
    }

    /// Enable or disable recursive directory scanning
    pub fn with_recursive(mut self, recursive: bool) -> Self {
        self.batch_config.recursive = recursive;
        self
    }

    /// Set maximum files to process in batch
    pub fn with_max_files(mut self, max_files: Option<usize>) -> Self {
        self.batch_config.max_files = max_files;
        self
    }

    /// Enable or disable fail-fast mode (stop on first error)
    pub fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.batch_config.fail_fast = fail_fast;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProcessorConfig::default();
        assert_eq!(config.consumer_id, "ics-processor");
        assert_eq!(config.module_name, "processor");
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_config_builder() {
        let config = ProcessorConfig::new("test-consumer", "test-module")
            .with_debug_logging()
            .with_max_threads(8)
            .with_recursive(false)
            .with_fail_fast(true);

        assert_eq!(config.consumer_id, "test-consumer");
        assert_eq!(config.module_name, "test-module");
        assert!(config.debug_logging);
        assert_eq!(config.batch_config.max_threads, 8);
        assert!(!config.batch_config.recursive);
        assert!(config.batch_config.fail_fast);
    }
}
