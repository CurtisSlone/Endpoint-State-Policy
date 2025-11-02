//! RHEL 9 command executor configuration

use crate::strategies::SystemCommandExecutor;
use std::time::Duration;

/// Create command executor configured for RHEL 9 STIG scanning
pub fn create_rhel9_command_executor() -> SystemCommandExecutor {
    let mut executor = SystemCommandExecutor::with_timeout(Duration::from_secs(5));

    executor.allow_commands(&[
        "rpm",        // Package management
        "systemctl",  // Service status
        "getenforce", // SELinux status
        "auditctl",   // Audit rules
        "sysctl",     // Kernel parameters
        "id",         // User info
        "stat",       // File metadata
        "getent",     // User/group database
    ]);

    executor
}
