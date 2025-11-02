//! # Scan Result Types
//!
//! Core data structures for ICS compliance validation results.
//! These types are designed for serialization to JSON and integration
//! with SIEM/SOAR tools and compliance reporting systems.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete scan result for one ICS definition file
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    /// Unique identifier for this scan execution
    pub scan_id: String,

    /// Metadata about the scan and definition
    pub metadata: ScanMetadata,

    /// Results of the configuration compliance check
    pub results: ComplianceResults,
}

/// Metadata for the scan execution and ICS definition
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanMetadata {
    /// Required ICS metadata fields for SIEM/SOAR integration
    #[serde(rename = "META")]
    pub ics_metadata: IcsMetadata,

    /// Host information where scan was executed
    pub host: HostContext,

    /// User context for scan execution
    pub user_context: UserContext,

    /// Scan execution timestamps
    pub timestamp: TimestampInfo,
}

/// Required fields from ICS META block for SIEM/SOAR output
#[derive(Debug, Serialize, Deserialize)]
pub struct IcsMetadata {
    /// Unique identifier for this ICS scan definition
    pub ics_scan_id: String,

    /// Control framework being validated (e.g., "NIST", "CIS", "PCI-DSS")
    pub control_framework: String,

    /// Specific control identifier (e.g., "AC-2", "CIS-5.1.1")
    pub control: String,

    /// Target platform (e.g., "Windows", "Linux", "Network")
    pub platform: String,

    /// Criticality level (low, medium, high, critical)
    pub criticality: String,

    /// Tags for categorization and filtering
    pub tags: String,
}

/// Host execution context
#[derive(Debug, Serialize, Deserialize)]
pub struct HostContext {
    /// Hostname where scan executed
    pub hostname: String,

    /// Operating system information
    pub os_info: String,

    /// IP address of scanning host
    pub ip_address: Option<String>,

    /// Additional host identifiers for asset correlation
    pub asset_id: Option<String>,
}

/// User execution context
#[derive(Debug, Serialize, Deserialize)]
pub struct UserContext {
    /// User account that executed the scan
    pub username: String,

    /// Execution privileges level
    pub privilege_level: String,

    /// Process information
    pub process_info: Option<String>,
}

/// Timestamp information for scan execution
#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampInfo {
    /// When scan execution started (RFC3339 format)
    pub scan_start: DateTime<Utc>,

    /// When scan execution completed (RFC3339 format)
    pub scan_end: DateTime<Utc>,

    /// Total execution duration in milliseconds
    pub duration_ms: u64,
}

/// Results of configuration compliance validation
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceResults {
    /// Overall compliance check result
    pub check: ComplianceCheck,

    /// Detailed findings from validation failures
    pub findings: Vec<ComplianceFinding>,

    /// Overall pass/fail status for the entire ICS definition
    pub passed: bool,
}

/// Summary of compliance validation execution
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceCheck {
    /// Total number of validation criteria in ICS definition
    pub total_criteria: u32,

    /// Number of criteria that passed validation
    pub passed_criteria: u32,

    /// Number of criteria that failed validation
    pub failed_criteria: u32,

    /// Number of criteria that had execution errors
    pub error_criteria: u32,

    /// Percentage of criteria that passed (0-100)
    pub pass_percentage: f32,

    /// Overall compliance status
    pub status: ComplianceStatus,
}

/// Overall compliance status enumeration
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplianceStatus {
    /// All criteria passed validation
    Compliant,

    /// Some criteria failed validation
    NonCompliant,

    /// Scan completed but with execution errors
    Partial,

    /// Scan failed to execute properly
    Error,
}

/// Individual compliance violation or issue
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplianceFinding {
    /// Unique identifier for this finding
    pub finding_id: String,

    /// Severity level of the compliance violation
    pub severity: FindingSeverity,

    /// Human-readable title of the finding
    pub title: String,

    /// Detailed description of what was found
    pub description: String,

    /// Expected configuration value
    pub expected: serde_json::Value,

    /// Actual configuration value found
    pub actual: serde_json::Value,

    /// Remediation guidance
    pub remediation: Option<String>,

    /// Field path that failed validation
    pub field_path: Option<String>,
}

/// Severity levels for compliance findings
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    /// Critical compliance violation
    Critical,

    /// High priority compliance issue
    High,

    /// Medium priority compliance issue
    Medium,

    /// Low priority compliance issue
    Low,

    /// Informational finding
    Info,
}

impl ScanResult {
    /// Create a new scan result with basic metadata
    pub fn new(
        scan_id: String,
        ics_metadata: IcsMetadata,
        host: HostContext,
        user_context: UserContext,
    ) -> Self {
        let now = Utc::now();

        Self {
            scan_id,
            metadata: ScanMetadata {
                ics_metadata,
                host,
                user_context,
                timestamp: TimestampInfo {
                    scan_start: now,
                    scan_end: now,
                    duration_ms: 0,
                },
            },
            results: ComplianceResults {
                check: ComplianceCheck {
                    total_criteria: 0,
                    passed_criteria: 0,
                    failed_criteria: 0,
                    error_criteria: 0,
                    pass_percentage: 0.0,
                    status: ComplianceStatus::Error,
                },
                findings: Vec::new(),
                passed: false,
            },
        }
    }

    /// Mark scan as completed and calculate final metrics
    pub fn finalize(&mut self) {
        self.metadata.timestamp.scan_end = Utc::now();
        self.metadata.timestamp.duration_ms = (self.metadata.timestamp.scan_end
            - self.metadata.timestamp.scan_start)
            .num_milliseconds() as u64;

        let check = &mut self.results.check;

        // Calculate pass percentage
        if check.total_criteria > 0 {
            check.pass_percentage =
                (check.passed_criteria as f32 / check.total_criteria as f32) * 100.0;
        }

        // Determine overall status
        check.status = if check.error_criteria > 0 {
            ComplianceStatus::Partial
        } else if check.failed_criteria == 0 {
            ComplianceStatus::Compliant
        } else {
            ComplianceStatus::NonCompliant
        };

        // Set overall passed flag
        self.results.passed = matches!(check.status, ComplianceStatus::Compliant);
    }

    /// Add a compliance finding
    pub fn add_finding(&mut self, finding: ComplianceFinding) {
        // Update counters based on finding
        match finding.severity {
            FindingSeverity::Critical | FindingSeverity::High => {
                self.results.check.failed_criteria += 1;
            }
            _ => {
                // Medium/Low/Info findings don't fail the criterion
            }
        }

        self.results.findings.push(finding);
    }

    /// Update criteria counts manually
    pub fn update_criteria_counts(&mut self, total: u32, passed: u32, failed: u32, errors: u32) {
        let check = &mut self.results.check;
        check.total_criteria = total;
        check.passed_criteria = passed;
        check.failed_criteria = failed;
        check.error_criteria = errors;
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to compact JSON string
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Get scan duration in seconds
    pub fn duration_seconds(&self) -> f64 {
        self.metadata.timestamp.duration_ms as f64 / 1000.0
    }

    /// Check if scan was successful (no errors)
    pub fn is_successful(&self) -> bool {
        self.results.check.error_criteria == 0
    }

    /// Get findings by severity level
    pub fn findings_by_severity(&self, severity: FindingSeverity) -> Vec<&ComplianceFinding> {
        self.results
            .findings
            .iter()
            .filter(|f| std::mem::discriminant(&f.severity) == std::mem::discriminant(&severity))
            .collect()
    }
}

impl IcsMetadata {
    /// Create from parsed metadata block
    pub fn from_metadata_block(
        metadata: &crate::types::metadata::MetaDataBlock,
    ) -> Result<Self, String> {
        Ok(Self {
            ics_scan_id: metadata
                .fields
                .get("ics_scan_id")
                .ok_or("Missing ics_scan_id")?
                .clone(),
            control_framework: metadata
                .fields
                .get("control_framework")
                .ok_or("Missing control_framework")?
                .clone(),
            control: metadata
                .fields
                .get("control")
                .ok_or("Missing control")?
                .clone(),
            platform: metadata
                .fields
                .get("platform")
                .ok_or("Missing platform")?
                .clone(),
            criticality: metadata
                .fields
                .get("criticality")
                .ok_or("Missing criticality")?
                .clone(),
            tags: metadata.fields.get("tags").ok_or("Missing tags")?.clone(),
        })
    }

    /// Create with default values for testing
    pub fn default_test() -> Self {
        Self {
            ics_scan_id: "test-scan-001".to_string(),
            control_framework: "TEST".to_string(),
            control: "TEST-1".to_string(),
            platform: "Test".to_string(),
            criticality: "medium".to_string(),
            tags: "test".to_string(),
        }
    }
}

impl HostContext {
    /// Create host context from system information
    pub fn from_system() -> Self {
        Self {
            hostname: hostname::get()
                .unwrap_or_else(|_| std::ffi::OsString::from("unknown"))
                .to_string_lossy()
                .to_string(),
            os_info: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
            ip_address: None, // Would need additional dependency to get IP
            asset_id: None,
        }
    }

    /// Create with custom values
    pub fn new(hostname: String, os_info: String) -> Self {
        Self {
            hostname,
            os_info,
            ip_address: None,
            asset_id: None,
        }
    }

    /// Set IP address
    pub fn with_ip_address(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Set asset ID
    pub fn with_asset_id(mut self, asset_id: String) -> Self {
        self.asset_id = Some(asset_id);
        self
    }
}

impl UserContext {
    /// Create user context from environment
    pub fn from_environment() -> Self {
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".to_string());

        // Simple privilege detection based on username and environment
        let privilege_level = if username == "root" {
            "root".to_string()
        } else if cfg!(unix) {
            // Check if running as root by testing UID = 0
            match std::env::var("UID").or_else(|_| std::env::var("EUID")) {
                Ok(uid) if uid == "0" => "root".to_string(),
                _ => "user".to_string(),
            }
        } else {
            // For non-Unix systems, check common admin indicators
            if username.to_lowercase().contains("admin")
                || std::env::var("USERPROFILE")
                    .unwrap_or_default()
                    .contains("Administrator")
            {
                "admin".to_string()
            } else {
                "user".to_string()
            }
        };

        Self {
            username,
            privilege_level,
            process_info: Some(format!("pid:{}", std::process::id())),
        }
    }

    /// Create with custom values
    pub fn new(username: String, privilege_level: String) -> Self {
        Self {
            username,
            privilege_level,
            process_info: None,
        }
    }

    /// Set process information
    pub fn with_process_info(mut self, process_info: String) -> Self {
        self.process_info = Some(process_info);
        self
    }
}

impl ComplianceFinding {
    /// Create a new compliance finding
    pub fn new(
        finding_id: String,
        severity: FindingSeverity,
        title: String,
        description: String,
        expected: serde_json::Value,
        actual: serde_json::Value,
    ) -> Self {
        Self {
            finding_id,
            severity,
            title,
            description,
            expected,
            actual,
            remediation: None,
            field_path: None,
        }
    }

    /// Add remediation guidance
    pub fn with_remediation(mut self, remediation: String) -> Self {
        self.remediation = Some(remediation);
        self
    }

    /// Add field path context
    pub fn with_field_path(mut self, field_path: String) -> Self {
        self.field_path = Some(field_path);
        self
    }

    /// Create a finding with auto-generated ID
    pub fn auto_id(
        severity: FindingSeverity,
        title: String,
        description: String,
        expected: serde_json::Value,
        actual: serde_json::Value,
    ) -> Self {
        let finding_id = uuid::Uuid::new_v4().to_string();
        Self::new(finding_id, severity, title, description, expected, actual)
    }
}

impl Default for HostContext {
    fn default() -> Self {
        Self::from_system()
    }
}

impl Default for UserContext {
    fn default() -> Self {
        Self::from_environment()
    }
}

impl From<&crate::types::metadata::MetaDataBlock> for IcsMetadata {
    fn from(metadata: &crate::types::metadata::MetaDataBlock) -> Self {
        Self {
            ics_scan_id: metadata
                .fields
                .get("ics_scan_id")
                .expect("ics_scan_id should be validated before conversion")
                .clone(),
            control_framework: metadata
                .fields
                .get("control_framework")
                .expect("control_framework should be validated before conversion")
                .clone(),
            control: metadata
                .fields
                .get("control")
                .expect("control should be validated before conversion")
                .clone(),
            platform: metadata
                .fields
                .get("platform")
                .expect("platform should be validated before conversion")
                .clone(),
            criticality: metadata
                .fields
                .get("criticality")
                .expect("criticality should be validated before conversion")
                .clone(),
            tags: metadata
                .fields
                .get("tags")
                .expect("tags should be validated before conversion")
                .clone(),
        }
    }
}
