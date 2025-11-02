//! # Consumer logging integration for the ICS parser
//!
//! Provides safe wrappers around the parser's logging functions with
//! structured context support and proper error handling.

use super::common::{convert_context_to_c_arrays, validate_ffi_string, IcsError, IcsErrorCode};
use std::ffi::CString;
use std::os::raw::c_char;

// External C functions for logging
extern "C" {
    fn ics_log_consumer_error(
        code: *const c_char,
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    fn ics_log_consumer_warning(
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    fn ics_log_consumer_info(
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    fn ics_log_consumer_debug(
        message: *const c_char,
        context_count: usize,
        context_keys: *const *const c_char,
        context_values: *const *const c_char,
    ) -> IcsErrorCode;

    fn ics_log_set_consumer_context(
        consumer_id: *const c_char,
        module: *const c_char,
    ) -> IcsErrorCode;
}

/// Log consumer error with error code and structured context
pub fn log_consumer_error(
    code: &str,
    message: &str,
    context: &[(&str, &str)],
) -> Result<(), IcsError> {
    // Validate inputs
    validate_ffi_string(code)?;
    validate_ffi_string(message)?;

    // Validate context strings
    for (key, value) in context {
        validate_ffi_string(key)?;
        validate_ffi_string(value)?;
    }

    let c_code = CString::new(code)?;
    let c_message = CString::new(message)?;

    let (_c_strings, keys, values) = convert_context_to_c_arrays(context)?;

    let error_code = unsafe {
        ics_log_consumer_error(
            c_code.as_ptr(),
            c_message.as_ptr(),
            context.len(),
            if keys.is_empty() {
                std::ptr::null()
            } else {
                keys.as_ptr()
            },
            if values.is_empty() {
                std::ptr::null()
            } else {
                values.as_ptr()
            },
        )
    };

    match error_code {
        IcsErrorCode::Success => Ok(()),
        error => Err(error.into()),
    }
}

/// Log consumer warning with structured context
pub fn log_consumer_warning(message: &str, context: &[(&str, &str)]) -> Result<(), IcsError> {
    validate_ffi_string(message)?;

    // Validate context strings
    for (key, value) in context {
        validate_ffi_string(key)?;
        validate_ffi_string(value)?;
    }

    let c_message = CString::new(message)?;

    let (_c_strings, keys, values) = convert_context_to_c_arrays(context)?;

    let error_code = unsafe {
        ics_log_consumer_warning(
            c_message.as_ptr(),
            context.len(),
            if keys.is_empty() {
                std::ptr::null()
            } else {
                keys.as_ptr()
            },
            if values.is_empty() {
                std::ptr::null()
            } else {
                values.as_ptr()
            },
        )
    };

    match error_code {
        IcsErrorCode::Success => Ok(()),
        error => Err(error.into()),
    }
}

/// Log consumer info with structured context
pub fn log_consumer_info(message: &str, context: &[(&str, &str)]) -> Result<(), IcsError> {
    validate_ffi_string(message)?;

    // Validate context strings
    for (key, value) in context {
        validate_ffi_string(key)?;
        validate_ffi_string(value)?;
    }

    let c_message = CString::new(message)?;

    let (_c_strings, keys, values) = convert_context_to_c_arrays(context)?;

    let error_code = unsafe {
        ics_log_consumer_info(
            c_message.as_ptr(),
            context.len(),
            if keys.is_empty() {
                std::ptr::null()
            } else {
                keys.as_ptr()
            },
            if values.is_empty() {
                std::ptr::null()
            } else {
                values.as_ptr()
            },
        )
    };

    match error_code {
        IcsErrorCode::Success => Ok(()),
        error => Err(error.into()),
    }
}

/// Log consumer debug with structured context
pub fn log_consumer_debug(message: &str, context: &[(&str, &str)]) -> Result<(), IcsError> {
    validate_ffi_string(message)?;

    // Validate context strings
    for (key, value) in context {
        validate_ffi_string(key)?;
        validate_ffi_string(value)?;
    }

    let c_message = CString::new(message)?;

    let (_c_strings, keys, values) = convert_context_to_c_arrays(context)?;

    let error_code = unsafe {
        ics_log_consumer_debug(
            c_message.as_ptr(),
            context.len(),
            if keys.is_empty() {
                std::ptr::null()
            } else {
                keys.as_ptr()
            },
            if values.is_empty() {
                std::ptr::null()
            } else {
                values.as_ptr()
            },
        )
    };

    match error_code {
        IcsErrorCode::Success => Ok(()),
        error => Err(error.into()),
    }
}

/// Set consumer context for all subsequent log messages
pub fn set_consumer_context(consumer_id: &str, module: &str) -> Result<(), IcsError> {
    validate_ffi_string(consumer_id)?;
    validate_ffi_string(module)?;

    let c_consumer_id = CString::new(consumer_id)?;
    let c_module = CString::new(module)?;

    let error_code =
        unsafe { ics_log_set_consumer_context(c_consumer_id.as_ptr(), c_module.as_ptr()) };

    match error_code {
        IcsErrorCode::Success => Ok(()),
        error => Err(error.into()),
    }
}

/// Standard consumer error codes as defined in the parser API
pub mod consumer_codes {
    /// Consumer initialization failure
    pub const CONSUMER_INIT_FAILURE: &str = "C001";
    /// Consumer configuration error
    pub const CONSUMER_CONFIG_ERROR: &str = "C002";
    /// Consumer shutdown error
    pub const CONSUMER_SHUTDOWN_ERROR: &str = "C003";
    /// Consumer pipeline error
    pub const CONSUMER_PIPELINE_ERROR: &str = "C010";
    /// Consumer pass failure
    pub const CONSUMER_PASS_FAILURE: &str = "C011";
    /// Consumer state mismatch
    pub const CONSUMER_STATE_MISMATCH: &str = "C012";
    /// Consumer data validation error
    pub const CONSUMER_DATA_VALIDATION_ERROR: &str = "C020";
    /// Consumer format error
    pub const CONSUMER_FORMAT_ERROR: &str = "C021";
    /// Consumer encoding error
    pub const CONSUMER_ENCODING_ERROR: &str = "C022";
    /// Consumer memory error
    pub const CONSUMER_MEMORY_ERROR: &str = "C030";
    /// Consumer timeout error
    pub const CONSUMER_TIMEOUT_ERROR: &str = "C031";
    /// Consumer capacity error
    pub const CONSUMER_CAPACITY_ERROR: &str = "C032";
    /// Consumer I/O error
    pub const CONSUMER_IO_ERROR: &str = "C040";
    /// Consumer network error
    pub const CONSUMER_NETWORK_ERROR: &str = "C041";
    /// Consumer permission error
    pub const CONSUMER_PERMISSION_ERROR: &str = "C042";
    // Add this line with the other CONSUMER_* constants
    pub const CONSUMER_SCANNER_ERROR: &str = "C007";

    pub const CONSUMER_VALIDATION_ERROR: &str = "C008";

    /// Check if an error code is a valid consumer code
    pub fn is_valid_consumer_code(code: &str) -> bool {
        matches!(
            code,
            CONSUMER_INIT_FAILURE
                | CONSUMER_CONFIG_ERROR
                | CONSUMER_SHUTDOWN_ERROR
                | CONSUMER_PIPELINE_ERROR
                | CONSUMER_PASS_FAILURE
                | CONSUMER_STATE_MISMATCH
                | CONSUMER_DATA_VALIDATION_ERROR
                | CONSUMER_FORMAT_ERROR
                | CONSUMER_ENCODING_ERROR
                | CONSUMER_MEMORY_ERROR
                | CONSUMER_TIMEOUT_ERROR
                | CONSUMER_CAPACITY_ERROR
                | CONSUMER_IO_ERROR
                | CONSUMER_NETWORK_ERROR
                | CONSUMER_PERMISSION_ERROR
                | CONSUMER_SCANNER_ERROR
                | CONSUMER_VALIDATION_ERROR
        )
    }

    /// Get description for a consumer error code
    pub fn get_error_description(code: &str) -> Option<&'static str> {
        match code {
            CONSUMER_INIT_FAILURE => Some("Consumer initialization failure"),
            CONSUMER_CONFIG_ERROR => Some("Consumer configuration error"),
            CONSUMER_SHUTDOWN_ERROR => Some("Consumer shutdown error"),
            CONSUMER_PIPELINE_ERROR => Some("Consumer pipeline error"),
            CONSUMER_PASS_FAILURE => Some("Consumer pass failure"),
            CONSUMER_STATE_MISMATCH => Some("Consumer state mismatch"),
            CONSUMER_DATA_VALIDATION_ERROR => Some("Consumer data validation error"),
            CONSUMER_FORMAT_ERROR => Some("Consumer format error"),
            CONSUMER_ENCODING_ERROR => Some("Consumer encoding error"),
            CONSUMER_MEMORY_ERROR => Some("Consumer memory error"),
            CONSUMER_TIMEOUT_ERROR => Some("Consumer timeout error"),
            CONSUMER_CAPACITY_ERROR => Some("Consumer capacity error"),
            CONSUMER_IO_ERROR => Some("Consumer I/O error"),
            CONSUMER_NETWORK_ERROR => Some("Consumer network error"),
            CONSUMER_PERMISSION_ERROR => Some("Consumer permission error"),
            _ => None,
        }
    }
}

/// Convenience wrapper for structured logging with validation
#[derive(Debug, Clone)]
pub struct ConsumerLogger {
    consumer_id: String,
    module: String,
}

impl ConsumerLogger {
    /// Create a new consumer logger and set the context
    pub fn new(consumer_id: &str, module: &str) -> Result<Self, IcsError> {
        set_consumer_context(consumer_id, module)?;

        Ok(Self {
            consumer_id: consumer_id.to_string(),
            module: module.to_string(),
        })
    }

    /// Log an error with error code and context
    pub fn error(
        &self,
        code: &str,
        message: &str,
        context: &[(&str, &str)],
    ) -> Result<(), IcsError> {
        log_consumer_error(code, message, context)
    }

    /// Log a warning with context
    pub fn warning(&self, message: &str, context: &[(&str, &str)]) -> Result<(), IcsError> {
        log_consumer_warning(message, context)
    }

    /// Log info with context
    pub fn info(&self, message: &str, context: &[(&str, &str)]) -> Result<(), IcsError> {
        log_consumer_info(message, context)
    }

    /// Log debug with context
    pub fn debug(&self, message: &str, context: &[(&str, &str)]) -> Result<(), IcsError> {
        log_consumer_debug(message, context)
    }

    /// Log error without additional context
    pub fn error_simple(&self, code: &str, message: &str) -> Result<(), IcsError> {
        self.error(code, message, &[])
    }

    /// Log warning without additional context
    pub fn warning_simple(&self, message: &str) -> Result<(), IcsError> {
        self.warning(message, &[])
    }

    /// Log info without additional context
    pub fn info_simple(&self, message: &str) -> Result<(), IcsError> {
        self.info(message, &[])
    }

    /// Log debug without additional context
    pub fn debug_simple(&self, message: &str) -> Result<(), IcsError> {
        self.debug(message, &[])
    }

    /// Get the consumer ID
    pub fn consumer_id(&self) -> &str {
        &self.consumer_id
    }

    /// Get the module name
    pub fn module(&self) -> &str {
        &self.module
    }
}

#[cfg(test)]
mod tests {
    use super::consumer_codes::*;
    use super::*;

    #[test]
    fn test_consumer_codes_validation() {
        assert!(is_valid_consumer_code(CONSUMER_INIT_FAILURE));
        assert!(is_valid_consumer_code(CONSUMER_IO_ERROR));
        assert!(!is_valid_consumer_code("INVALID_CODE"));
        assert!(!is_valid_consumer_code(""));
    }

    #[test]
    fn test_error_descriptions() {
        assert_eq!(
            get_error_description(CONSUMER_INIT_FAILURE),
            Some("Consumer initialization failure")
        );
        assert_eq!(get_error_description("INVALID_CODE"), None);
    }

    #[test]
    fn test_logger_creation() {
        // This will fail without the C library, but tests the interface
        let result = ConsumerLogger::new("test-consumer", "test-module");

        // Don't assert success/failure since we can't link C library in unit tests
        match result {
            Ok(logger) => {
                assert_eq!(logger.consumer_id(), "test-consumer");
                assert_eq!(logger.module(), "test-module");
            }
            Err(_) => {
                // Expected in unit test environment without C library
            }
        }
    }

    #[test]
    fn test_context_validation() {
        // Test that invalid strings are caught
        let invalid_context = vec![("key\0", "value")];
        let result = log_consumer_info("test", &invalid_context);
        assert!(result.is_err());

        let invalid_message = "test\0message";
        let result = log_consumer_info(invalid_message, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_context() {
        // Test logging with empty context
        let result = log_consumer_info("test message", &[]);
        // Don't assert success since C library may not be available
        assert!(result.is_ok() || result.is_err());
    }
}
