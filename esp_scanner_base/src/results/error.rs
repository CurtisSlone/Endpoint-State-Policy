// results/error.rs

/// Result-specific processing errors
#[derive(Debug, Clone)]
pub enum ResultGenerationError {
    /// Failed to generate scan result from resolution context
    ScanResultGenerationFailed { scan_id: String, cause: String },

    /// Failed to extract metadata from ICS definition
    MetadataExtractionFailed {
        scan_id: String,
        missing_field: String,
        cause: String,
    },

    /// Invalid scan identifier format
    InvalidScanIdFormat {
        scan_id: String,
        format_requirements: String,
    },

    /// Failed to create host context
    HostContextCreationFailed { hostname: String, cause: String },

    /// Failed to create user context
    UserContextCreationFailed { username: String, cause: String },

    /// Compliance finding generation failed
    ComplianceFindingGenerationFailed {
        finding_id: String,
        field_path: String,
        cause: String,
    },

    /// Invalid severity level specified
    InvalidSeverityLevel {
        severity: String,
        valid_levels: Vec<String>,
    },

    /// Timestamp calculation failed
    TimestampCalculationFailed {
        scan_id: String,
        calculation_type: String,
        cause: String,
    },

    /// JSON serialization failed
    JsonSerializationFailed {
        scan_id: String,
        format: String,
        cause: String,
    },

    /// JSON deserialization failed
    JsonDeserializationFailed { source_type: String, cause: String },

    /// Compliance metrics calculation failed
    MetricsCalculationFailed {
        scan_id: String,
        metric_type: String,
        cause: String,
    },

    /// Result finalization failed
    ResultFinalizationFailed {
        scan_id: String,
        finalization_step: String,
        cause: String,
    },

    /// Invalid criteria count values
    InvalidCriteriaCount {
        scan_id: String,
        total: u32,
        passed: u32,
        failed: u32,
        errors: u32,
        validation_error: String,
    },

    /// Finding ID generation failed
    FindingIdGenerationFailed { context: String, cause: String },

    /// Finding severity mapping failed
    FindingSeverityMappingFailed {
        field_name: String,
        field_value: String,
        mapping_error: String,
    },

    /// Result aggregation failed (for future multi-scan support)
    ResultAggregationFailed {
        scan_ids: Vec<String>,
        aggregation_type: String,
        cause: String,
    },

    /// Invalid compliance status transition
    InvalidComplianceStatusTransition {
        scan_id: String,
        from_status: String,
        to_status: String,
        reason: String,
    },

    /// System information collection failed
    SystemInfoCollectionFailed { info_type: String, cause: String },

    /// Process information extraction failed
    ProcessInfoExtractionFailed { process_id: u32, cause: String },

    /// Duration calculation overflow
    DurationCalculationOverflow {
        scan_id: String,
        start_time: String,
        end_time: String,
    },

    /// ICS metadata validation failed during result generation
    IcsMetadataValidationFailed {
        scan_id: String,
        validation_errors: Vec<String>,
    },

    /// Result structure validation failed
    ResultStructureValidationFailed {
        scan_id: String,
        validation_rule: String,
        cause: String,
    },

    /// Export format not supported
    UnsupportedExportFormat {
        requested_format: String,
        supported_formats: Vec<String>,
    },

    /// Result transformation failed
    ResultTransformationFailed {
        scan_id: String,
        from_format: String,
        to_format: String,
        cause: String,
    },

    /// Concurrent result access violation
    ConcurrentAccessViolation {
        scan_id: String,
        operation: String,
        cause: String,
    },
}

impl ResultGenerationError {
    /// Create scan result generation error
    pub fn scan_result_generation_failed(scan_id: &str, cause: &str) -> Self {
        Self::ScanResultGenerationFailed {
            scan_id: scan_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create metadata extraction error
    pub fn metadata_extraction_failed(scan_id: &str, missing_field: &str, cause: &str) -> Self {
        Self::MetadataExtractionFailed {
            scan_id: scan_id.to_string(),
            missing_field: missing_field.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid scan ID format error
    pub fn invalid_scan_id_format(scan_id: &str, format_requirements: &str) -> Self {
        Self::InvalidScanIdFormat {
            scan_id: scan_id.to_string(),
            format_requirements: format_requirements.to_string(),
        }
    }

    /// Create host context creation error
    pub fn host_context_creation_failed(hostname: &str, cause: &str) -> Self {
        Self::HostContextCreationFailed {
            hostname: hostname.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create user context creation error
    pub fn user_context_creation_failed(username: &str, cause: &str) -> Self {
        Self::UserContextCreationFailed {
            username: username.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create compliance finding generation error
    pub fn compliance_finding_generation_failed(
        finding_id: &str,
        field_path: &str,
        cause: &str,
    ) -> Self {
        Self::ComplianceFindingGenerationFailed {
            finding_id: finding_id.to_string(),
            field_path: field_path.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid severity level error
    pub fn invalid_severity_level(severity: &str) -> Self {
        Self::InvalidSeverityLevel {
            severity: severity.to_string(),
            valid_levels: vec![
                "critical".to_string(),
                "high".to_string(),
                "medium".to_string(),
                "low".to_string(),
                "info".to_string(),
            ],
        }
    }

    /// Create JSON serialization error
    pub fn json_serialization_failed(scan_id: &str, format: &str, cause: &str) -> Self {
        Self::JsonSerializationFailed {
            scan_id: scan_id.to_string(),
            format: format.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create JSON deserialization error
    pub fn json_deserialization_failed(source_type: &str, cause: &str) -> Self {
        Self::JsonDeserializationFailed {
            source_type: source_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create metrics calculation error
    pub fn metrics_calculation_failed(scan_id: &str, metric_type: &str, cause: &str) -> Self {
        Self::MetricsCalculationFailed {
            scan_id: scan_id.to_string(),
            metric_type: metric_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create result finalization error
    pub fn result_finalization_failed(scan_id: &str, finalization_step: &str, cause: &str) -> Self {
        Self::ResultFinalizationFailed {
            scan_id: scan_id.to_string(),
            finalization_step: finalization_step.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid criteria count error
    pub fn invalid_criteria_count(
        scan_id: &str,
        total: u32,
        passed: u32,
        failed: u32,
        errors: u32,
        validation_error: &str,
    ) -> Self {
        Self::InvalidCriteriaCount {
            scan_id: scan_id.to_string(),
            total,
            passed,
            failed,
            errors,
            validation_error: validation_error.to_string(),
        }
    }

    /// Create finding ID generation error
    pub fn finding_id_generation_failed(context: &str, cause: &str) -> Self {
        Self::FindingIdGenerationFailed {
            context: context.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create system info collection error
    pub fn system_info_collection_failed(info_type: &str, cause: &str) -> Self {
        Self::SystemInfoCollectionFailed {
            info_type: info_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create ICS metadata validation error
    pub fn ics_metadata_validation_failed(scan_id: &str, validation_errors: Vec<String>) -> Self {
        Self::IcsMetadataValidationFailed {
            scan_id: scan_id.to_string(),
            validation_errors,
        }
    }

    /// Create unsupported export format error
    pub fn unsupported_export_format(
        requested_format: &str,
        supported_formats: Vec<String>,
    ) -> Self {
        Self::UnsupportedExportFormat {
            requested_format: requested_format.to_string(),
            supported_formats,
        }
    }
}

impl std::fmt::Display for ResultGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScanResultGenerationFailed { scan_id, cause } => {
                write!(
                    f,
                    "Failed to generate scan result for '{}': {}",
                    scan_id, cause
                )
            }
            Self::MetadataExtractionFailed {
                scan_id,
                missing_field,
                cause,
            } => {
                write!(
                    f,
                    "Failed to extract metadata field '{}' for scan '{}': {}",
                    missing_field, scan_id, cause
                )
            }
            Self::InvalidScanIdFormat {
                scan_id,
                format_requirements,
            } => {
                write!(
                    f,
                    "Invalid scan ID format '{}': {}",
                    scan_id, format_requirements
                )
            }
            Self::HostContextCreationFailed { hostname, cause } => {
                write!(
                    f,
                    "Failed to create host context for '{}': {}",
                    hostname, cause
                )
            }
            Self::UserContextCreationFailed { username, cause } => {
                write!(
                    f,
                    "Failed to create user context for '{}': {}",
                    username, cause
                )
            }
            Self::ComplianceFindingGenerationFailed {
                finding_id,
                field_path,
                cause,
            } => {
                write!(
                    f,
                    "Failed to generate compliance finding '{}' for field '{}': {}",
                    finding_id, field_path, cause
                )
            }
            Self::InvalidSeverityLevel {
                severity,
                valid_levels,
            } => {
                write!(
                    f,
                    "Invalid severity level '{}'. Valid levels: [{}]",
                    severity,
                    valid_levels.join(", ")
                )
            }
            Self::TimestampCalculationFailed {
                scan_id,
                calculation_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to calculate {} timestamp for scan '{}': {}",
                    calculation_type, scan_id, cause
                )
            }
            Self::JsonSerializationFailed {
                scan_id,
                format,
                cause,
            } => {
                write!(
                    f,
                    "Failed to serialize scan '{}' to {} format: {}",
                    scan_id, format, cause
                )
            }
            Self::JsonDeserializationFailed { source_type, cause } => {
                write!(
                    f,
                    "Failed to deserialize {} from JSON: {}",
                    source_type, cause
                )
            }
            Self::MetricsCalculationFailed {
                scan_id,
                metric_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to calculate {} metrics for scan '{}': {}",
                    metric_type, scan_id, cause
                )
            }
            Self::ResultFinalizationFailed {
                scan_id,
                finalization_step,
                cause,
            } => {
                write!(
                    f,
                    "Failed to finalize scan '{}' at step '{}': {}",
                    scan_id, finalization_step, cause
                )
            }
            Self::InvalidCriteriaCount {
                scan_id,
                total,
                passed,
                failed,
                errors,
                validation_error,
            } => {
                write!(f, "Invalid criteria counts for scan '{}' (total:{}, passed:{}, failed:{}, errors:{}): {}", 
                       scan_id, total, passed, failed, errors, validation_error)
            }
            Self::FindingIdGenerationFailed { context, cause } => {
                write!(
                    f,
                    "Failed to generate finding ID in context '{}': {}",
                    context, cause
                )
            }
            Self::FindingSeverityMappingFailed {
                field_name,
                field_value,
                mapping_error,
            } => {
                write!(
                    f,
                    "Failed to map severity for field '{}' with value '{}': {}",
                    field_name, field_value, mapping_error
                )
            }
            Self::ResultAggregationFailed {
                scan_ids,
                aggregation_type,
                cause,
            } => {
                write!(
                    f,
                    "Failed to aggregate {} results for scans [{}]: {}",
                    aggregation_type,
                    scan_ids.join(", "),
                    cause
                )
            }
            Self::InvalidComplianceStatusTransition {
                scan_id,
                from_status,
                to_status,
                reason,
            } => {
                write!(
                    f,
                    "Invalid compliance status transition for scan '{}': {} -> {} ({})",
                    scan_id, from_status, to_status, reason
                )
            }
            Self::SystemInfoCollectionFailed { info_type, cause } => {
                write!(
                    f,
                    "Failed to collect {} system information: {}",
                    info_type, cause
                )
            }
            Self::ProcessInfoExtractionFailed { process_id, cause } => {
                write!(
                    f,
                    "Failed to extract process information for PID {}: {}",
                    process_id, cause
                )
            }
            Self::DurationCalculationOverflow {
                scan_id,
                start_time,
                end_time,
            } => {
                write!(
                    f,
                    "Duration calculation overflow for scan '{}' (start: {}, end: {})",
                    scan_id, start_time, end_time
                )
            }
            Self::IcsMetadataValidationFailed {
                scan_id,
                validation_errors,
            } => {
                write!(
                    f,
                    "ICS metadata validation failed for scan '{}': [{}]",
                    scan_id,
                    validation_errors.join(", ")
                )
            }
            Self::ResultStructureValidationFailed {
                scan_id,
                validation_rule,
                cause,
            } => {
                write!(
                    f,
                    "Result structure validation failed for scan '{}' (rule: {}): {}",
                    scan_id, validation_rule, cause
                )
            }
            Self::UnsupportedExportFormat {
                requested_format,
                supported_formats,
            } => {
                write!(
                    f,
                    "Unsupported export format '{}'. Supported formats: [{}]",
                    requested_format,
                    supported_formats.join(", ")
                )
            }
            Self::ResultTransformationFailed {
                scan_id,
                from_format,
                to_format,
                cause,
            } => {
                write!(
                    f,
                    "Failed to transform scan '{}' from {} to {}: {}",
                    scan_id, from_format, to_format, cause
                )
            }
            Self::ConcurrentAccessViolation {
                scan_id,
                operation,
                cause,
            } => {
                write!(
                    f,
                    "Concurrent access violation for scan '{}' during operation '{}': {}",
                    scan_id, operation, cause
                )
            }
        }
    }
}

impl std::error::Error for ResultGenerationError {}
