//! # ICS Scanner Implementation
//!
//! Complete scanner that orchestrates ICS processing and compliance validation.
use crate::api::{IcsProcessor, ProcessResult, ProcessorConfig, ProcessorError};
use crate::execution::ExecutionEngine;
use crate::results::ScanResult;
use crate::strategies::CtnStrategyRegistry;
use std::path::Path;
use std::sync::Arc;
/// Complete ICS compliance scanner
pub struct IcsScanner {
    processor: IcsProcessor,
    registry: Arc<CtnStrategyRegistry>,
}
impl IcsScanner {
    /// Create scanner with strategy registry
    pub fn new(registry: CtnStrategyRegistry) -> Result<Self, ScannerError> {
        Ok(Self {
            processor: IcsProcessor::new()?,
            registry: Arc::new(registry),
        })
    }
    /// Create scanner with custom config and registry
    pub fn with_config(
        config: ProcessorConfig,
        registry: CtnStrategyRegistry,
    ) -> Result<Self, ScannerError> {
        Ok(Self {
            processor: IcsProcessor::with_config(config)?,
            registry: Arc::new(registry),
        })
    }

    /// Scan a single ICS file and return structured result
    pub fn scan_file(&mut self, path: &str) -> Result<ScanResult, ScannerError> {
        // Phase 1: Process ICS file to ExecutionContext
        let process_result = self.processor.process_file(path)?;

        if !process_result.success {
            return Err(ScannerError::ProcessingFailed {
                reason: format!("Processing failed: {:?}", process_result.errors),
            });
        }

        let execution_context = process_result.into_context()?;

        // Phase 2: Execute compliance validation
        // FIXED: No scan_id parameter needed
        let mut engine = ExecutionEngine::new(execution_context, self.registry.clone());
        let scan_result = engine.execute()?;

        Ok(scan_result)
    }

    /// Scan directory of ICS files
    pub fn scan_directory(&mut self, path: &str) -> Result<BatchScanResult, ScannerError> {
        let process_results = self.processor.process_directory(path)?;

        let mut scan_results = Vec::new();
        let mut failed_files = Vec::new();

        for process_result in process_results {
            let file_path = process_result.file_path.clone();

            if !process_result.success {
                failed_files.push((file_path, "Processing failed".to_string()));
                continue;
            }

            let execution_context = match process_result.into_context() {
                Ok(ctx) => ctx,
                Err(e) => {
                    failed_files.push((file_path, format!("Context creation failed: {}", e)));
                    continue;
                }
            };

            // FIXED: No scan_id parameter needed
            let mut engine = ExecutionEngine::new(execution_context, self.registry.clone());
            match engine.execute() {
                Ok(result) => scan_results.push(result),
                Err(e) => {
                    failed_files.push((file_path, format!("Execution failed: {}", e)));
                }
            }
        }

        Ok(BatchScanResult::new(
            path.to_string(),
            scan_results,
            failed_files,
        ))
    }

    /// Get registry for inspection
    pub fn registry(&self) -> &CtnStrategyRegistry {
        &self.registry
    }

    /// Get mutable registry for strategy registration
    pub fn registry_mut(&mut self) -> &mut CtnStrategyRegistry {
        Arc::get_mut(&mut self.registry)
            .expect("Cannot get mutable registry while shared references exist")
    }
}
/// Batch scan results for directory scans
#[derive(Debug)]
pub struct BatchScanResult {
    pub directory_path: String,
    pub scan_results: Vec<ScanResult>,
    pub failed_files: Vec<(String, String)>,
    pub statistics: BatchStatistics,
}
#[derive(Debug)]
pub struct BatchStatistics {
    pub total_files: usize,
    pub successful_scans: usize,
    pub failed_scans: usize,
    pub compliant_scans: usize,
    pub non_compliant_scans: usize,
    pub total_findings: usize,
}
impl BatchScanResult {
    fn new(
        directory_path: String,
        scan_results: Vec<ScanResult>,
        failed_files: Vec<(String, String)>,
    ) -> Self {
        let total_files = scan_results.len() + failed_files.len();
        let successful_scans = scan_results.len();
        let failed_scans = failed_files.len();
        let compliant_scans = scan_results.iter().filter(|r| r.results.passed).count();
        let non_compliant_scans = successful_scans - compliant_scans;
        let total_findings = scan_results.iter().map(|r| r.results.findings.len()).sum();
        Self {
            directory_path,
            scan_results,
            failed_files,
            statistics: BatchStatistics {
                total_files,
                successful_scans,
                failed_scans,
                compliant_scans,
                non_compliant_scans,
                total_findings,
            },
        }
    }

    /// Export all results as JSON array
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.scan_results)
    }

    /// Get summary report
    pub fn summary(&self) -> String {
        format!(
            "Batch Scan Summary:\n\
         Directory: {}\n\
         Total Files: {}\n\
         Successful Scans: {}\n\
         Failed Scans: {}\n\
         Compliant: {}\n\
         Non-Compliant: {}\n\
         Total Findings: {}",
            self.directory_path,
            self.statistics.total_files,
            self.statistics.successful_scans,
            self.statistics.failed_scans,
            self.statistics.compliant_scans,
            self.statistics.non_compliant_scans,
            self.statistics.total_findings
        )
    }
}
/// Scanner errors
#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    #[error("Processing failed: {reason}")]
    ProcessingFailed { reason: String },
    #[error("Execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Processor error: {0}")]
    ProcessorError(#[from] ProcessorError),

    #[error("Execution error: {0}")]
    ExecutionError(#[from] crate::execution::ExecutionError),

    #[error("Strategy error: {0}")]
    StrategyError(#[from] crate::strategies::StrategyError),

    #[error("Result generation error: {0}")]
    ResultError(#[from] crate::results::ResultGenerationError),
}
