//! Raw FFI bindings for the simplified ICS parser library
//!
//! This module contains the external function declarations for the
//! JSON-based ICS parser library located in libs/

use std::os::raw::{c_char, c_int};

/// Error codes returned by the C library
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcsErrorCode {
    Success = 0,
    FileNotFound = 1,
    ParseFailed = 2,
    ValidationFailed = 3,
    InternalError = 4,
    InvalidPath = 5,
    NullPointer = 6,
}

/// Batch processing configuration (must match library exactly)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IcsBatchConfig {
    pub max_threads: c_int,
    pub recursive: c_int, // 0 = false, 1 = true
    pub max_files: c_int, // -1 = no limit
    pub fail_fast: c_int, // 0 = false, 1 = true
}

// Link to the dynamic library in libs/
#[link(name = "ics_lib")]
extern "C" {
    // ========================================================================
    // Library Initialization and Version
    // ========================================================================

    /// Initialize the ICS parser library
    pub fn ics_init() -> IcsErrorCode;

    /// Get library version string (points to static memory)
    pub fn ics_parser_version() -> *const c_char;

    // ========================================================================
    // Primary JSON-Based API
    // ========================================================================

    /// Parse a single file and return JSON result
    /// Returns: JSON string that must be freed with ics_free_string()
    pub fn ics_parse_file_json(file_path: *const c_char) -> *mut c_char;

    /// Parse a directory and return JSON batch results
    /// Returns: JSON string that must be freed with ics_free_string()
    pub fn ics_parse_directory_json(dir_path: *const c_char, config: IcsBatchConfig)
        -> *mut c_char;

    /// Create default batch configuration
    pub fn ics_create_default_batch_config() -> IcsBatchConfig;

    // ========================================================================
    // Memory Management
    // ========================================================================

    /// Free strings returned by the library
    pub fn ics_free_string(s: *mut c_char);

    // ========================================================================
    // Logging Functions
    // ========================================================================

    /// Set consumer context for logging
    pub fn ics_log_set_consumer_context(
        consumer_id: *const c_char,
        module: *const c_char,
    ) -> IcsErrorCode;

    /// Log consumer error with context
    pub fn ics_log_consumer_error(
        code: *const c_char,
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    /// Log consumer warning with context
    pub fn ics_log_consumer_warning(
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    /// Log consumer info with context
    pub fn ics_log_consumer_info(
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    /// Log consumer debug with context
    pub fn ics_log_consumer_debug(
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;
}
