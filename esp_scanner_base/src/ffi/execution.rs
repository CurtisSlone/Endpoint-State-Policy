//! # Execution data retrieval and deserialization
//!
//! Provides safe wrappers around the parser's execution data functions with
//! automatic JSON deserialization into typed Rust structures and enhanced
//! error handling with context preservation.

use super::bindings::{
    ics_get_execution_data, ics_get_resolved_variables, ics_get_execution_metadata,
    ics_free_string, IcsParseResult
};
use super::common::{
    IcsErrorCode, IcsError, convert_c_string_to_string, 
    validate_json_format
};
use super::types::{ExecutionData, ExecutionMetadata};
use std::collections::HashMap;
use std::os::raw::c_char;

/// Get complete execution data with automatic JSON deserialization
pub fn get_execution_data_parsed(
    result: *const IcsParseResult
) -> Result<ExecutionData, IcsError> {
    let json_string = get_execution_data_json(result)?;
    
    // Validate JSON format before deserialization
    validate_json_format(&json_string)?;
    
    serde_json::from_str(&json_string)
        .map_err(|e| IcsError::json_error_with_snippet(e, &json_string))
}

/// Get execution data as raw JSON string
pub fn get_execution_data_json(
    result: *const IcsParseResult
) -> Result<String, IcsError> {
    if result.is_null() {
        return Err(IcsError::NullPointer);
    }
    
    let mut json_ptr: *mut c_char = std::ptr::null_mut();
    
    let error_code = unsafe {
        ics_get_execution_data(result, &mut json_ptr)
    };
    
    match error_code {
        IcsErrorCode::Success => {
            if json_ptr.is_null() {
                Err(IcsError::FfiBoundary { 
                    context: "Parser returned success but null JSON pointer for execution data".to_string()
                })
            } else {
                let json_string = unsafe {
                    let result = convert_c_string_to_string(json_ptr);
                    ics_free_string(json_ptr);
                    result
                };
                
                if json_string.is_empty() {
                    Err(IcsError::JsonDeserialization {
                        message: "Parser returned empty execution data JSON".to_string(),
                        json_snippet: "empty".to_string(),
                    })
                } else {
                    Ok(json_string)
                }
            }
        }
        IcsErrorCode::InternalError => Err(IcsError::InternalError),
        IcsErrorCode::NullPointer => Err(IcsError::NullPointer),
        error => Err(IcsError::FfiBoundary { 
            context: format!("Unexpected error code from get_execution_data: {:?}", error)
        }),
    }
}

/// Get resolved variables with automatic JSON deserialization
pub fn get_resolved_variables_parsed(
    result: *const IcsParseResult
) -> Result<HashMap<String, String>, IcsError> {
    let json_string = get_resolved_variables_json(result)?;
    
    // Validate JSON format before deserialization
    validate_json_format(&json_string)?;
    
    serde_json::from_str(&json_string)
        .map_err(|e| IcsError::json_error_with_snippet(e, &json_string))
}

/// Get resolved variables as raw JSON string
pub fn get_resolved_variables_json(
    result: *const IcsParseResult
) -> Result<String, IcsError> {
    if result.is_null() {
        return Err(IcsError::NullPointer);
    }
    
    let mut json_ptr: *mut c_char = std::ptr::null_mut();
    
    let error_code = unsafe {
        ics_get_resolved_variables(result, &mut json_ptr)
    };
    
    match error_code {
        IcsErrorCode::Success => {
            if json_ptr.is_null() {
                Err(IcsError::FfiBoundary { 
                    context: "Parser returned success but null JSON pointer for resolved variables".to_string()
                })
            } else {
                let json_string = unsafe {
                    let result = convert_c_string_to_string(json_ptr);
                    ics_free_string(json_ptr);
                    result
                };
                
                // Empty variables object is valid
                if json_string.is_empty() {
                    Ok("{}".to_string())
                } else {
                    Ok(json_string)
                }
            }
        }
        IcsErrorCode::InternalError => Err(IcsError::InternalError),
        IcsErrorCode::NullPointer => Err(IcsError::NullPointer),
        error => Err(IcsError::FfiBoundary { 
            context: format!("Unexpected error code from get_resolved_variables: {:?}", error)
        }),
    }
}

/// Get execution metadata with automatic JSON deserialization
pub fn get_execution_metadata_parsed(
    result: *const IcsParseResult
) -> Result<ExecutionMetadata, IcsError> {
    let json_string = get_execution_metadata_json(result)?;
    
    // Validate JSON format before deserialization
    validate_json_format(&json_string)?;
    
    serde_json::from_str(&json_string)
        .map_err(|e| IcsError::json_error_with_snippet(e, &json_string))
}

/// Get execution metadata as raw JSON string
pub fn get_execution_metadata_json(
    result: *const IcsParseResult
) -> Result<String, IcsError> {
    if result.is_null() {
        return Err(IcsError::NullPointer);
    }
    
    let mut json_ptr: *mut c_char = std::ptr::null_mut();
    
    let error_code = unsafe {
        ics_get_execution_metadata(result, &mut json_ptr)
    };
    
    match error_code {
        IcsErrorCode::Success => {
            if json_ptr.is_null() {
                Err(IcsError::FfiBoundary { 
                    context: "Parser returned success but null JSON pointer for execution metadata".to_string()
                })
            } else {
                let json_string = unsafe {
                    let result = convert_c_string_to_string(json_ptr);
                    ics_free_string(json_ptr);
                    result
                };
                
                if json_string.is_empty() {
                    Err(IcsError::JsonDeserialization {
                        message: "Parser returned empty execution metadata JSON".to_string(),
                        json_snippet: "empty".to_string(),
                    })
                } else {
                    Ok(json_string)
                }
            }
        }
        IcsErrorCode::InternalError => Err(IcsError::InternalError),
        IcsErrorCode::NullPointer => Err(IcsError::NullPointer),
        error => Err(IcsError::FfiBoundary { 
            context: format!("Unexpected error code from get_execution_metadata: {:?}", error)
        }),
    }
}

/// Validate execution data structure and content
pub fn validate_execution_data(execution_data: &ExecutionData) -> Result<(), IcsError> {
    // Check for basic structural validity
    if execution_data.criteria.is_empty() {
        return Err(IcsError::JsonDeserialization {
            message: "Execution data contains no criteria".to_string(),
            json_snippet: "criteria: []".to_string(),
        });
    }
    
    // Validate CTN types are not empty
    for (index, criterion) in execution_data.criteria.iter().enumerate() {
        if criterion.ctn_type.is_empty() {
            return Err(IcsError::JsonDeserialization {
                message: format!("Criterion at index {} has empty CTN type", index),
                json_snippet: format!("criterion[{}].ctn_type: \"\"", index),
            });
        }
        
        // Validate test specification has required fields
        if criterion.test_specification.existence_check.is_empty() {
            return Err(IcsError::JsonDeserialization {
                message: format!("Criterion at index {} has empty existence_check", index),
                json_snippet: format!("criterion[{}].test_specification.existence_check: \"\"", index),
            });
        }
        
        if criterion.test_specification.item_check.is_empty() {
            return Err(IcsError::JsonDeserialization {
                message: format!("Criterion at index {} has empty item_check", index),
                json_snippet: format!("criterion[{}].test_specification.item_check: \"\"", index),
            });
        }
        
        // Validate state operations have required fields
        for state in &criterion.local_states {
            for (op_index, operation) in state.operations.iter().enumerate() {
                if operation.field.is_empty() {
                    return Err(IcsError::JsonDeserialization {
                        message: format!("State '{}' operation {} has empty field name", state.id, op_index),
                        json_snippet: format!("state.operations[{}].field: \"\"", op_index),
                    });
                }
                
                if operation.operation.is_empty() {
                    return Err(IcsError::JsonDeserialization {
                        message: format!("State '{}' operation {} has empty operation type", state.id, op_index),
                        json_snippet: format!("state.operations[{}].operation: \"\"", op_index),
                    });
                }
                
                if operation.data_type.is_empty() {
                    return Err(IcsError::JsonDeserialization {
                        message: format!("State '{}' operation {} has empty data type", state.id, op_index),
                        json_snippet: format!("state.operations[{}].data_type: \"\"", op_index),
                    });
                }
            }
        }
        
        // Validate resolved global states
        for state in &criterion.resolved_global_states {
            if state.id.is_empty() {
                return Err(IcsError::JsonDeserialization {
                    message: "Resolved global state has empty ID".to_string(),
                    json_snippet: "resolved_global_states[].id: \"\"".to_string(),
                });
            }
        }
        
        // Validate object data
        if let Some(ref local_obj) = criterion.local_object {
            if local_obj.id.is_empty() {
                return Err(IcsError::JsonDeserialization {
                    message: "Local object has empty ID".to_string(),
                    json_snippet: "local_object.id: \"\"".to_string(),
                });
            }
        }
        
        for obj in &criterion.resolved_global_objects {
            if obj.id.is_empty() {
                return Err(IcsError::JsonDeserialization {
                    message: "Resolved global object has empty ID".to_string(),
                    json_snippet: "resolved_global_objects[].id: \"\"".to_string(),
                });
            }
        }
    }
    
    Ok(())
}

/// Validate execution metadata structure and content
pub fn validate_execution_metadata(metadata: &ExecutionMetadata) -> Result<(), IcsError> {
    // Check that metadata makes sense
    if metadata.total_criteria == 0 && metadata.validation_passed {
        return Err(IcsError::JsonDeserialization {
            message: "Metadata indicates validation passed but total_criteria is 0".to_string(),
            json_snippet: "total_criteria: 0, validation_passed: true".to_string(),
        });
    }
    
    // Check CTN types list
    if metadata.ctn_types.is_empty() && metadata.total_criteria > 0 {
        return Err(IcsError::JsonDeserialization {
            message: "Metadata has criteria but no CTN types listed".to_string(),
            json_snippet: "ctn_types: [], total_criteria: > 0".to_string(),
        });
    }
    
    // Validate CTN types are not empty strings
    for (index, ctn_type) in metadata.ctn_types.iter().enumerate() {
        if ctn_type.is_empty() {
            return Err(IcsError::JsonDeserialization {
                message: format!("CTN type at index {} is empty", index),
                json_snippet: format!("ctn_types[{}]: \"\"", index),
            });
        }
    }
    
    // Validate processing stats make sense
    if metadata.processing_stats.duration_ms == 0 && metadata.processing_stats.token_count > 0 {
        return Err(IcsError::JsonDeserialization {
            message: "Processing stats show tokens processed but zero duration".to_string(),
            json_snippet: "duration_ms: 0, token_count: > 0".to_string(),
        });
    }
    
    // Check for reasonable bounds on processing stats
    if metadata.processing_stats.token_count > 10_000_000 {
        return Err(IcsError::JsonDeserialization {
            message: format!("Token count {} exceeds reasonable limit", metadata.processing_stats.token_count),
            json_snippet: format!("token_count: {}", metadata.processing_stats.token_count),
        });
    }
    
    if metadata.processing_stats.symbol_count > 1_000_000 {
        return Err(IcsError::JsonDeserialization {
            message: format!("Symbol count {} exceeds reasonable limit", metadata.processing_stats.symbol_count),
            json_snippet: format!("symbol_count: {}", metadata.processing_stats.symbol_count),
        });
    }
    
    // Validate counts match expectations
    if metadata.global_object_count > 10_000 {
        return Err(IcsError::JsonDeserialization {
            message: format!("Global object count {} exceeds reasonable limit", metadata.global_object_count),
            json_snippet: format!("global_object_count: {}", metadata.global_object_count),
        });
    }
    
    if metadata.global_state_count > 10_000 {
        return Err(IcsError::JsonDeserialization {
            message: format!("Global state count {} exceeds reasonable limit", metadata.global_state_count),
            json_snippet: format!("global_state_count: {}", metadata.global_state_count),
        });
    }
    
    Ok(())
}

/// Get execution data with validation
pub fn get_execution_data_validated(
    result: *const IcsParseResult
) -> Result<ExecutionData, IcsError> {
    let execution_data = get_execution_data_parsed(result)?;
    validate_execution_data(&execution_data)?;
    Ok(execution_data)
}

/// Get execution metadata with validation
pub fn get_execution_metadata_validated(
    result: *const IcsParseResult
) -> Result<ExecutionMetadata, IcsError> {
    let metadata = get_execution_metadata_parsed(result)?;
    validate_execution_metadata(&metadata)?;
    Ok(metadata)
}

/// Get all execution data components in a single operation
pub fn get_complete_execution_data(
    result: *const IcsParseResult
) -> Result<CompleteExecutionData, IcsError> {
    let execution_data = get_execution_data_parsed(result)?;
    let metadata = get_execution_metadata_parsed(result)?;
    let variables = get_resolved_variables_parsed(result)?;
    
    // Validate consistency between components
    if execution_data.criteria.len() != metadata.total_criteria {
        return Err(IcsError::JsonDeserialization {
            message: format!(
                "Inconsistent criteria count: execution_data has {} but metadata says {}",
                execution_data.criteria.len(),
                metadata.total_criteria
            ),
            json_snippet: format!(
                "criteria.len(): {}, total_criteria: {}",
                execution_data.criteria.len(),
                metadata.total_criteria
            ),
        });
    }
    
    // Validate that resolved variables in execution data match the separate variables call
    for var_name in execution_data.resolved_variables.keys() {
        if !variables.contains_key(var_name) {
            return Err(IcsError::JsonDeserialization {
                message: format!("Variable '{}' in execution data but not in resolved variables", var_name),
                json_snippet: format!("variable '{}' missing from resolved_variables", var_name),
            });
        }
    }
    
    // Validate CTN types consistency
    let execution_ctn_types = execution_data.get_ctn_types();
    for ctn_type in &execution_ctn_types {
        if !metadata.ctn_types.contains(ctn_type) {
            return Err(IcsError::JsonDeserialization {
                message: format!("CTN type '{}' in execution data but not in metadata", ctn_type),
                json_snippet: format!("ctn_type '{}' missing from metadata.ctn_types", ctn_type),
            });
        }
    }
    
    Ok(CompleteExecutionData {
        execution_data,
        metadata,
        resolved_variables: variables,
    })
}

/// Complete execution data package with validation
#[derive(Debug, Clone)]
pub struct CompleteExecutionData {
    pub execution_data: ExecutionData,
    pub metadata: ExecutionMetadata,
    pub resolved_variables: HashMap<String, String>,
}

impl CompleteExecutionData {
    /// Check if this execution data is ready for scanner processing
    pub fn is_ready_for_scanning(&self) -> bool {
        self.metadata.validation_passed && 
        !self.execution_data.criteria.is_empty() &&
        self.execution_data.has_validations()
    }
    
    /// Get summary of what scanner modules will be needed
    pub fn get_scanner_requirements(&self) -> ScannerRequirements {
        let ctn_types = self.execution_data.get_ctn_types();
        let total_criteria = self.execution_data.criteria.len();
        let has_dependencies = self.metadata.has_dependencies;
        
        // Analyze complexity factors
        let total_operations: usize = self.execution_data.criteria
            .iter()
            .map(|c| c.total_operations())
            .sum();
        
        let unique_object_count = self.execution_data.global_objects.len() +
            self.execution_data.criteria
                .iter()
                .filter(|c| c.local_object.is_some())
                .count();
        
        let unique_state_count = self.execution_data.global_states.len() +
            self.execution_data.criteria
                .iter()
                .map(|c| c.local_states.len())
                .sum::<usize>();
        
        ScannerRequirements {
            required_ctn_types: ctn_types,
            total_criteria,
            total_operations,
            unique_object_count,
            unique_state_count,
            has_dependencies,
            requires_variable_resolution: !self.resolved_variables.is_empty(),
            requires_global_state_resolution: !self.execution_data.global_states.is_empty(),
            requires_global_object_resolution: !self.execution_data.global_objects.is_empty(),
        }
    }
    
    /// Check if execution data has any issues that would prevent scanning
    pub fn validate_for_scanning(&self) -> Result<(), IcsError> {
        // Basic validation
        validate_execution_data(&self.execution_data)?;
        validate_execution_metadata(&self.metadata)?;
        
        // Scanner-specific validation
        if !self.is_ready_for_scanning() {
            return Err(IcsError::Configuration {
                message: "Execution data is not ready for scanning".to_string(),
            });
        }
        
        // Check for required components
        if self.execution_data.criteria.is_empty() {
            return Err(IcsError::Configuration {
                message: "No criteria found in execution data".to_string(),
            });
        }
        
        // Validate that all criteria have some form of validation
        for (index, criterion) in self.execution_data.criteria.iter().enumerate() {
            if !criterion.has_explicit_validations() && !criterion.has_implicit_validations() {
                return Err(IcsError::Configuration {
                    message: format!("Criterion at index {} has no validations", index),
                });
            }
        }
        
        Ok(())
    }
    
    /// Get performance estimation for scanning
    pub fn get_performance_estimate(&self) -> PerformanceEstimate {
        let requirements = self.get_scanner_requirements();
        let complexity = requirements.complexity_score();
        
        // Estimate based on complexity score
        let estimated_duration_ms = match complexity {
            0..=20 => 100,      // Very simple
            21..=40 => 500,     // Simple
            41..=60 => 2000,    // Moderate
            61..=80 => 10000,   // Complex
            _ => 30000,         // Very complex
        };
        
        let memory_estimate_mb = (requirements.total_criteria * 2) +
            (requirements.unique_object_count / 10) +
            (requirements.unique_state_count / 10) +
            if requirements.has_dependencies { 10 } else { 0 };
        
        PerformanceEstimate {
            complexity_score: complexity,
            estimated_duration_ms,
            estimated_memory_mb: memory_estimate_mb as u32,
            parallel_execution_possible: !requirements.has_dependencies,
            io_intensive: requirements.required_ctn_types.contains(&"file_test".to_string()),
            cpu_intensive: requirements.total_operations > 100,
        }
    }
}

/// Requirements for scanner implementation with detailed metrics
#[derive(Debug, Clone)]
pub struct ScannerRequirements {
    pub required_ctn_types: Vec<String>,
    pub total_criteria: usize,
    pub total_operations: usize,
    pub unique_object_count: usize,
    pub unique_state_count: usize,
    pub has_dependencies: bool,
    pub requires_variable_resolution: bool,
    pub requires_global_state_resolution: bool,
    pub requires_global_object_resolution: bool,
}

impl ScannerRequirements {
    /// Check if all required CTN types are supported
    pub fn check_ctn_support(&self, supported_types: &[&str]) -> Vec<String> {
        self.required_ctn_types
            .iter()
            .filter(|ctn_type| !supported_types.contains(&ctn_type.as_str()))
            .cloned()
            .collect()
    }
    
    /// Get complexity score for this scan (0-100)
    pub fn complexity_score(&self) -> u32 {
        let mut score = 0;
        
        // Base score from criteria count (0-25 points)
        score += std::cmp::min(self.total_criteria * 3, 25) as u32;
        
        // Operations complexity (0-20 points)
        score += std::cmp::min(self.total_operations / 5, 20) as u32;
        
        // CTN type diversity (0-15 points)
        score += std::cmp::min(self.required_ctn_types.len() * 3, 15) as u32;
        
        // Object/state complexity (0-20 points)
        let data_complexity = (self.unique_object_count + self.unique_state_count) / 2;
        score += std::cmp::min(data_complexity * 2, 20) as u32;
        
        // Additional complexity factors (0-20 points total)
        if self.has_dependencies { score += 8; }
        if self.requires_variable_resolution { score += 4; }
        if self.requires_global_state_resolution { score += 4; }
        if self.requires_global_object_resolution { score += 4; }
        
        std::cmp::min(score, 100)
    }
    
    /// Check if this scan can be executed in parallel
    pub fn supports_parallel_execution(&self) -> bool {
        !self.has_dependencies
    }
    
    /// Get recommended scanner configuration
    pub fn get_recommended_config(&self) -> ScannerConfig {
        let complexity = self.complexity_score();
        
        ScannerConfig {
            max_concurrent_criteria: if self.supports_parallel_execution() {
                match complexity {
                    0..=30 => 4,
                    31..=60 => 2,
                    _ => 1,
                }
            } else {
                1
            },
            timeout_ms: match complexity {
                0..=20 => 30_000,
                21..=50 => 120_000,
                _ => 300_000,
            },
            memory_limit_mb: 100 + (complexity * 5),
            enable_caching: self.unique_object_count > 10 || self.unique_state_count > 10,
            log_level: if complexity > 70 { "debug" } else { "info" }.to_string(),
        }
    }
}

/// Performance estimation for scanning operations
#[derive(Debug, Clone)]
pub struct PerformanceEstimate {
    pub complexity_score: u32,
    pub estimated_duration_ms: u32,
    pub estimated_memory_mb: u32,
    pub parallel_execution_possible: bool,
    pub io_intensive: bool,
    pub cpu_intensive: bool,
}

/// Recommended scanner configuration
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub max_concurrent_criteria: usize,
    pub timeout_ms: u32,
    pub memory_limit_mb: u32,
    pub enable_caching: bool,
    pub log_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{CriterionData, TestSpecification, ProcessingStats};
    
    #[test]
    fn test_null_pointer_handling() {
        let result = get_execution_data_json(std::ptr::null());
        assert!(matches!(result, Err(IcsError::NullPointer)));
        
        let result = get_resolved_variables_json(std::ptr::null());
        assert!(matches!(result, Err(IcsError::NullPointer)));
        
        let result = get_execution_metadata_json(std::ptr::null());
        assert!(matches!(result, Err(IcsError::NullPointer)));
    }
    
    #[test]
    fn test_execution_data_validation() {
        // Test empty criteria validation
        let empty_execution_data = ExecutionData {
            criteria: vec![],
            resolved_variables: HashMap::new(),
            global_objects: HashMap::new(),
            global_states: HashMap::new(),
        };
        
        let result = validate_execution_data(&empty_execution_data);
        assert!(result.is_err());
        
        // Test valid execution data
        let valid_execution_data = ExecutionData {
            criteria: vec![CriterionData {
                ctn_type: "file_test".to_string(),
                test_specification: TestSpecification {
                    existence_check: "any".to_string(),
                    item_check: "all".to_string(),
                    state_operator: None,
                },
                resolved_global_states: vec![],
                resolved_global_objects: vec![],
                local_states: vec![],
                local_object: None,
            }],
            resolved_variables: HashMap::new(),
            global_objects: HashMap::new(),
            global_states: HashMap::new(),
        };
        
        let result = validate_execution_data(&valid_execution_data);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_metadata_validation() {
        let invalid_metadata = ExecutionMetadata {
            validation_passed: true,
            total_criteria: 0, // Invalid: passed but no criteria
            has_dependencies: false,
            global_object_count: 0,
            global_state_count: 0,
            processing_stats: ProcessingStats {
                token_count: 0,
                symbol_count: 0,
                duration_ms: 0,
                file_size_bytes: 0,
            },
            ctn_types: vec![],
        };
        
        let result = validate_execution_metadata(&invalid_metadata);
        assert!(result.is_err());
        
        let valid_metadata = ExecutionMetadata {
            validation_passed: true,
            total_criteria: 1,
            has_dependencies: false,
            global_object_count: 0,
            global_state_count: 0,
            processing_stats: ProcessingStats {
                token_count: 100,
                symbol_count: 10,
                duration_ms: 50,
                file_size_bytes: 1024,
            },
            ctn_types: vec!["file_test".to_string()],
        };
        
        let result = validate_execution_metadata(&valid_metadata);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_scanner_requirements() {
        let requirements = ScannerRequirements {
            required_ctn_types: vec!["file_test".to_string(), "process_test".to_string()],
            total_criteria: 5,
            total_operations: 15,
            unique_object_count: 3,
            unique_state_count: 4,
            has_dependencies: true,
            requires_variable_resolution: true,
            requires_global_state_resolution: false,
            requires_global_object_resolution: true,
        };
        
        let unsupported = requirements.check_ctn_support(&["file_test"]);
        assert_eq!(unsupported, vec!["process_test"]);
        
        let complexity = requirements.complexity_score();
        assert!(complexity > 0 && complexity <= 100);
        
        assert!(!requirements.supports_parallel_execution()); // Has dependencies
        
        let config = requirements.get_recommended_config();
        assert_eq!(config.max_concurrent_criteria, 1); // No parallel due to dependencies
    }
    
    #[test]
    fn test_complete_execution_data() {
        let execution_data = ExecutionData {
            criteria: vec![],
            resolved_variables: HashMap::new(),
            global_objects: HashMap::new(),
            global_states: HashMap::new(),
        };
        
        let metadata = ExecutionMetadata {
            validation_passed: false,
            total_criteria: 0,
            has_dependencies: false,
            global_object_count: 0,
            global_state_count: 0,
            processing_stats: ProcessingStats {
                token_count: 0,
                symbol_count: 0,
                duration_ms: 0,
                file_size_bytes: 0,
            },
            ctn_types: vec![],
        };
        
        let complete = CompleteExecutionData {
            execution_data,
            metadata,
            resolved_variables: HashMap::new(),
        };
        
        assert!(!complete.is_ready_for_scanning());
        
        let requirements = complete.get_scanner_requirements();
        assert_eq!(requirements.total_criteria, 0);
        
        let performance = complete.get_performance_estimate();
        assert_eq!(performance.complexity_score, 0);
        assert!(performance.parallel_execution_possible);
    }
    
    #[test]
    fn test_performance_estimate() {
        let simple_complete = CompleteExecutionData {
            execution_data: ExecutionData {
                criteria: vec![CriterionData {
                    ctn_type: "file_test".to_string(),
                    test_specification: TestSpecification {
                        existence_check: "any".to_string(),
                        item_check: "all".to_string(),
                        state_operator: None,
                    },
                    resolved_global_states: vec![],
                    resolved_global_objects: vec![],
                    local_states: vec![],
                    local_object: None,
                }],
                resolved_variables: HashMap::new(),
                global_objects: HashMap::new(),
                global_states: HashMap::new(),
            },
            metadata: ExecutionMetadata {
                validation_passed: true,
                total_criteria: 1,
                has_dependencies: false,
                global_object_count: 0,
                global_state_count: 0,
                processing_stats: ProcessingStats {
                    token_count: 100,
                    symbol_count: 10,
                    duration_ms: 50,
                    file_size_bytes: 1024,
                },
                ctn_types: vec!["file_test".to_string()],
            },
            resolved_variables: HashMap::new(),
        };
        
        let performance = simple_complete.get_performance_estimate();
        assert!(performance.complexity_score <= 100);
        assert!(performance.estimated_duration_ms > 0);
        assert!(performance.io_intensive); // file_test is IO intensive
    }
}