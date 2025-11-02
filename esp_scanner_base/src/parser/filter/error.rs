/// Filter-specific parsing errors
#[derive(Debug, Clone)]
pub enum FilterParsingError {
    /// Failed to parse filter specification from JSON
    FilterSpecParsingFailed { context: String, cause: String },

    /// Invalid filter action encountered
    InvalidFilterAction {
        context: String,
        action: String,
        valid_actions: Vec<String>,
    },

    /// Missing filter action field
    MissingFilterAction { context: String },

    /// Filter state references array missing or invalid
    InvalidStateReferences { context: String, cause: String },

    /// Empty state references (filters must reference at least one state)
    EmptyStateReferences { context: String },

    /// Invalid state reference format
    InvalidStateReference {
        context: String,
        state_ref: String,
        cause: String,
    },

    /// Filter block structure invalid
    InvalidFilterStructure {
        context: String,
        available_keys: Vec<String>,
        expected_keys: Vec<String>,
    },

    /// Multiple filter actions specified (should be one)
    MultipleFilterActions {
        context: String,
        actions: Vec<String>,
    },

    /// Filter specification empty or null
    EmptyFilterSpec { context: String },

    /// State reference validation failed
    StateReferenceValidationFailed {
        context: String,
        state_id: String,
        validation_error: String,
    },

    /// Filter nesting too deep (if nested filters are supported)
    FilterNestingTooDeep {
        context: String,
        depth: usize,
        max_depth: usize,
    },

    /// Invalid filter context (e.g., filter used in wrong place)
    InvalidFilterContext {
        filter_context: String,
        expected_contexts: Vec<String>,
    },
}

impl FilterParsingError {
    /// Create filter spec parsing error
    pub fn filter_spec_parsing_failed(context: &str, cause: &str) -> Self {
        Self::FilterSpecParsingFailed {
            context: context.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid filter action error
    pub fn invalid_filter_action(context: &str, action: &str) -> Self {
        Self::InvalidFilterAction {
            context: context.to_string(),
            action: action.to_string(),
            valid_actions: vec!["include".to_string(), "exclude".to_string()],
        }
    }

    /// Create missing filter action error
    pub fn missing_filter_action(context: &str) -> Self {
        Self::MissingFilterAction {
            context: context.to_string(),
        }
    }

    /// Create invalid state references error
    pub fn invalid_state_references(context: &str, cause: &str) -> Self {
        Self::InvalidStateReferences {
            context: context.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create empty state references error
    pub fn empty_state_references(context: &str) -> Self {
        Self::EmptyStateReferences {
            context: context.to_string(),
        }
    }

    /// Create invalid state reference error
    pub fn invalid_state_reference(context: &str, state_ref: &str, cause: &str) -> Self {
        Self::InvalidStateReference {
            context: context.to_string(),
            state_ref: state_ref.to_string(),
            cause: cause.to_string(),
        }
    }

    /// Create invalid filter structure error
    pub fn invalid_filter_structure(
        context: &str,
        available_keys: Vec<String>,
        expected_keys: Vec<String>,
    ) -> Self {
        Self::InvalidFilterStructure {
            context: context.to_string(),
            available_keys,
            expected_keys,
        }
    }

    /// Create empty filter spec error
    pub fn empty_filter_spec(context: &str) -> Self {
        Self::EmptyFilterSpec {
            context: context.to_string(),
        }
    }
}

impl std::fmt::Display for FilterParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FilterSpecParsingFailed { context, cause } => {
                write!(
                    f,
                    "Failed to parse filter specification in '{}': {}",
                    context, cause
                )
            }
            Self::InvalidFilterAction {
                context,
                action,
                valid_actions,
            } => {
                write!(
                    f,
                    "Invalid filter action '{}' in '{}'. Valid actions: [{}]",
                    action,
                    context,
                    valid_actions.join(", ")
                )
            }
            Self::MissingFilterAction { context } => {
                write!(f, "Filter action missing in '{}'", context)
            }
            Self::InvalidStateReferences { context, cause } => {
                write!(f, "Invalid state references in '{}': {}", context, cause)
            }
            Self::EmptyStateReferences { context } => {
                write!(
                    f,
                    "Filter in '{}' has no state references (at least one required)",
                    context
                )
            }
            Self::InvalidStateReference {
                context,
                state_ref,
                cause,
            } => {
                write!(
                    f,
                    "Invalid state reference '{}' in '{}': {}",
                    state_ref, context, cause
                )
            }
            Self::InvalidFilterStructure {
                context,
                available_keys,
                expected_keys,
            } => {
                write!(
                    f,
                    "Invalid filter structure in '{}'. Available: [{}], Expected: [{}]",
                    context,
                    available_keys.join(", "),
                    expected_keys.join(", ")
                )
            }
            Self::MultipleFilterActions { context, actions } => {
                write!(
                    f,
                    "Multiple filter actions specified in '{}': [{}] (only one allowed)",
                    context,
                    actions.join(", ")
                )
            }
            Self::EmptyFilterSpec { context } => {
                write!(f, "Empty filter specification in '{}'", context)
            }
            Self::StateReferenceValidationFailed {
                context,
                state_id,
                validation_error,
            } => {
                write!(
                    f,
                    "State reference validation failed for '{}' in '{}': {}",
                    state_id, context, validation_error
                )
            }
            Self::FilterNestingTooDeep {
                context,
                depth,
                max_depth,
            } => {
                write!(
                    f,
                    "Filter nesting too deep in '{}': {} > {} (max allowed)",
                    context, depth, max_depth
                )
            }
            Self::InvalidFilterContext {
                filter_context,
                expected_contexts,
            } => {
                write!(
                    f,
                    "Filter used in invalid context '{}'. Valid contexts: [{}]",
                    filter_context,
                    expected_contexts.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for FilterParsingError {}
