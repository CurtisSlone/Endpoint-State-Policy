//! Filter evaluation for OBJECT and SET filters
//! 
//! Filters are applied AFTER collection but BEFORE executor validation.
//! This allows filtering based on collected data while keeping collectors simple.

use crate::strategies::{CollectedData, CtnExecutionError};
use crate::types::execution_context::ExecutionContext;
use crate::types::filter::ResolvedFilterSpec;
use crate::types::ExecutableObject;
use std::collections::HashMap;

pub struct FilterEvaluator;

impl FilterEvaluator {
    /// Evaluate filter against collected data
    /// Returns true if object should be INCLUDED, false if EXCLUDED
    pub fn evaluate_filter(
        filter: &ResolvedFilterSpec,
        collected_data: &CollectedData,
        context: &ExecutionContext,
    ) -> Result<bool, FilterEvaluationError> {
        // Get all states referenced by filter
        let mut all_states_satisfied = true;
        
        for state_ref in &filter.state_refs {
            let state = context.global_states.get(state_ref)
                .ok_or_else(|| FilterEvaluationError::StateNotFound {
                    state_id: state_ref.clone(),
                })?;
            
            // Evaluate this state against collected data
            let satisfied = Self::evaluate_state(state, collected_data)?;
            
            if !satisfied {
                all_states_satisfied = false;
                break; // Early exit - one failure means filter fails
            }
        }
        
        // Apply filter action logic
        let include = match filter.action {
            crate::types::filter::FilterAction::Include => all_states_satisfied,
            crate::types::filter::FilterAction::Exclude => !all_states_satisfied,
        };
        
        Ok(include)
    }
    
    /// Evaluate a single state against collected data
    fn evaluate_state(
        state: &crate::types::state::ResolvedState,
        collected_data: &CollectedData,
    ) -> Result<bool, FilterEvaluationError> {
        // For each field in the state, check if it matches collected data
        for field in &state.resolved_fields {
            // Map state field name to collected data field name
            let data_field_name = &field.name; // May need mapping logic here
            
            let actual_value = collected_data.get_field(data_field_name)
                .ok_or_else(|| FilterEvaluationError::FieldNotFound {
                    field_name: field.name.clone(),
                })?;
            
            // Perform comparison using existing comparison logic
            let field_passed = crate::execution::comparisons::compare_values(
                &field.value,
                actual_value,
                field.operation,
            )?;
            
            if !field_passed {
                return Ok(false); // One field fails = state fails
            }
        }
        
        Ok(true) // All fields passed
    }
    
    /// Filter a collection of collected data based on object filters
    /// Returns only the objects that pass all filters
    pub fn apply_object_filters(
        objects_with_data: HashMap<String, (ExecutableObject, CollectedData)>,
        context: &ExecutionContext,
    ) -> Result<HashMap<String, CollectedData>, FilterEvaluationError> {
        let mut filtered = HashMap::new();
        
        for (object_id, (object, data)) in objects_with_data {
            let should_include = Self::should_include_object(&object, &data, context)?;
            
            if should_include {
                filtered.insert(object_id, data);
            }
        }
        
        Ok(filtered)
    }
    
    /// Check if a single object should be included based on its filters
    fn should_include_object(
        object: &ExecutableObject,
        data: &CollectedData,
        context: &ExecutionContext,
    ) -> Result<bool, FilterEvaluationError> {
        let filters = object.get_filters();
        
        if filters.is_empty() {
            return Ok(true); // No filters = always include
        }
        
        // ALL filters must pass (AND logic between multiple filters)
        for filter in filters {
            let include = Self::evaluate_filter(filter, data, context)?;
            if !include {
                return Ok(false); // One filter excludes = object excluded
            }
        }
        
        Ok(true)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FilterEvaluationError {
    #[error("Filter references undefined state: {state_id}")]
    StateNotFound { state_id: String },
    
    #[error("Field '{field_name}' not found in collected data")]
    FieldNotFound { field_name: String },
    
    #[error("Comparison failed: {reason}")]
    ComparisonFailed { reason: String },
    
    #[error("Invalid filter configuration: {reason}")]
    InvalidFilter { reason: String },
}

// Convert comparison errors to filter errors
impl From<crate::execution::comparisons::ComparisonError> for FilterEvaluationError {
    fn from(err: crate::execution::comparisons::ComparisonError) -> Self {
        FilterEvaluationError::ComparisonFailed {
            reason: err.to_string(),
        }
    }
}