//! Enhanced Cycle Detection with Global Logging Integration and Compile-Time Security Limits
//!
//! Simplified cycle detection that works with existing validation results and enforces security boundaries.

use super::types::{SemanticError, SemanticInput};
use crate::{config::constants::compile_time::semantic::*, log_debug, log_info, utils::Span};

/// Analyze dependency cycles from validation results with security limits
pub fn analyze_dependency_cycles(
    input: &SemanticInput,
) -> Result<Vec<SemanticError>, SemanticError> {
    let cycles = &input.validation_result.cycles;

    log_info!("Starting dependency cycle analysis",
        "cycle_count" => cycles.len(),
        "max_cycle_path_length" => MAX_CYCLE_PATH_LENGTH,
        "max_errors_limit" => MAX_SEMANTIC_ERRORS);

    let mut errors = Vec::new();

    if cycles.is_empty() {
        log_info!("No dependency cycles found");
        return Ok(errors);
    }

    // Convert existing cycles to semantic errors with security limits
    for (index, cycle) in cycles.iter().enumerate() {
        log_debug!("Processing detected cycle",
            "cycle_number" => index + 1,
            "cycle_length" => cycle.len(),
            "cycle_path" => cycle.join(" -> "));

        // SECURITY: Limit cycle path length to prevent DoS via deep cycle reporting
        let limited_cycle: Vec<String> = if cycle.len() > MAX_CYCLE_PATH_LENGTH {
            let mut truncated = cycle[..MAX_CYCLE_PATH_LENGTH].to_vec();
            truncated.push("... [truncated for security]".to_string());

            log_info!("Cycle path truncated due to security limit",
                "original_length" => cycle.len(),
                "truncated_length" => truncated.len(),
                "limit" => MAX_CYCLE_PATH_LENGTH);

            truncated
        } else {
            cycle.clone()
        };

        let cycle_error =
            SemanticError::circular_dependency("dependency", limited_cycle, Span::dummy());
        errors.push(cycle_error);

        // SECURITY: Limit total error collection to prevent DoS
        if errors.len() >= MAX_SEMANTIC_ERRORS {
            log_info!("Cycle analysis error limit reached",
                "max_errors" => MAX_SEMANTIC_ERRORS,
                "total_cycles" => cycles.len(),
                "processed_cycles" => index + 1,
                "stopping_analysis" => true);
            break;
        }
    }

    // Log completion summary
    log_info!("Circular dependency analysis completed",
        "cycles_found" => cycles.len(),
        "errors_generated" => errors.len(),
        "truncated_cycles" => errors.iter().filter(|e| {
            if let SemanticError::CircularDependency { cycle_path, .. } = e {
                cycle_path.iter().any(|p| p.contains("truncated"))
            } else {
                false
            }
        }).count(),
        "error_limit_reached" => errors.len() >= MAX_SEMANTIC_ERRORS);

    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input_with_cycle() -> SemanticInput {
        let ast = crate::grammar::ast::nodes::EspFile {
            metadata: None,
            definition: crate::grammar::ast::nodes::DefinitionNode {
                variables: Vec::new(),
                runtime_operations: Vec::new(),
                states: Vec::new(),
                objects: Vec::new(),
                set_operations: Vec::new(),
                criteria: Vec::new(),
                span: Some(Span::dummy()),
            },
            span: Some(Span::dummy()),
        };

        let symbols = crate::symbols::SymbolDiscoveryResult::new();

        // Create validation result with cycle
        let mut validation_result =
            crate::reference_resolution::ReferenceValidationResult::default();
        validation_result.cycles = vec![vec!["A".to_string(), "B".to_string()]];

        SemanticInput::new(ast, symbols, validation_result)
    }

    fn create_test_input_with_long_cycle() -> SemanticInput {
        let ast = crate::grammar::ast::nodes::EspFile {
            metadata: None,
            definition: crate::grammar::ast::nodes::DefinitionNode {
                variables: Vec::new(),
                runtime_operations: Vec::new(),
                states: Vec::new(),
                objects: Vec::new(),
                set_operations: Vec::new(),
                criteria: Vec::new(),
                span: Some(Span::dummy()),
            },
            span: Some(Span::dummy()),
        };

        let symbols = crate::symbols::SymbolDiscoveryResult::new();

        // Create validation result with excessively long cycle
        let mut validation_result =
            crate::reference_resolution::ReferenceValidationResult::default();

        let long_cycle: Vec<String> = (0..150).map(|i| format!("Node{}", i)).collect();

        validation_result.cycles = vec![long_cycle];

        SemanticInput::new(ast, symbols, validation_result)
    }

    #[test]
    fn test_cycle_detection() {
        let _ = crate::logging::init_global_logging();

        let input = create_test_input_with_cycle();
        let result = analyze_dependency_cycles(&input);

        assert!(result.is_ok());
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            SemanticError::CircularDependency { cycle_path, .. } => {
                assert_eq!(cycle_path.len(), 2);
            }
            _ => panic!("Expected circular dependency error"),
        }
    }

    #[test]
    fn test_cycle_path_truncation() {
        let _ = crate::logging::init_global_logging();

        let input = create_test_input_with_long_cycle();
        let result = analyze_dependency_cycles(&input);

        assert!(result.is_ok());
        let errors = result.unwrap();
        assert_eq!(errors.len(), 1);

        match &errors[0] {
            SemanticError::CircularDependency { cycle_path, .. } => {
                // Should be truncated to MAX_CYCLE_PATH_LENGTH + 1 (for truncation message)
                assert!(cycle_path.len() <= MAX_CYCLE_PATH_LENGTH + 1);
                assert!(cycle_path.last().unwrap().contains("truncated"));
            }
            _ => panic!("Expected circular dependency error"),
        }
    }

    #[test]
    fn test_security_constants() {
        // Verify security boundaries are properly set
        assert!(MAX_CYCLE_PATH_LENGTH > 0);
        assert!(MAX_SEMANTIC_ERRORS > 0);
    }
}
