//! Configuration module for ESP Compiler
//! Automatically uses generated constants from TOML configuration

// Include generated constants from build.rs
// This file is generated at compile time from your TOML configuration
include!(concat!(env!("OUT_DIR"), "/constants.rs"));

// Keep original constants file for reference and runtime configuration
pub mod constants;
pub mod runtime;

// Runtime configuration types and loader

/// Build information and configuration metadata
pub mod build_info {
    /// Returns the configuration profile used during build
    pub fn profile() -> &'static str {
        option_env!("ESP_BUILD_PROFILE").unwrap_or("development")
    }

    /// Returns the configuration directory used during build
    pub fn config_dir() -> &'static str {
        option_env!("ESP_CONFIG_DIR").unwrap_or("config")
    }

    /// Returns configuration source information
    pub fn source_info() -> String {
        format!("Generated from {}/{}.toml", config_dir(), profile())
    }

    /// Returns whether constants were successfully generated
    pub fn constants_generated() -> bool {
        // This will be true if the include! worked
        true
    }

    /// Returns the OUT_DIR path used for generation (for debugging)
    pub fn out_dir() -> &'static str {
        env!("OUT_DIR")
    }
}
