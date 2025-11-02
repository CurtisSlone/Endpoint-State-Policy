//! # Scanner Module
//!
//! Complete scanner implementation with file system and command execution support.

pub mod collectors;
pub mod commands;
pub mod contracts;
pub mod executors;

// Re-export main components
pub use collectors::command::CommandCollector;
pub use collectors::filesystem::FileSystemCollector;
pub use commands::create_rhel9_command_executor;
pub use contracts::file_contracts::{create_file_content_contract, create_file_metadata_contract};
pub use contracts::rpm_contracts::create_rpm_package_contract;
pub use contracts::selinux_contracts::create_selinux_status_contract;
pub use contracts::sysctl_contracts::create_sysctl_parameter_contract;
pub use contracts::systemd_contracts::create_systemd_service_contract;
use crate::scanner::contracts::create_json_record_contract;
pub use executors::file_content::FileContentExecutor;
pub use executors::file_metadata::FileMetadataExecutor;
pub use executors::rpm_package::RpmPackageExecutor;
pub use executors::selinux_status::SelinuxStatusExecutor;
pub use executors::sysctl_parameter::SysctlParameterExecutor;
pub use executors::systemd_service::SystemdServiceExecutor;
 use crate::scanner::executors::JsonRecordExecutor;

use crate::api::*;

/// Create a registry with all available strategies
/// Includes file system + command execution (RHEL 9)
pub fn create_scanner_registry() -> Result<CtnStrategyRegistry, StrategyError> {
    let mut registry = CtnStrategyRegistry::new();

    // Register file system strategies
    let metadata_contract = create_file_metadata_contract();
    let content_contract = create_file_content_contract();

    registry.register_ctn_strategy(
        Box::new(FileSystemCollector::new()),
        Box::new(FileMetadataExecutor::new(metadata_contract)),
    )?;

    registry.register_ctn_strategy(
        Box::new(FileSystemCollector::new()),
        Box::new(FileContentExecutor::new(content_contract)),
    )?;

    // Create ONE command executor with full RHEL 9 whitelist
    let command_executor = create_rhel9_command_executor();
    let command_collector = CommandCollector::new("rhel9-command-collector", command_executor);

    // Register rpm_package strategy
    let rpm_contract = create_rpm_package_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(RpmPackageExecutor::new(rpm_contract)),
    )?;

    // Register systemd_service strategy
    let systemd_contract = create_systemd_service_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(SystemdServiceExecutor::new(systemd_contract)),
    )?;

    // Register sysctl_parameter strategy
    let sysctl_contract = create_sysctl_parameter_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(SysctlParameterExecutor::new(sysctl_contract)),
    )?;

    // Register selinux_status strategy
    let selinux_contract = create_selinux_status_contract();
    registry.register_ctn_strategy(
        Box::new(command_collector.clone()),
        Box::new(SelinuxStatusExecutor::new(selinux_contract)),
    )?;

    // Register JSON record strategy (NEW)
    let json_contract = create_json_record_contract();
    registry.register_ctn_strategy(
        Box::new(FileSystemCollector::new()),
        Box::new(JsonRecordExecutor::new(json_contract)),
    )?;

    Ok(registry)
}

/// Alias for backward compatibility
pub fn create_file_scanner_registry() -> Result<CtnStrategyRegistry, StrategyError> {
    create_scanner_registry()
}