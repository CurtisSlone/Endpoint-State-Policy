//! # ICS Processor
//!
//! Main entry point for processing ICS files up to ExecutionContext delivery.
use crate::api::config::ProcessorConfig;
use crate::api::errors::ProcessorError;
use crate::ffi::logging::ConsumerLogger;
use crate::ffi::parsing::IcsParser;
use crate::resolution::engine::ResolutionEngine;
use crate::types::execution_context::ExecutionContext;
use crate::types::metadata::MetaDataBlock;
use std::path::Path;
use std::time::Instant;
/// Result of processing a single ICS file
#[derive(Debug)]
pub struct ProcessResult {
    /// File path that was processed
    pub file_path: String,
    /// Whether processing succeeded
    pub success: bool,

    /// Execution context (if successful)
    pub execution_context: Option<ExecutionContext>,

    /// Processing errors
    pub errors: Vec<String>,

    /// Processing warnings
    pub warnings: Vec<String>,

    /// Processing duration in milliseconds
    pub duration_ms: u64,

    /// Metadata from ICS file
    pub metadata: Option<MetaDataBlock>,
}
impl ProcessResult {
    /// Check if processing was successful
    pub fn is_success(&self) -> bool {
        self.success && self.execution_context.is_some()
    }
    /// Check if processing failed
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Get the execution context, returning an error if not available
    pub fn context(&self) -> Result<&ExecutionContext, ProcessorError> {
        self.execution_context
            .as_ref()
            .ok_or_else(|| ProcessorError::ProcessingFailed {
                reason: self.errors.join("; "),
            })
    }

    /// Consume this result and return the execution context
    pub fn into_context(self) -> Result<ExecutionContext, ProcessorError> {
        self.execution_context
            .ok_or_else(|| ProcessorError::ProcessingFailed {
                reason: self.errors.join("; "),
            })
    }

    /// Get all errors as a single string
    pub fn error_summary(&self) -> String {
        self.errors.join("; ")
    }

    /// Get all warnings as a single string
    pub fn warning_summary(&self) -> String {
        self.warnings.join("; ")
    }

    /// Get the number of criteria in the execution context
    /// FIXED: Use count_criteria() method instead of .criteria.len()
    pub fn criteria_count(&self) -> usize {
        self.execution_context
            .as_ref()
            .map(|ctx| ctx.count_criteria())
            .unwrap_or(0)
    }

    /// Create a summary string for logging or display
    pub fn summary(&self) -> String {
        if self.success {
            format!(
                "SUCCESS: {} ({} criteria, {}ms)",
                self.file_path,
                self.criteria_count(),
                self.duration_ms
            )
        } else {
            format!(
                "FAILED: {} - {} ({}ms)",
                self.file_path,
                self.error_summary(),
                self.duration_ms
            )
        }
    }
}
impl std::fmt::Display for ProcessResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}
/// ICS Processor for converting ICS files into ExecutionContexts
pub struct IcsProcessor {
    parser: IcsParser,
    resolution_engine: ResolutionEngine,
    config: ProcessorConfig,
    logger: Option<ConsumerLogger>,
}
impl IcsProcessor {
    /// Create a new processor with default configuration
    pub fn new() -> Result<Self, ProcessorError> {
        Self::with_config(ProcessorConfig::default())
    }
    /// Create a new processor with custom configuration
    pub fn with_config(config: ProcessorConfig) -> Result<Self, ProcessorError> {
        // Initialize FFI parser
        let parser = if config.debug_logging {
            IcsParser::with_logger(&config.consumer_id, &config.module_name)
                .map_err(ProcessorError::FfiError)?
        } else {
            IcsParser::new().map_err(ProcessorError::FfiError)?
        };

        // Create logger if enabled
        let logger = if config.debug_logging {
            Some(
                ConsumerLogger::new(&config.consumer_id, &config.module_name)
                    .map_err(ProcessorError::FfiError)?,
            )
        } else {
            None
        };

        // Log initialization
        if let Some(ref logger) = logger {
            let _ = logger.info_simple("ICS Processor initialized");
        }

        Ok(Self {
            parser,
            resolution_engine: ResolutionEngine::new(),
            config,
            logger,
        })
    }

    /// Process a single ICS file and return ExecutionContext
    pub fn process_file(&mut self, path: &str) -> Result<ProcessResult, ProcessorError> {
        let start = Instant::now();

        if let Some(ref logger) = self.logger {
            let _ = logger.info("Processing ICS file", &[("path", path)]);
        }

        // Validate path exists
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(ProcessorError::FileNotFound {
                path: path.to_string(),
            });
        }

        // Phase 1: Parse ICS file via FFI
        let parse_result = self
            .parser
            .parse_file(path)
            .map_err(ProcessorError::FfiError)?;

        if !parse_result.is_success() {
            let error_msg = parse_result
                .error_message()
                .unwrap_or("Unknown parse error")
                .to_string();

            if let Some(ref logger) = self.logger {
                let _ = logger.error(
                    crate::ffi::logging::consumer_codes::CONSUMER_PIPELINE_ERROR,
                    "Parse failed",
                    &[("path", path), ("error", &error_msg)],
                );
            }

            return Ok(ProcessResult {
                file_path: path.to_string(),
                success: false,
                execution_context: None,
                errors: vec![error_msg],
                warnings: vec![],
                duration_ms: start.elapsed().as_millis() as u64,
                metadata: None,
            });
        }

        // Get pipeline output
        let output = parse_result
            .output()
            .ok_or_else(|| ProcessorError::ProcessingFailed {
                reason: "Parse succeeded but no output available".to_string(),
            })?;

        // Phase 2: Resolution engine processing
        let execution_context = match self.resolution_engine.process_pipeline_output(output) {
            Ok(context) => context,
            Err(resolution_error) => {
                let error_msg = resolution_error.to_string();

                if let Some(ref logger) = self.logger {
                    let _ = logger.error(
                        crate::ffi::logging::consumer_codes::CONSUMER_PIPELINE_ERROR,
                        "Resolution failed",
                        &[("path", path), ("error", &error_msg)],
                    );
                }

                return Ok(ProcessResult {
                    file_path: path.to_string(),
                    success: false,
                    execution_context: None,
                    errors: vec![error_msg],
                    warnings: vec![],
                    duration_ms: start.elapsed().as_millis() as u64,
                    metadata: None,
                });
            }
        };

        // Extract metadata
        let metadata = execution_context.metadata.clone();

        // Success
        let duration_ms = start.elapsed().as_millis() as u64;

        if let Some(ref logger) = self.logger {
            let _ = logger.info(
                "Processing completed successfully",
                &[
                    ("path", path),
                    ("duration_ms", &duration_ms.to_string()),
                    // FIXED: Use count_criteria() method
                    (
                        "criteria_count",
                        &execution_context.count_criteria().to_string(),
                    ),
                ],
            );
        }

        Ok(ProcessResult {
            file_path: path.to_string(),
            success: true,
            execution_context: Some(execution_context),
            errors: vec![],
            warnings: vec![],
            duration_ms,
            metadata,
        })
    }

    /// Process all ICS files in a directory
    pub fn process_directory(&mut self, path: &str) -> Result<Vec<ProcessResult>, ProcessorError> {
        let start = Instant::now();

        if let Some(ref logger) = self.logger {
            let _ = logger.info("Processing directory", &[("path", path)]);
        }

        // Validate directory exists
        let path_obj = Path::new(path);
        if !path_obj.exists() {
            return Err(ProcessorError::FileNotFound {
                path: path.to_string(),
            });
        }

        if !path_obj.is_dir() {
            return Err(ProcessorError::ConfigurationError {
                reason: format!("Path is not a directory: {}", path),
            });
        }

        // Use FFI batch processing for parsing
        let batch_result = self
            .parser
            .parse_directory(path, Some(&self.config.batch_config))
            .map_err(ProcessorError::FfiError)?;

        let mut results = Vec::new();

        // Process each parsed file through resolution engine
        for file_result in batch_result.file_results() {
            let file_start = Instant::now();

            if !file_result.success {
                // Parse failed
                results.push(ProcessResult {
                    file_path: file_result.file_path.clone(),
                    success: false,
                    execution_context: None,
                    errors: vec![file_result
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "Unknown error".to_string())],
                    warnings: vec![],
                    duration_ms: 0,
                    metadata: None,
                });
                continue;
            }

            // Get output
            let output = match &file_result.output {
                Some(out) => out,
                None => {
                    results.push(ProcessResult {
                        file_path: file_result.file_path.clone(),
                        success: false,
                        execution_context: None,
                        errors: vec!["No output from parser".to_string()],
                        warnings: vec![],
                        duration_ms: 0,
                        metadata: None,
                    });
                    continue;
                }
            };

            // Process through resolution engine
            match self.resolution_engine.process_pipeline_output(output) {
                Ok(execution_context) => {
                    let metadata = execution_context.metadata.clone();
                    results.push(ProcessResult {
                        file_path: file_result.file_path.clone(),
                        success: true,
                        execution_context: Some(execution_context),
                        errors: vec![],
                        warnings: vec![],
                        duration_ms: file_start.elapsed().as_millis() as u64,
                        metadata,
                    });
                }
                Err(resolution_error) => {
                    results.push(ProcessResult {
                        file_path: file_result.file_path.clone(),
                        success: false,
                        execution_context: None,
                        errors: vec![resolution_error.to_string()],
                        warnings: vec![],
                        duration_ms: file_start.elapsed().as_millis() as u64,
                        metadata: None,
                    });
                }
            }
        }

        let total_duration = start.elapsed().as_millis() as u64;

        if let Some(ref logger) = self.logger {
            let success_count = results.iter().filter(|r| r.success).count();
            let _ = logger.info(
                "Directory processing completed",
                &[
                    ("path", path),
                    ("total_files", &results.len().to_string()),
                    ("successful", &success_count.to_string()),
                    ("duration_ms", &total_duration.to_string()),
                ],
            );
        }

        Ok(results)
    }

    /// Get the configuration
    pub fn config(&self) -> &ProcessorConfig {
        &self.config
    }

    /// Get the logger (if enabled)
    pub fn logger(&self) -> Option<&ConsumerLogger> {
        self.logger.as_ref()
    }
}
impl Default for IcsProcessor {
    fn default() -> Self {
        Self::new().expect("Failed to create default IcsProcessor")
    }
}
