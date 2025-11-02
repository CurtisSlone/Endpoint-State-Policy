use serde::{Deserialize, Serialize};

/// Filter specification for conditional inclusion/exclusion
/// EBNF: filter_spec ::= "FILTER" space filter_action? statement_end filter_references "FILTER_END" statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterSpec {
    /// Filter action (include/exclude)
    pub action: FilterAction,
    /// State references to filter by (must reference global states only)
    pub state_refs: Vec<String>,
}

impl FilterSpec {
    /// Create a new filter specification
    pub fn new(action: FilterAction, state_refs: Vec<String>) -> Self {
        Self { action, state_refs }
    }

    /// Create an include filter
    pub fn include(state_refs: Vec<String>) -> Self {
        Self {
            action: FilterAction::Include,
            state_refs,
        }
    }

    /// Create an exclude filter
    pub fn exclude(state_refs: Vec<String>) -> Self {
        Self {
            action: FilterAction::Exclude,
            state_refs,
        }
    }

    /// Check if this filter has any state references
    pub fn has_state_references(&self) -> bool {
        !self.state_refs.is_empty()
    }

    /// Get the number of state references
    pub fn state_reference_count(&self) -> usize {
        self.state_refs.len()
    }

    /// Check if this filter references a specific state
    pub fn references_state(&self, state_id: &str) -> bool {
        self.state_refs.contains(&state_id.to_string())
    }

    /// Get all state dependencies for this filter
    pub fn get_state_dependencies(&self) -> Vec<String> {
        self.state_refs.clone()
    }

    /// Validate that this filter has at least one state reference
    pub fn validate(&self) -> Result<(), String> {
        if self.state_refs.is_empty() {
            return Err("Filter must reference at least one state".to_string());
        }
        Ok(())
    }
}

/// Resolved filter specification with validated state references
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedFilterSpec {
    /// Filter action (include/exclude)
    pub action: FilterAction,
    /// Validated state references (guaranteed to exist as global states)
    pub state_refs: Vec<String>,
}

impl ResolvedFilterSpec {
    /// Create a new resolved filter specification
    pub fn new(action: FilterAction, state_refs: Vec<String>) -> Self {
        Self { action, state_refs }
    }

    /// Check if this filter should include items that satisfy the states
    pub fn should_include_on_satisfaction(&self) -> bool {
        matches!(self.action, FilterAction::Include)
    }

    /// Check if this filter should exclude items that satisfy the states
    pub fn should_exclude_on_satisfaction(&self) -> bool {
        matches!(self.action, FilterAction::Exclude)
    }
}

/// Filter actions for include/exclude logic
/// EBNF: filter_action ::= "include" | "exclude"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterAction {
    Include, // include - include items that satisfy the referenced states
    Exclude, // exclude - exclude items that satisfy the referenced states
}

impl FilterAction {
    /// Parse filter action from string (case-sensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "include" | "Include" => Some(FilterAction::Include),
            "exclude" | "Exclude" => Some(FilterAction::Exclude),
            _ => None,
        }
    }

    /// Get the action as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Include => "include",
            Self::Exclude => "exclude",
        }
    }

    /// Get the opposite action
    pub fn invert(&self) -> Self {
        match self {
            Self::Include => Self::Exclude,
            Self::Exclude => Self::Include,
        }
    }

    /// Check if this action includes on state satisfaction
    pub fn includes_on_match(&self) -> bool {
        matches!(self, Self::Include)
    }
}

/// Filter reference for referencing global states in filters
/// EBNF: filter_references ::= state_reference+
/// Note: This is essentially the same as StateRef but used in filter context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterStateRef {
    /// Referenced state ID (must be global)
    pub state_id: String,
}

impl FilterStateRef {
    /// Create a new filter state reference
    pub fn new(state_id: impl Into<String>) -> Self {
        Self {
            state_id: state_id.into(),
        }
    }
}

/// Filter application context for execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterContext {
    /// Filter applied to object collection
    ObjectFilter,
    /// Filter applied to set operands
    SetFilter,
}

/// Filter evaluation result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterResult {
    /// Item should be included
    Include,
    /// Item should be excluded
    Exclude,
    /// Cannot determine (e.g., state evaluation failed)
    Unknown,
}

impl FilterResult {
    /// Check if this result allows the item to be included
    pub fn allows_inclusion(&self) -> bool {
        matches!(self, Self::Include)
    }

    /// Check if this result requires exclusion
    pub fn requires_exclusion(&self) -> bool {
        matches!(self, Self::Exclude)
    }

    /// Check if the result is indeterminate
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

// Display implementations
impl std::fmt::Display for FilterAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for FilterSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FILTER {} [{}]", self.action, self.state_refs.join(", "))
    }
}

impl std::fmt::Display for FilterResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Include => write!(f, "include"),
            Self::Exclude => write!(f, "exclude"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Filter evaluation logic helpers
impl FilterSpec {
    /// Determine filter result based on state satisfaction and action
    pub fn evaluate(&self, states_satisfied: bool) -> FilterResult {
        match (self.action, states_satisfied) {
            (FilterAction::Include, true) => FilterResult::Include,
            (FilterAction::Include, false) => FilterResult::Exclude,
            (FilterAction::Exclude, true) => FilterResult::Exclude,
            (FilterAction::Exclude, false) => FilterResult::Include,
        }
    }

    /// Get the default result when state evaluation is impossible
    pub fn default_result(&self) -> FilterResult {
        // Conservative approach: if we can't evaluate, default based on action
        match self.action {
            FilterAction::Include => FilterResult::Exclude, // Conservative: exclude if unsure
            FilterAction::Exclude => FilterResult::Include, // Conservative: include if unsure
        }
    }
}

/// Filter dependency tracking for DAG construction
pub trait FilterDependencies {
    /// Get all state dependencies from filters
    fn get_filter_dependencies(&self) -> Vec<String>;
}

impl FilterDependencies for FilterSpec {
    fn get_filter_dependencies(&self) -> Vec<String> {
        self.state_refs.clone()
    }
}

impl FilterDependencies for Vec<FilterSpec> {
    fn get_filter_dependencies(&self) -> Vec<String> {
        let mut deps: Vec<String> = self
            .iter()
            .flat_map(|filter| filter.state_refs.iter().cloned())
            .collect();
        deps.sort();
        deps.dedup();
        deps
    }
}
