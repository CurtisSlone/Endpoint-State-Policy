//! High-level parsing interface for ICS SDK

use super::bindings::{
    ics_create_default_batch_config, ics_free_string, ics_init, ics_parse_directory_json,
    ics_parse_file_json, ics_parser_version, IcsBatchConfig, IcsErrorCode,
};
use super::common::{convert_c_string_to_string, validate_json_format, IcsError};
use super::logging::{consumer_codes, ConsumerLogger};
use super::types::{ExecutionMetadata, PipelineOutput, ProcessingStats};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::path::Path;
use std::sync::Once;

static INIT: Once = Once::new();
static mut LIBRARY_INITIALIZED: bool = false;

/// Ensure the parser library is initialized exactly once
fn ensure_library_initialized() -> Result<(), IcsError> {
    let mut init_success = false;

    INIT.call_once(|| match unsafe { ics_init() } {
        IcsErrorCode::Success => {
            init_success = true;
            unsafe {
                LIBRARY_INITIALIZED = true;
            }
        }
        _error => unsafe {
            LIBRARY_INITIALIZED = false;
        },
    });

    if unsafe { LIBRARY_INITIALIZED } {
        Ok(())
    } else {
        Err(IcsError::NotInitialized)
    }
}

// ============================================================================
// Data Structures (matching the simplified FFI)
// ============================================================================

/// Single file processing result from the parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileOutputResult {
    pub file_path: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub output: Option<PipelineOutput>,
}

/// Batch processing results from the parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOutputResults {
    pub results: Vec<FileOutputResult>,
    pub summary: BatchSummary,
}

/// Basic batch processing summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub total_files: usize,
    pub successful_files: usize,
    pub failed_files: usize,
}

/// Batch processing configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_threads: usize,
    pub recursive: bool,
    pub max_files: Option<usize>,
    pub fail_fast: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        let c_config = unsafe { ics_create_default_batch_config() };
        Self::from(c_config)
    }
}

impl From<IcsBatchConfig> for BatchConfig {
    fn from(config: IcsBatchConfig) -> Self {
        Self {
            max_threads: config.max_threads.max(1) as usize,
            recursive: config.recursive != 0,
            max_files: if config.max_files < 0 {
                None
            } else {
                Some(config.max_files as usize)
            },
            fail_fast: config.fail_fast != 0,
        }
    }
}

impl From<&BatchConfig> for IcsBatchConfig {
    fn from(config: &BatchConfig) -> Self {
        Self {
            max_threads: config.max_threads as i32,
            recursive: if config.recursive { 1 } else { 0 },
            max_files: config.max_files.map(|n| n as i32).unwrap_or(-1),
            fail_fast: if config.fail_fast { 1 } else { 0 },
        }
    }
}

// ============================================================================
// High-Level Parser Interface
// ============================================================================

/// Main ICS parser interface for the SDK
#[derive(Debug)]
pub struct IcsParser {
    initialized: bool,
    logger: Option<ConsumerLogger>,
}

impl IcsParser {
    /// Create a new parser instance
    pub fn new() -> Result<Self, IcsError> {
        ensure_library_initialized()?;
        Ok(Self {
            initialized: true,
            logger: None,
        })
    }

    /// Create a parser with logging integration
    pub fn with_logger(consumer_id: &str, module: &str) -> Result<Self, IcsError> {
        ensure_library_initialized()?;
        let logger = ConsumerLogger::new(consumer_id, module)?;

        logger.info_simple("ICS SDK initialized with parser library")?;

        Ok(Self {
            initialized: true,
            logger: Some(logger),
        })
    }

    /// Parse a single ICS file
    pub fn parse_file<P: AsRef<Path>>(&self, path: P) -> Result<ParseResult, IcsError> {
        if !self.initialized {
            return Err(IcsError::NotInitialized);
        }

        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| IcsError::InvalidPath {
                path: path.as_ref().display().to_string(),
            })?;

        if let Some(ref logger) = self.logger {
            logger.info(
                "Parsing ICS file",
                &[("file_path", path_str), ("operation", "parse_file")],
            )?;
        }

        let c_path = CString::new(path_str)?;

        let result_ptr = unsafe { ics_parse_file_json(c_path.as_ptr()) };

        if result_ptr.is_null() {
            let error = IcsError::FfiBoundary {
                context: "Parser returned null pointer".to_string(),
            };
            if let Some(ref logger) = self.logger {
                let _ = logger.error(
                    consumer_codes::CONSUMER_PIPELINE_ERROR,
                    "Parser returned null result",
                    &[("file_path", path_str)],
                );
            }
            return Err(error);
        }

        let json_string = unsafe {
            let result = convert_c_string_to_string(result_ptr);
            ics_free_string(result_ptr);
            result
        };

        if json_string.is_empty() {
            let error = IcsError::JsonDeserialization {
                message: "Parser returned empty JSON".to_string(),
                json_snippet: "empty".to_string(),
            };
            if let Some(ref logger) = self.logger {
                let _ = logger.error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    "Empty JSON result from parser",
                    &[("file_path", path_str)],
                );
            }
            return Err(error);
        }

        // Parse JSON result
        validate_json_format(&json_string)?;
        let file_result: FileOutputResult = serde_json::from_str(&json_string)
            .map_err(|e| IcsError::json_error_with_snippet(e, &json_string))?;

        let result = ParseResult::new(file_result);

        if let Some(ref logger) = self.logger {
            logger.info(
                "File parsing completed",
                &[
                    ("file_path", path_str),
                    ("success", &result.is_success().to_string()),
                    ("has_output", &result.has_output().to_string()),
                ],
            )?;
        }

        Ok(result)
    }

    /// Parse all ICS files in a directory
    pub fn parse_directory<P: AsRef<Path>>(
        &self,
        path: P,
        config: Option<&BatchConfig>,
    ) -> Result<BatchResult, IcsError> {
        if !self.initialized {
            return Err(IcsError::NotInitialized);
        }

        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| IcsError::InvalidPath {
                path: path.as_ref().display().to_string(),
            })?;

        let batch_config = config.cloned().unwrap_or_default();
        let c_config = IcsBatchConfig::from(&batch_config);

        if let Some(ref logger) = self.logger {
            logger.info(
                "Starting batch processing",
                &[
                    ("directory_path", path_str),
                    ("max_threads", &batch_config.max_threads.to_string()),
                    ("recursive", &batch_config.recursive.to_string()),
                    ("fail_fast", &batch_config.fail_fast.to_string()),
                ],
            )?;
        }

        let c_path = CString::new(path_str)?;

        let result_ptr = unsafe { ics_parse_directory_json(c_path.as_ptr(), c_config) };

        if result_ptr.is_null() {
            let error = IcsError::FfiBoundary {
                context: "Parser returned null pointer for batch processing".to_string(),
            };
            if let Some(ref logger) = self.logger {
                let _ = logger.error(
                    consumer_codes::CONSUMER_IO_ERROR,
                    "Null pointer from batch processing",
                    &[("directory_path", path_str)],
                );
            }
            return Err(error);
        }

        let json_string = unsafe {
            let result = convert_c_string_to_string(result_ptr);
            ics_free_string(result_ptr);
            result
        };

        if json_string.is_empty() {
            let error = IcsError::JsonDeserialization {
                message: "Parser returned empty batch JSON".to_string(),
                json_snippet: "empty".to_string(),
            };
            if let Some(ref logger) = self.logger {
                let _ = logger.error(
                    consumer_codes::CONSUMER_FORMAT_ERROR,
                    "Empty batch JSON from parser",
                    &[("directory_path", path_str)],
                );
            }
            return Err(error);
        }

        // Parse JSON result
        validate_json_format(&json_string)?;
        let batch_result: BatchOutputResults = serde_json::from_str(&json_string)
            .map_err(|e| IcsError::json_error_with_snippet(e, &json_string))?;

        let result = BatchResult::new(batch_result);

        if let Some(ref logger) = self.logger {
            logger.info(
                "Batch processing completed",
                &[
                    ("directory_path", path_str),
                    ("total_files", &result.total_count().to_string()),
                    ("successful_files", &result.success_count().to_string()),
                    ("failed_files", &result.failure_count().to_string()),
                ],
            )?;
        }

        Ok(result)
    }

    /// Get parser library version
    pub fn version(&self) -> String {
        unsafe { convert_c_string_to_string(ics_parser_version()) }
    }

    /// Get parser library version parts
    pub fn version_parts(&self) -> (u32, u32, u32) {
        // Use the string version as fallback since individual version functions aren't available
        let version_str = self.version();

        // Try to parse version string like "1.2.3"
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() >= 3 {
            let major = parts[0].parse::<u32>().unwrap_or(0);
            let minor = parts[1].parse::<u32>().unwrap_or(0);
            let patch = parts[2].parse::<u32>().unwrap_or(0);
            (major, minor, patch)
        } else {
            // Fallback if we can't parse the version string
            (0, 1, 0)
        }
    }

    /// Get the logger instance
    pub fn logger(&self) -> Option<&ConsumerLogger> {
        self.logger.as_ref()
    }
}

impl Default for IcsParser {
    fn default() -> Self {
        Self::new().expect("Failed to initialize ICS parser")
    }
}

// ============================================================================
// Parse Result Wrapper
// ============================================================================

/// Wrapper for individual file parse results
pub struct ParseResult {
    result: FileOutputResult,
}

impl ParseResult {
    pub(crate) fn new(result: FileOutputResult) -> Self {
        Self { result }
    }

    /// Check if parsing was successful
    pub fn is_success(&self) -> bool {
        self.result.success
    }

    /// Get error message if parsing failed
    pub fn error_message(&self) -> Option<&str> {
        self.result.error_message.as_deref()
    }

    /// Check if result has output data
    pub fn has_output(&self) -> bool {
        self.result.output.is_some()
    }

    /// Get the pipeline output (AST and symbols)
    pub fn output(&self) -> Option<&PipelineOutput> {
        self.result.output.as_ref()
    }

    /// Get the file path that was parsed
    pub fn file_path(&self) -> &str {
        &self.result.file_path
    }

    /// Get the AST as JSON value
    pub fn ast(&self) -> Option<&serde_json::Value> {
        self.result.output.as_ref().map(|o| &o.ast_tree)
    }

    /// Get the symbols as JSON value  
    pub fn symbols(&self) -> Option<&serde_json::Value> {
        self.result.output.as_ref().map(|o| &o.symbols)
    }

    /// Extract AST as a typed structure
    pub fn ast_as<T>(&self) -> Result<T, IcsError>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.ast() {
            Some(ast_value) => serde_json::from_value(ast_value.clone()).map_err(|e| {
                IcsError::JsonDeserialization {
                    message: format!("Failed to deserialize AST: {}", e),
                    json_snippet: ast_value.to_string().chars().take(200).collect(),
                }
            }),
            None => Err(IcsError::JsonDeserialization {
                message: "No AST data available".to_string(),
                json_snippet: "null".to_string(),
            }),
        }
    }

    /// Extract symbols as a typed structure
    pub fn symbols_as<T>(&self) -> Result<T, IcsError>
    where
        T: for<'de> Deserialize<'de>,
    {
        match self.symbols() {
            Some(symbols_value) => serde_json::from_value(symbols_value.clone()).map_err(|e| {
                IcsError::JsonDeserialization {
                    message: format!("Failed to deserialize symbols: {}", e),
                    json_snippet: symbols_value.to_string().chars().take(200).collect(),
                }
            }),
            None => Err(IcsError::JsonDeserialization {
                message: "No symbols data available".to_string(),
                json_snippet: "null".to_string(),
            }),
        }
    }

    /// Get processing stats (simplified)
    pub fn get_processing_stats(&self) -> ProcessingStats {
        ProcessingStats {
            success: self.result.success,
            error_message: self.result.error_message.clone(),
            file_path: self.result.file_path.clone(),
        }
    }

    /// Get execution metadata (simplified)
    pub fn get_execution_metadata(&self) -> ExecutionMetadata {
        ExecutionMetadata {
            validation_passed: self.result.success,
            file_path: self.result.file_path.clone(),
            has_output: self.result.output.is_some(),
            processing_stats: self.get_processing_stats(),
        }
    }

    /// Get summary information
    pub fn get_summary(&self) -> ParseSummary {
        ParseSummary {
            file_path: self.result.file_path.clone(),
            success: self.result.success,
            error_message: self.result.error_message.clone(),
            has_ast: self.ast().is_some(),
            has_symbols: self.symbols().is_some(),
        }
    }
}

// ============================================================================
// Batch Result Wrapper
// ============================================================================

/// Wrapper for batch processing results
pub struct BatchResult {
    result: BatchOutputResults,
}

impl BatchResult {
    pub(crate) fn new(result: BatchOutputResults) -> Self {
        Self { result }
    }

    /// Get number of successful files
    pub fn success_count(&self) -> usize {
        self.result.summary.successful_files
    }

    /// Get number of failed files
    pub fn failure_count(&self) -> usize {
        self.result.summary.failed_files
    }

    /// Get total number of files
    pub fn total_count(&self) -> usize {
        self.result.summary.total_files
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.total_count();
        if total == 0 {
            0.0
        } else {
            self.success_count() as f64 / total as f64
        }
    }

    /// Get all file results
    pub fn file_results(&self) -> &[FileOutputResult] {
        &self.result.results
    }

    /// Get successful file results
    pub fn successful_results(&self) -> Vec<&FileOutputResult> {
        self.result.results.iter().filter(|r| r.success).collect()
    }

    /// Get failed file results
    pub fn failed_results(&self) -> Vec<&FileOutputResult> {
        self.result.results.iter().filter(|r| !r.success).collect()
    }

    /// Get batch summary
    pub fn get_summary(&self) -> BatchSummary {
        self.result.summary.clone()
    }
}

// ============================================================================
// Summary Structures
// ============================================================================

/// Summary of parse result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSummary {
    pub file_path: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub has_ast: bool,
    pub has_symbols: bool,
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Parse a single file (convenience function)
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ParseResult, IcsError> {
    let parser = IcsParser::new()?;
    parser.parse_file(path)
}

/// Parse directory (convenience function)  
pub fn parse_directory<P: AsRef<Path>>(
    path: P,
    config: Option<&BatchConfig>,
) -> Result<BatchResult, IcsError> {
    let parser = IcsParser::new()?;
    parser.parse_directory(path, config)
}

/// Get parser version (convenience function)
pub fn get_parser_version() -> Result<String, IcsError> {
    let parser = IcsParser::new()?;
    Ok(parser.version())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_conversion() {
        let config = BatchConfig {
            max_threads: 4,
            recursive: true,
            max_files: Some(100),
            fail_fast: false,
        };

        let c_config = IcsBatchConfig::from(&config);
        assert_eq!(c_config.max_threads, 4);
        assert_eq!(c_config.recursive, 1);
        assert_eq!(c_config.max_files, 100);
        assert_eq!(c_config.fail_fast, 0);
    }

    #[test]
    fn test_parse_summary_creation() {
        let summary = ParseSummary {
            file_path: "test.ics".to_string(),
            success: true,
            error_message: None,
            has_ast: true,
            has_symbols: true,
        };

        assert_eq!(summary.file_path, "test.ics");
        assert!(summary.success);
        assert!(summary.has_ast);
    }
}
