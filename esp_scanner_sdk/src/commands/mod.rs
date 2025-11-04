//! Command execution configurations for different platforms
//!
//! Provides whitelisted command executors for secure system scanning.

pub mod rhel9;

pub use rhel9::create_rhel9_command_executor;
