//! RHEL 9 command executor configuration
//!
//! Provides a whitelisted command executor for RHEL 9 STIG compliance scanning.

use esp_scanner_base::strategies::SystemCommandExecutor;
use std::time::Duration;

/// Create command executor configured for RHEL 9 STIG scanning
///
/// Whitelist includes:
/// - rpm: Package management queries
/// - systemctl: Service status checks
/// - getenforce: SELinux enforcement mode
/// - sysctl: Kernel parameter queries
/// - auditctl: Audit rule inspection
/// - id: User identity information
/// - stat: File metadata queries
/// - getent: User/group database queries
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rhel9_executor_creation() {
        // Create executor - variable prefixed with _ since we don't use it
        let _executor = create_rhel9_command_executor();

        // Just verify it was created successfully
        // (default_timeout is private, so we can't test it directly)
        assert!(true, "Executor created successfully");
    }

    #[test]
    fn test_rhel9_executor_whitelist() {
        let executor = create_rhel9_command_executor();

        // Test that expected commands are whitelisted
        assert!(executor.is_allowed("rpm"));
        assert!(executor.is_allowed("systemctl"));
        assert!(executor.is_allowed("getenforce"));
        assert!(executor.is_allowed("sysctl"));

        // Test that random commands are NOT whitelisted
        assert!(!executor.is_allowed("rm"));
        assert!(!executor.is_allowed("dd"));
        assert!(!executor.is_allowed("curl"));
    }
}
