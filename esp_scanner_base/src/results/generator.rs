//! # Result Generator
//!
//! Converts execution results into SIEM-compatible findings and structured reports.
use crate::execution::engine::CtnResult;
use crate::results::{
    ComplianceCheck, ComplianceFinding, ComplianceResults,
    ComplianceStatus as ResultComplianceStatus, FindingSeverity, ResultGenerationError, ScanResult,
};
use crate::strategies::ComplianceStatus;
/// Result generator for converting execution results to SIEM-compatible format
pub struct ResultGenerator;
impl ResultGenerator {
    /// Generate findings from CTN results and add to scan result
    pub fn generate_findings(
        ctn_results: &[CtnResult],
        scan_result: &mut ScanResult,
    ) -> Result<(), ResultGenerationError> {
        for ctn_result in ctn_results {
            // Only create findings for failures/errors
            if ctn_result.status != ComplianceStatus::Pass {
                let finding = Self::ctn_result_to_finding(ctn_result)?;
                scan_result.add_finding(finding);
            }
        }
        Ok(())
    }

    /// Convert single CTN result to compliance finding
    fn ctn_result_to_finding(
        ctn_result: &CtnResult,
    ) -> Result<ComplianceFinding, ResultGenerationError> {
        let severity = Self::map_status_to_severity(ctn_result.status);

        // Extract description from execution result
        let description = ctn_result.execution_result.message.clone();

        // Extract expected and actual values from state validation results
        let (expected, actual) = Self::extract_expected_and_actual(ctn_result);

        // Create finding with auto-generated ID
        let mut finding = ComplianceFinding::auto_id(
            severity,
            format!("{} validation failed", ctn_result.criterion_type),
            description,
            expected,
            actual,
        );

        // Add field path if we can construct one from state results
        if let Some(path) = Self::construct_field_path(ctn_result) {
            finding = finding.with_field_path(path);
        }

        // Add remediation guidance based on failure type
        if let Some(remediation) = Self::generate_remediation(ctn_result) {
            finding = finding.with_remediation(remediation);
        }

        Ok(finding)
    }

    /// Map ComplianceStatus to FindingSeverity
    fn map_status_to_severity(status: ComplianceStatus) -> FindingSeverity {
        match status {
            ComplianceStatus::Pass => FindingSeverity::Info,
            ComplianceStatus::Fail => FindingSeverity::High,
            ComplianceStatus::Error => FindingSeverity::Critical,
            ComplianceStatus::Unknown => FindingSeverity::Medium,
        }
    }

    /// Extract expected and actual values from state validation results
    fn extract_expected_and_actual(
        ctn_result: &CtnResult,
    ) -> (serde_json::Value, serde_json::Value) {
        let mut expected_map = serde_json::Map::new();
        let mut actual_map = serde_json::Map::new();

        // Iterate through state validation results
        for state_result in &ctn_result.execution_result.state_results {
            for field_result in &state_result.state_results {
                if !field_result.passed {
                    // Add failed field to expected/actual maps
                    expected_map.insert(
                        field_result.field_name.clone(),
                        serde_json::json!(format!("{:?}", field_result.expected_value)),
                    );
                    actual_map.insert(
                        field_result.field_name.clone(),
                        serde_json::json!(format!("{:?}", field_result.actual_value)),
                    );
                }
            }
        }

        // If no state failures, use details from execution result
        if expected_map.is_empty() {
            expected_map.insert("status".to_string(), serde_json::json!("compliant"));
            actual_map.insert(
                "status".to_string(),
                serde_json::json!(format!("{:?}", ctn_result.status)),
            );
        }

        (
            serde_json::Value::Object(expected_map),
            serde_json::Value::Object(actual_map),
        )
    }

    /// Construct field path from failed state results
    fn construct_field_path(ctn_result: &CtnResult) -> Option<String> {
        let failed_fields: Vec<String> = ctn_result
            .execution_result
            .state_results
            .iter()
            .flat_map(|sr| {
                sr.state_results
                    .iter()
                    .filter(|fr| !fr.passed)
                    .map(|fr| format!("{}.{}", sr.object_id, fr.field_name))
            })
            .collect();

        if failed_fields.is_empty() {
            None
        } else {
            Some(failed_fields.join(", "))
        }
    }

    /// Generate remediation guidance based on failure type
    fn generate_remediation(ctn_result: &CtnResult) -> Option<String> {
        // Check if we have state validation failures
        let has_state_failures = ctn_result
            .execution_result
            .state_results
            .iter()
            .any(|sr| !sr.combined_result);

        if has_state_failures {
            Some(format!(
                "Review and correct the configuration values for {} to match the expected state. \
             Refer to the control framework documentation for detailed remediation steps.",
                ctn_result.criterion_type
            ))
        } else if ctn_result.status == ComplianceStatus::Error {
            Some(
            "An error occurred during validation. Check system permissions and resource availability.".to_string()
        )
        } else {
            None
        }
    }

    /// Generate compliance summary statistics from CTN results
    pub fn generate_statistics(ctn_results: &[CtnResult]) -> ComplianceStatistics {
        let mut stats = ComplianceStatistics::default();

        for ctn_result in ctn_results {
            stats.total_criteria += 1;

            match ctn_result.status {
                ComplianceStatus::Pass => stats.passed += 1,
                ComplianceStatus::Fail => stats.failed += 1,
                ComplianceStatus::Error => stats.errors += 1,
                ComplianceStatus::Unknown => stats.unknown += 1,
            }

            stats.total_execution_time_ms += ctn_result.execution_time_ms;
        }

        stats
    }

    /// Build compliance check structure from statistics
    pub fn build_compliance_check(stats: &ComplianceStatistics) -> ComplianceCheck {
        let pass_percentage = if stats.total_criteria > 0 {
            (stats.passed as f32 / stats.total_criteria as f32) * 100.0
        } else {
            0.0
        };

        let status = if stats.errors > 0 {
            ResultComplianceStatus::Partial
        } else if stats.failed == 0 {
            ResultComplianceStatus::Compliant
        } else {
            ResultComplianceStatus::NonCompliant
        };

        ComplianceCheck {
            total_criteria: stats.total_criteria,
            passed_criteria: stats.passed,
            failed_criteria: stats.failed,
            error_criteria: stats.errors,
            pass_percentage,
            status,
        }
    }

    /// Build complete compliance results structure
    pub fn build_compliance_results(
        ctn_results: &[CtnResult],
        findings: Vec<ComplianceFinding>,
    ) -> ComplianceResults {
        let stats = Self::generate_statistics(ctn_results);
        let check = Self::build_compliance_check(&stats);
        let passed = stats.failed == 0 && stats.errors == 0;

        ComplianceResults {
            check,
            findings,
            passed,
        }
    }
}
/// Statistics for compliance scan (internal helper)
#[derive(Debug, Default, Clone)]
pub struct ComplianceStatistics {
    pub total_criteria: u32,
    pub passed: u32,
    pub failed: u32,
    pub errors: u32,
    pub unknown: u32,
    pub total_execution_time_ms: u64,
}
impl ComplianceStatistics {
    /// Calculate pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total_criteria == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total_criteria as f64) * 100.0
        }
    }
    /// Check if scan was fully compliant
    pub fn is_compliant(&self) -> bool {
        self.failed == 0 && self.errors == 0
    }

    /// Get average execution time per criterion
    pub fn avg_execution_time_ms(&self) -> f64 {
        if self.total_criteria == 0 {
            0.0
        } else {
            self.total_execution_time_ms as f64 / self.total_criteria as f64
        }
    }
}
