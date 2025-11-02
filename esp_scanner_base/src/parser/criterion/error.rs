/// Criterion-specific parsing errors
#[derive(Debug, Clone)]
pub enum CriterionParsingError {
    /// Failed to parse criterion declaration from JSON
    CriterionDeclarationParsingFailed { ctn_type: String, cause: String },

    /// Failed to parse criteria structure from JSON
    CriteriaStructureParsingFailed {
        criteria_index: usize,
        cause: String,
    },

    /// Test specification parsing failed
    TestSpecificationParsingFailed { ctn_type: String, cause: String },

    /// Invalid existence check value
    InvalidExistenceCheck {
        ctn_type: String,
        existence_check: String,
        valid_checks: Vec<String>,
    },

    /// Invalid item check value
    InvalidItemCheck {
        ctn_type: String,
        item_check: String,
        valid_checks: Vec<String>,
    },

    /// Invalid state join operator
    InvalidStateJoinOperator {
        ctn_type: String,
        state_operator: String,
        valid_operators: Vec<String>,
    },

    /// CTN content ordering violation
    CtnContentOrderingViolation {
        ctn_type: String,
        expected_order: String,
        violation_description: String,
    },

    /// Multiple local objects in single CTN
    MultipleCtnObjects {
        ctn_type: String,
        first_object_id: String,
        second_object_id: String,
    },

    /// Empty CTN definition (no content)
    EmptyCtnDefinition { ctn_type: String },

    /// CTN type identifier validation failed
    InvalidCtnTypeIdentifier { ctn_type: String, cause: String },

    /// Missing required field in CTN definition
    MissingRequiredField {
        ctn_type: String,
        missing_field: String,
    },

    /// State reference parsing failed in CTN
    StateReferenceParsingFailed {
        ctn_type: String,
        reference_index: usize,
        cause: String,
    },

    /// Object reference parsing failed in CTN
    ObjectReferenceParsingFailed {
        ctn_type: String,
        reference_index: usize,
        cause: String,
    },

    /// Local state parsing failed in CTN
    LocalStateParsingFailed {
        ctn_type: String,
        state_index: usize,
        state_id: String,
        cause: String,
    },

    /// Local object parsing failed in CTN
    LocalObjectParsingFailed {
        ctn_type: String,
        object_id: String,
        cause: String,
    },

    /// Criteria nesting depth limit exceeded
    CriteriaNestingDepthExceeded {
        current_depth: usize,
        max_depth: usize,
    },

    /// Invalid logical operator for criteria
    InvalidLogicalOperator {
        criteria_index: usize,
        logical_op: String,
        valid_operators: Vec<String>,
    },

    /// Criteria flattening failed
    CriteriaFlatteningFailed {
        criteria_index: usize,
        cause: String,
    },

    /// CTN node ID assignment failed
    CtnNodeIdAssignmentFailed { ctn_type: String, cause: String },

    /// Test specification validation failed
    TestSpecificationValidationFailed {
        ctn_type: String,
        test_inconsistency: String,
    },

    /// CTN element count limit exceeded
    CtnElementCountLimitExceeded {
        ctn_type: String,
        element_count: usize,
        max_elements: usize,
    },

    /// State reference validation failed
    StateReferenceValidationFailed {
        ctn_type: String,
        state_id: String,
        validation_error: String,
    },

    /// Object reference validation failed
    ObjectReferenceValidationFailed {
        ctn_type: String,
        object_id: String,
        validation_error: String,
    },

    /// Local symbol scope conflict
    LocalSymbolScopeConflict {
        ctn_type: String,
        symbol_id: String,
        conflict_type: String,
    },

    /// CTN content structure validation failed
    CtnContentStructureValidationFailed {
        ctn_type: String,
        expected_structure: String,
        actual_structure: String,
    },

    /// Criterion dependency resolution failed
    CriterionDependencyResolutionFailed {
        ctn_type: String,
        dependency_chain: Vec<String>,
        resolution_error: String,
    },

    /// Test and state count mismatch
    TestStateCountMismatch {
        ctn_type: String,
        test_spec: String,
        state_count: usize,
        expected_state_count: Option<usize>,
    },
}

impl CriterionParsingError {
    /// Create criterion declaration parsing error
    pub fn criterion_declaration_parsing_failed(ctn_type: &str, cause: &str) -> Self {
        Self::CriterionDeclarationParsingFailed {
            ctn_type: ctn_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create criteria structure parsing error
    pub fn criteria_structure_parsing_failed(criteria_index: usize, cause: &str) -> Self {
        Self::CriteriaStructureParsingFailed {
            criteria_index,
            cause: cause.to_string(),
        }
    }

    /// Create test specification parsing error
    pub fn test_specification_parsing_failed(ctn_type: &str, cause: &str) -> Self {
        Self::TestSpecificationParsingFailed {
            ctn_type: ctn_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid existence check error
    pub fn invalid_existence_check(ctn_type: &str, existence_check: &str) -> Self {
        Self::InvalidExistenceCheck {
            ctn_type: ctn_type.to_string(),
            existence_check: existence_check.to_string(),
            valid_checks: vec![
                "any".to_string(),
                "all".to_string(),
                "none".to_string(),
                "at_least_one".to_string(),
                "only_one".to_string(),
            ],
        }
    }

    /// Create invalid item check error
    pub fn invalid_item_check(ctn_type: &str, item_check: &str) -> Self {
        Self::InvalidItemCheck {
            ctn_type: ctn_type.to_string(),
            item_check: item_check.to_string(),
            valid_checks: vec![
                "all".to_string(),
                "at_least_one".to_string(),
                "only_one".to_string(),
                "none_satisfy".to_string(),
            ],
        }
    }

    /// Create CTN content ordering violation error
    pub fn ctn_content_ordering_violation(
        ctn_type: &str,
        expected_order: &str,
        violation_description: &str,
    ) -> Self {
        Self::CtnContentOrderingViolation {
            ctn_type: ctn_type.to_string(),
            expected_order: expected_order.to_string(),
            violation_description: violation_description.to_string(),
        }
    }

    /// Create multiple CTN objects error
    pub fn multiple_ctn_objects(
        ctn_type: &str,
        first_object_id: &str,
        second_object_id: &str,
    ) -> Self {
        Self::MultipleCtnObjects {
            ctn_type: ctn_type.to_string(),
            first_object_id: first_object_id.to_string(),
            second_object_id: second_object_id.to_string(),
        }
    }

    /// Create empty CTN definition error
    pub fn empty_ctn_definition(ctn_type: &str) -> Self {
        Self::EmptyCtnDefinition {
            ctn_type: ctn_type.to_string(),
        }
    }

    /// Create invalid CTN type identifier error
    pub fn invalid_ctn_type_identifier(ctn_type: &str, cause: &str) -> Self {
        Self::InvalidCtnTypeIdentifier {
            ctn_type: ctn_type.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create missing required field error
    pub fn missing_required_field(ctn_type: &str, missing_field: &str) -> Self {
        Self::MissingRequiredField {
            ctn_type: ctn_type.to_string(),
            missing_field: missing_field.to_string(),
        }
    }

    /// Create local state parsing error
    pub fn local_state_parsing_failed(
        ctn_type: &str,
        state_index: usize,
        state_id: &str,
        cause: &str,
    ) -> Self {
        Self::LocalStateParsingFailed {
            ctn_type: ctn_type.to_string(),
            state_index,
            state_id: state_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create local object parsing error
    pub fn local_object_parsing_failed(ctn_type: &str, object_id: &str, cause: &str) -> Self {
        Self::LocalObjectParsingFailed {
            ctn_type: ctn_type.to_string(),
            object_id: object_id.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create criteria nesting depth exceeded error
    pub fn criteria_nesting_depth_exceeded(current_depth: usize, max_depth: usize) -> Self {
        Self::CriteriaNestingDepthExceeded {
            current_depth,
            max_depth,
        }
    }

    /// Create test state count mismatch error
    pub fn test_state_count_mismatch(
        ctn_type: &str,
        test_spec: &str,
        state_count: usize,
        expected_state_count: Option<usize>,
    ) -> Self {
        Self::TestStateCountMismatch {
            ctn_type: ctn_type.to_string(),
            test_spec: test_spec.to_string(),
            state_count,
            expected_state_count,
        }
    }
}

impl std::fmt::Display for CriterionParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CriterionDeclarationParsingFailed { ctn_type, cause } => {
                write!(f, "Failed to parse criterion '{}': {}", ctn_type, cause)
            }
            Self::CriteriaStructureParsingFailed {
                criteria_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse criteria structure at index {}: {}",
                    criteria_index, cause
                )
            }
            Self::TestSpecificationParsingFailed { ctn_type, cause } => {
                write!(
                    f,
                    "Failed to parse test specification for CTN '{}': {}",
                    ctn_type, cause
                )
            }
            Self::InvalidExistenceCheck {
                ctn_type,
                existence_check,
                valid_checks,
            } => {
                write!(
                    f,
                    "Invalid existence check '{}' for CTN '{}'. Valid checks: [{}]",
                    existence_check,
                    ctn_type,
                    valid_checks.join(", ")
                )
            }
            Self::InvalidItemCheck {
                ctn_type,
                item_check,
                valid_checks,
            } => {
                write!(
                    f,
                    "Invalid item check '{}' for CTN '{}'. Valid checks: [{}]",
                    item_check,
                    ctn_type,
                    valid_checks.join(", ")
                )
            }
            Self::InvalidStateJoinOperator {
                ctn_type,
                state_operator,
                valid_operators,
            } => {
                write!(
                    f,
                    "Invalid state join operator '{}' for CTN '{}'. Valid operators: [{}]",
                    state_operator,
                    ctn_type,
                    valid_operators.join(", ")
                )
            }
            Self::CtnContentOrderingViolation {
                ctn_type,
                expected_order,
                violation_description,
            } => {
                write!(
                    f,
                    "CTN content ordering violation in '{}'. Expected order: {}. Violation: {}",
                    ctn_type, expected_order, violation_description
                )
            }
            Self::MultipleCtnObjects {
                ctn_type,
                first_object_id,
                second_object_id,
            } => {
                write!(
                    f,
                    "CTN '{}' has multiple objects: '{}' and '{}' (max 1 allowed per EBNF)",
                    ctn_type, first_object_id, second_object_id
                )
            }
            Self::EmptyCtnDefinition { ctn_type } => {
                write!(
                    f,
                    "CTN '{}' has no content (empty CTNs not allowed)",
                    ctn_type
                )
            }
            Self::InvalidCtnTypeIdentifier { ctn_type, cause } => {
                write!(f, "Invalid CTN type identifier '{}': {}", ctn_type, cause)
            }
            Self::MissingRequiredField {
                ctn_type,
                missing_field,
            } => {
                write!(
                    f,
                    "CTN '{}' missing required field '{}'",
                    ctn_type, missing_field
                )
            }
            Self::StateReferenceParsingFailed {
                ctn_type,
                reference_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse state reference {} in CTN '{}': {}",
                    reference_index, ctn_type, cause
                )
            }
            Self::ObjectReferenceParsingFailed {
                ctn_type,
                reference_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse object reference {} in CTN '{}': {}",
                    reference_index, ctn_type, cause
                )
            }
            Self::LocalStateParsingFailed {
                ctn_type,
                state_index,
                state_id,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse local state '{}' (index {}) in CTN '{}': {}",
                    state_id, state_index, ctn_type, cause
                )
            }
            Self::LocalObjectParsingFailed {
                ctn_type,
                object_id,
                cause,
            } => {
                write!(
                    f,
                    "Failed to parse local object '{}' in CTN '{}': {}",
                    object_id, ctn_type, cause
                )
            }
            Self::CriteriaNestingDepthExceeded {
                current_depth,
                max_depth,
            } => {
                write!(
                    f,
                    "Criteria nesting depth exceeded: {} > {} (max allowed)",
                    current_depth, max_depth
                )
            }
            Self::InvalidLogicalOperator {
                criteria_index,
                logical_op,
                valid_operators,
            } => {
                write!(
                    f,
                    "Invalid logical operator '{}' at criteria index {}. Valid operators: [{}]",
                    logical_op,
                    criteria_index,
                    valid_operators.join(", ")
                )
            }
            Self::CriteriaFlatteningFailed {
                criteria_index,
                cause,
            } => {
                write!(
                    f,
                    "Failed to flatten criteria at index {}: {}",
                    criteria_index, cause
                )
            }
            Self::CtnNodeIdAssignmentFailed { ctn_type, cause } => {
                write!(
                    f,
                    "Failed to assign CTN node ID to '{}': {}",
                    ctn_type, cause
                )
            }
            Self::TestSpecificationValidationFailed {
                ctn_type,
                test_inconsistency,
            } => {
                write!(
                    f,
                    "Test specification validation failed for CTN '{}': {}",
                    ctn_type, test_inconsistency
                )
            }
            Self::CtnElementCountLimitExceeded {
                ctn_type,
                element_count,
                max_elements,
            } => {
                write!(
                    f,
                    "CTN '{}' element count {} exceeds maximum {} allowed",
                    ctn_type, element_count, max_elements
                )
            }
            Self::StateReferenceValidationFailed {
                ctn_type,
                state_id,
                validation_error,
            } => {
                write!(
                    f,
                    "State reference validation failed for '{}' in CTN '{}': {}",
                    state_id, ctn_type, validation_error
                )
            }
            Self::ObjectReferenceValidationFailed {
                ctn_type,
                object_id,
                validation_error,
            } => {
                write!(
                    f,
                    "Object reference validation failed for '{}' in CTN '{}': {}",
                    object_id, ctn_type, validation_error
                )
            }
            Self::LocalSymbolScopeConflict {
                ctn_type,
                symbol_id,
                conflict_type,
            } => {
                write!(
                    f,
                    "Local symbol scope conflict for '{}' in CTN '{}': {}",
                    symbol_id, ctn_type, conflict_type
                )
            }
            Self::CtnContentStructureValidationFailed {
                ctn_type,
                expected_structure,
                actual_structure,
            } => {
                write!(
                    f,
                    "CTN content structure validation failed for '{}'. Expected: {}, Found: {}",
                    ctn_type, expected_structure, actual_structure
                )
            }
            Self::CriterionDependencyResolutionFailed {
                ctn_type,
                dependency_chain,
                resolution_error,
            } => {
                write!(
                    f,
                    "Criterion dependency resolution failed for '{}' [{}]: {}",
                    ctn_type,
                    dependency_chain.join(" -> "),
                    resolution_error
                )
            }
            Self::TestStateCountMismatch {
                ctn_type,
                test_spec,
                state_count,
                expected_state_count,
            } => {
                if let Some(expected) = expected_state_count {
                    write!(f, "Test state count mismatch for CTN '{}'. Test '{}' expects {} states but found {}", 
                           ctn_type, test_spec, expected, state_count)
                } else {
                    write!(f, "Test state count validation failed for CTN '{}'. Test '{}' incompatible with {} states", 
                           ctn_type, test_spec, state_count)
                }
            }
        }
    }
}

impl std::error::Error for CriterionParsingError {}
