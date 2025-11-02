//! Common error types and utility functions for FFI operations

use std::ffi::{CStr, CString, NulError};
use std::os::raw::c_char;
// Removed unused HashMap import

// Re-export IcsErrorCode from bindings to avoid duplication
pub use super::bindings::IcsErrorCode;

/// Comprehensive error type for all FFI operations
#[derive(Debug, thiserror::Error)]
pub enum IcsError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Parse failed: {message}")]
    ParseFailed { message: String },

    #[error("Validation failed: {message}")]
    ValidationFailed { message: String },

    #[error("Internal library error")]
    InternalError,

    #[error("Invalid file path: {path}")]
    InvalidPath { path: String },

    #[error("Null pointer error")]
    NullPointer,

    #[error("Library not initialized")]
    NotInitialized,

    #[error("String conversion error: {0}")]
    StringConversion(#[from] NulError),

    #[error("JSON deserialization error: {message}\nJSON snippet: {json_snippet}")]
    JsonDeserialization {
        message: String,
        json_snippet: String,
    },

    #[error("FFI boundary error: {context}")]
    FfiBoundary { context: String },

    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Unsupported operation: {operation}")]
    UnsupportedOperation { operation: String },

    #[error("Version compatibility error: {message}")]
    VersionCompatibility { message: String },

    #[error("Memory allocation error: {context}")]
    MemoryAllocation { context: String },
}

impl From<IcsErrorCode> for IcsError {
    fn from(code: IcsErrorCode) -> Self {
        match code {
            IcsErrorCode::Success => unreachable!("Success should not be converted to error"),
            IcsErrorCode::FileNotFound => IcsError::FileNotFound {
                path: "unknown".to_string(),
            },
            IcsErrorCode::ParseFailed => IcsError::ParseFailed {
                message: "Parse operation failed".to_string(),
            },
            IcsErrorCode::ValidationFailed => IcsError::ValidationFailed {
                message: "Validation failed".to_string(),
            },
            IcsErrorCode::InternalError => IcsError::InternalError,
            IcsErrorCode::InvalidPath => IcsError::InvalidPath {
                path: "unknown".to_string(),
            },
            IcsErrorCode::NullPointer => IcsError::NullPointer,
        }
    }
}

impl From<serde_json::Error> for IcsError {
    fn from(error: serde_json::Error) -> Self {
        let json_snippet = if error.line() != 0 {
            format!("line {}, column {}", error.line(), error.column())
        } else {
            "location unknown".to_string()
        };

        IcsError::JsonDeserialization {
            message: error.to_string(),
            json_snippet,
        }
    }
}

impl IcsError {
    /// Create a JSON deserialization error with actual JSON snippet
    pub fn json_error_with_snippet(error: serde_json::Error, json: &str) -> Self {
        Self::JsonDeserialization {
            message: error.to_string(),
            json_snippet: truncate_json_for_error(json, 200),
        }
    }

    /// Create a configuration error with context
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a version compatibility error
    pub fn version_error(message: impl Into<String>) -> Self {
        Self::VersionCompatibility {
            message: message.into(),
        }
    }

    /// Create an FFI boundary error with context
    pub fn ffi_error(context: impl Into<String>) -> Self {
        Self::FfiBoundary {
            context: context.into(),
        }
    }

    /// Check if this error indicates a recoverable condition
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            IcsError::FileNotFound { .. }
                | IcsError::InvalidPath { .. }
                | IcsError::ParseFailed { .. }
                | IcsError::ValidationFailed { .. }
        )
    }

    /// Check if this error indicates a library/system issue
    pub fn is_system_error(&self) -> bool {
        matches!(
            self,
            IcsError::InternalError
                | IcsError::NullPointer
                | IcsError::NotInitialized
                | IcsError::FfiBoundary { .. }
                | IcsError::MemoryAllocation { .. }
                | IcsError::VersionCompatibility { .. }
        )
    }
}

/// Convert context key-value pairs to C arrays for logging functions
pub fn convert_context_to_c_arrays(
    context: &[(&str, &str)],
) -> Result<(Vec<CString>, Vec<*const c_char>, Vec<*const c_char>), NulError> {
    let mut c_strings = Vec::new();
    let mut keys = Vec::new();
    let mut values = Vec::new();

    for (key, value) in context {
        let c_key = CString::new(*key)?;
        let c_value = CString::new(*value)?;

        keys.push(c_key.as_ptr());
        values.push(c_value.as_ptr());

        c_strings.push(c_key);
        c_strings.push(c_value);
    }

    Ok((c_strings, keys, values))
}

/// Safely convert C string to Rust string
pub unsafe fn convert_c_string_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return "unknown".to_string();
    }

    match CStr::from_ptr(ptr).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => CStr::from_ptr(ptr).to_string_lossy().to_string(),
    }
}

/// Validate that a string is suitable for FFI (no null bytes)
pub fn validate_ffi_string(s: &str) -> Result<(), IcsError> {
    if s.contains('\0') {
        Err(IcsError::StringConversion(CString::new(s).unwrap_err()))
    } else {
        Ok(())
    }
}

/// Validate JSON string format before deserialization
pub fn validate_json_format(json: &str) -> Result<(), IcsError> {
    if json.is_empty() {
        return Err(IcsError::JsonDeserialization {
            message: "Empty JSON string".to_string(),
            json_snippet: "empty".to_string(),
        });
    }

    let trimmed = json.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        return Err(IcsError::JsonDeserialization {
            message: "JSON must start with '{' or '['".to_string(),
            json_snippet: truncate_json_for_error(trimmed, 50),
        });
    }

    Ok(())
}

/// Truncate JSON string for error messages
pub fn truncate_json_for_error(json: &str, max_len: usize) -> String {
    if json.len() <= max_len {
        json.to_string()
    } else {
        let truncated = &json[..max_len];
        if let Some(pos) = truncated.rfind(',') {
            format!("{}...", &json[..pos])
        } else if let Some(pos) = truncated.rfind('{') {
            format!("{}...", &json[..pos])
        } else {
            format!("{}...", truncated)
        }
    }
}

/// Enhanced result type alias for FFI operations
pub type FfiResult<T> = Result<T, IcsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        let error: IcsError = IcsErrorCode::FileNotFound.into();
        assert!(matches!(error, IcsError::FileNotFound { .. }));

        let error: IcsError = IcsErrorCode::InternalError.into();
        assert!(matches!(error, IcsError::InternalError));
    }

    #[test]
    fn test_error_categorization() {
        let recoverable = IcsError::FileNotFound {
            path: "test.ics".to_string(),
        };
        assert!(recoverable.is_recoverable());
        assert!(!recoverable.is_system_error());

        let system_error = IcsError::InternalError;
        assert!(!system_error.is_recoverable());
        assert!(system_error.is_system_error());
    }

    #[test]
    fn test_string_validation() {
        assert!(validate_ffi_string("valid_string").is_ok());
        assert!(validate_ffi_string("string\0with_null").is_err());
        assert!(validate_ffi_string("").is_ok());
    }

    #[test]
    fn test_json_validation() {
        assert!(validate_json_format(r#"{"valid": "json"}"#).is_ok());
        assert!(validate_json_format(r#"["valid", "array"]"#).is_ok());
        assert!(validate_json_format("").is_err());
        assert!(validate_json_format("invalid json").is_err());
    }

    #[test]
    fn test_json_truncation() {
        let short_json = r#"{"key": "value"}"#;
        assert_eq!(truncate_json_for_error(short_json, 50), short_json);

        let long_json = r#"{"very": "long", "json": "object", "with": "many", "fields": "here"}"#;
        let truncated = truncate_json_for_error(long_json, 30);
        assert!(truncated.len() <= 33); // 30 + "..."
        assert!(truncated.ends_with("..."));
    }
}
