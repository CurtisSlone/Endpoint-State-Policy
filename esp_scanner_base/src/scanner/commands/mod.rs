//! Command execution configurations for different platforms

pub mod rhel9;

pub use rhel9::create_rhel9_command_executor;
