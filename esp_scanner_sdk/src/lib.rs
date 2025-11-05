//! # ESP Scanner SDK
//!
//! Extended scanner strategies for ESP compliance validation.
//! Provides file system, RPM, systemd, sysctl, SELinux, and JSON validation.

// Module declarations
pub mod collectors;
pub mod commands;
pub mod contracts;
pub mod executors;

// REMOVED: pub use create_scanner_registry; (this was the duplicate)

use esp_scanner_base::strategies::{CtnStrategyRegistry, StrategyError};

/// Create a registry with all available strategies
///
/// Includes:
/// - File metadata validation (fast stat-based checks)
/// - File content validation (string operations)
/// - JSON record validation (structured data)
/// - RPM package validation (installation and version checks)
/// - Systemd service validation (active, enabled, loaded status)
/// - Sysctl parameter validation (kernel parameters)
/// - SELinux status validation (enforcement mode)
pub fn create_scanner_registry() -> Result<CtnStrategyRegistry, StrategyError> {
    let mut registry = CtnStrategyRegistry::new();

    // Register file system strategies
    let metadata_contract = contracts::create_file_metadata_contract();
    let content_contract = contracts::create_file_content_contract();
    let json_contract = contracts::create_json_record_contract();
    let computed_values_contract = contracts::create_computed_values_contract();

    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::FileMetadataExecutor::new(metadata_contract)),
    )?;

    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::FileContentExecutor::new(content_contract)),
    )?;

    registry.register_ctn_strategy(
        Box::new(collectors::ComputedValuesCollector::new()),
        Box::new(executors::ComputedValuesExecutor::new(
            computed_values_contract,
        )),
    )?;

    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::JsonRecordExecutor::new(json_contract)),
    )?;

    // Create ONE command executor with full RHEL 9 whitelist
    let command_executor = commands::create_rhel9_command_executor();
    let command_collector =
        collectors::CommandCollector::new("rhel9-command-collector", command_executor);

    // Register command-based strategies
    let rpm_contract = contracts::create_rpm_package_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(executors::RpmPackageExecutor::new(rpm_contract)),
    )?;

    let systemd_contract = contracts::create_systemd_service_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(executors::SystemdServiceExecutor::new(systemd_contract)),
    )?;

    let sysctl_contract = contracts::create_sysctl_parameter_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(executors::SysctlParameterExecutor::new(sysctl_contract)),
    )?;

    let selinux_contract = contracts::create_selinux_status_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector),
        Box::new(executors::SelinuxStatusExecutor::new(selinux_contract)),
    )?;

    Ok(registry)
}
