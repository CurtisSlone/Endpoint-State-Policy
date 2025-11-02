//! # Simplified data structures for ICS parser consumption
//!
//! These types represent the simplified JSON output from the new FFI interface.
//! The new library only exposes AST and symbols data, not execution metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pipeline output containing only essential parsing data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineOutput {
    /// The parsed AST as a JSON value
    pub ast_tree: serde_json::Value,
    /// Symbol discovery results as a JSON value  
    pub symbols: serde_json::Value,
}

/// Processing statistics (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Basic success/failure status
    pub success: bool,
    /// Error message if processing failed
    pub error_message: Option<String>,
    /// File path that was processed
    pub file_path: String,
}

impl ProcessingStats {
    /// Create stats for successful processing
    pub fn success(file_path: String) -> Self {
        Self {
            success: true,
            error_message: None,
            file_path,
        }
    }

    /// Create stats for failed processing
    pub fn failure(file_path: String, error: String) -> Self {
        Self {
            success: false,
            error_message: Some(error),
            file_path,
        }
    }

    /// Check if processing was successful
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Get error message if processing failed
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Get the processed file path
    pub fn file_path(&self) -> &str {
        &self.file_path
    }
}

/// Metadata about the parsing operation (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Whether parsing completed successfully
    pub validation_passed: bool,
    /// File path that was processed
    pub file_path: String,
    /// Whether output data is available
    pub has_output: bool,
    /// Basic processing information
    pub processing_stats: ProcessingStats,
}

impl ExecutionMetadata {
    /// Create metadata for successful processing
    pub fn success(file_path: String, has_output: bool) -> Self {
        Self {
            validation_passed: true,
            file_path: file_path.clone(),
            has_output,
            processing_stats: ProcessingStats::success(file_path),
        }
    }

    /// Create metadata for failed processing  
    pub fn failure(file_path: String, error: String) -> Self {
        Self {
            validation_passed: false,
            file_path: file_path.clone(),
            has_output: false,
            processing_stats: ProcessingStats::failure(file_path, error),
        }
    }

    /// Check if the processing is ready for further use
    pub fn is_ready_for_execution(&self) -> bool {
        self.validation_passed && self.has_output
    }

    /// Get a summary string of the processing results
    pub fn summary(&self) -> String {
        format!(
            "Validation: {}, File: {}, Has Output: {}",
            if self.validation_passed {
                "PASS"
            } else {
                "FAIL"
            },
            self.file_path,
            self.has_output
        )
    }
}

/// Simple execution data wrapper (replaces the complex ExecutionData)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionData {
    /// The pipeline output (AST + symbols)
    pub output: Option<PipelineOutput>,
    /// Processing metadata
    pub metadata: ExecutionMetadata,
    /// Any resolved variables (empty for simplified version)
    pub resolved_variables: HashMap<String, String>,
}

impl ExecutionData {
    /// Create execution data from pipeline output
    pub fn from_output(file_path: String, output: Option<PipelineOutput>) -> Self {
        let has_output = output.is_some();
        let metadata = if has_output {
            ExecutionMetadata::success(file_path, true)
        } else {
            ExecutionMetadata::failure(file_path, "No output data available".to_string())
        };

        Self {
            output,
            metadata,
            resolved_variables: HashMap::new(),
        }
    }

    /// Create execution data for a failed operation
    pub fn failure(file_path: String, error: String) -> Self {
        Self {
            output: None,
            metadata: ExecutionMetadata::failure(file_path, error),
            resolved_variables: HashMap::new(),
        }
    }

    /// Check if execution data is available and valid
    pub fn is_valid(&self) -> bool {
        self.metadata.validation_passed && self.output.is_some()
    }

    /// Get the AST data if available
    pub fn ast(&self) -> Option<&serde_json::Value> {
        self.output.as_ref().map(|o| &o.ast_tree)
    }

    /// Get the symbols data if available
    pub fn symbols(&self) -> Option<&serde_json::Value> {
        self.output.as_ref().map(|o| &o.symbols)
    }

    /// Check if execution data has any useful content
    pub fn is_empty(&self) -> bool {
        self.output.is_none() && self.resolved_variables.is_empty()
    }

    /// Get a summary of the execution data
    pub fn summary(&self) -> String {
        format!(
            "Valid: {}, Has AST: {}, Has Symbols: {}, Variables: {}",
            self.is_valid(),
            self.ast().is_some(),
            self.symbols().is_some(),
            self.resolved_variables.len()
        )
    }
}

impl PipelineOutput {
    /// Create a new pipeline output
    pub fn new(ast_tree: serde_json::Value, symbols: serde_json::Value) -> Self {
        Self { ast_tree, symbols }
    }

    /// Check if the pipeline output has valid data
    pub fn is_valid(&self) -> bool {
        !self.ast_tree.is_null() && !self.symbols.is_null()
    }

    /// Get the AST as a specific type
    pub fn ast_as<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.ast_tree.clone())
    }

    /// Get the symbols as a specific type
    pub fn symbols_as<T>(&self) -> Result<T, serde_json::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.symbols.clone())
    }

    /// Get the AST as a string
    pub fn ast_as_string(&self) -> String {
        self.ast_tree.to_string()
    }

    /// Get the symbols as a string
    pub fn symbols_as_string(&self) -> String {
        self.symbols.to_string()
    }

    /// Check if AST is empty
    pub fn ast_is_empty(&self) -> bool {
        self.ast_tree.is_null()
            || (self.ast_tree.is_object() && self.ast_tree.as_object().unwrap().is_empty())
    }

    /// Check if symbols are empty
    pub fn symbols_is_empty(&self) -> bool {
        self.symbols.is_null()
            || (self.symbols.is_object() && self.symbols.as_object().unwrap().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_processing_stats_creation() {
        let success_stats = ProcessingStats::success("test.ics".to_string());
        assert!(success_stats.is_success());
        assert_eq!(success_stats.file_path(), "test.ics");
        assert!(success_stats.error_message().is_none());

        let failure_stats =
            ProcessingStats::failure("test.ics".to_string(), "Parse failed".to_string());
        assert!(!failure_stats.is_success());
        assert_eq!(failure_stats.file_path(), "test.ics");
        assert_eq!(failure_stats.error_message(), Some("Parse failed"));
    }

    #[test]
    fn test_execution_metadata_creation() {
        let success_metadata = ExecutionMetadata::success("test.ics".to_string(), true);
        assert!(success_metadata.is_ready_for_execution());
        assert!(success_metadata.validation_passed);
        assert!(success_metadata.has_output);

        let failure_metadata =
            ExecutionMetadata::failure("test.ics".to_string(), "Failed".to_string());
        assert!(!failure_metadata.is_ready_for_execution());
        assert!(!failure_metadata.validation_passed);
        assert!(!failure_metadata.has_output);
    }

    #[test]
    fn test_pipeline_output_creation() {
        let ast = json!({"root": {"type": "IcsFile", "children": []}});
        let symbols = json!({"symbol_count": 5, "symbols": []});

        let output = PipelineOutput::new(ast.clone(), symbols.clone());
        assert!(output.is_valid());
        assert_eq!(output.ast_tree, ast);
        assert_eq!(output.symbols, symbols);
        assert!(!output.ast_is_empty());
        assert!(!output.symbols_is_empty());
    }

    #[test]
    fn test_execution_data_from_output() {
        let ast = json!({"type": "IcsFile"});
        let symbols = json!({"count": 1});
        let output = PipelineOutput::new(ast, symbols);

        let exec_data = ExecutionData::from_output("test.ics".to_string(), Some(output));
        assert!(exec_data.is_valid());
        assert!(exec_data.ast().is_some());
        assert!(exec_data.symbols().is_some());
        assert!(!exec_data.is_empty());

        let failed_data =
            ExecutionData::failure("test.ics".to_string(), "Parse failed".to_string());
        assert!(!failed_data.is_valid());
        assert!(failed_data.ast().is_none());
        assert!(failed_data.symbols().is_none());
    }

    #[test]
    fn test_pipeline_output_type_conversion() {
        let ast = json!({"name": "test", "value": 42});
        let symbols = json!({"items": ["sym1", "sym2"]});

        let output = PipelineOutput::new(ast, symbols);

        // Test successful conversion
        #[derive(Deserialize)]
        struct TestAst {
            name: String,
            value: i32,
        }

        let ast_typed: Result<TestAst, _> = output.ast_as();
        assert!(ast_typed.is_ok());
        assert_eq!(ast_typed.unwrap().name, "test");

        // Test string conversion
        let ast_str = output.ast_as_string();
        assert!(ast_str.contains("test"));
        assert!(ast_str.contains("42"));
    }

    #[test]
    fn test_empty_pipeline_output() {
        let empty_ast = json!(null);
        let empty_symbols = json!({});

        let output = PipelineOutput::new(empty_ast, empty_symbols.clone());
        assert!(!output.is_valid()); // null AST makes it invalid
        assert!(output.ast_is_empty());
        assert!(output.symbols_is_empty());

        let valid_empty = PipelineOutput::new(json!({}), empty_symbols);
        assert!(valid_empty.is_valid()); // empty object is valid
        assert!(valid_empty.ast_is_empty());
        assert!(valid_empty.symbols_is_empty());
    }

    #[test]
    fn test_execution_data_summary() {
        let output = PipelineOutput::new(json!({"test": true}), json!({"count": 1}));
        let exec_data = ExecutionData::from_output("test.ics".to_string(), Some(output));

        let summary = exec_data.summary();
        assert!(summary.contains("Valid: true"));
        assert!(summary.contains("Has AST: true"));
        assert!(summary.contains("Has Symbols: true"));
        assert!(summary.contains("Variables: 0"));
    }
}
