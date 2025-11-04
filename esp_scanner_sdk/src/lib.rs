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

    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::FileMetadataExecutor::new(metadata_contract)),
    )?;

    registry.register_ctn_strategy(
        Box::new(collectors::FileSystemCollector::new()),
        Box::new(executors::FileContentExecutor::new(content_contract)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = create_scanner_registry().expect("Failed to create registry");

        // Get registry statistics
        let stats = registry.get_statistics();

        // Debug: Print what we got
        eprintln!("Registry Statistics:");
        eprintln!("  Total CTN types: {}", stats.total_ctn_types);
        eprintln!("  Total collectors: {}", stats.total_collectors);
        eprintln!("  Total executors: {}", stats.total_executors);
        eprintln!("  Registry health: {:?}", stats.registry_health);
        eprintln!("  Is healthy: {}", stats.registry_health.is_healthy());

        // Test 1: Verify we have 7 CTN types registered
        assert_eq!(
            stats.total_ctn_types, 7,
            "Expected 7 CTN types to be registered"
        );

        // Test 2: Check if registry is healthy (but don't fail yet)
        if !stats.registry_health.is_healthy() {
            eprintln!("\n⚠️  Registry is not healthy! Checking validation...");

            // Validate all contracts to see what's wrong
            let validation_results = registry.validate_all_contracts();
            for result in validation_results {
                if !result.is_valid {
                    eprintln!("\n❌ Contract validation failed for: {}", result.ctn_type);
                    for error in &result.errors {
                        eprintln!("   Error: {}", error);
                    }
                    for warning in &result.warnings {
                        eprintln!("   Warning: {}", warning);
                    }
                }
            }
        }

        // Now assert (will show the debug info above if it fails)
        assert!(
            stats.registry_health.is_healthy(),
            "Registry should be healthy. Health status: {:?}",
            stats.registry_health
        );

        // Test 3: Verify all expected CTN types can be retrieved
        let expected_types = vec![
            "file_metadata",
            "file_content",
            "json_record",
            "rpm_package",
            "systemd_service",
            "sysctl_parameter",
            "selinux_status",
        ];

        for ctn_type in expected_types {
            assert!(
                registry.get_ctn_contract(ctn_type).is_ok(),
                "Failed to get contract for CTN type: {}",
                ctn_type
            );
        }
    }
}
