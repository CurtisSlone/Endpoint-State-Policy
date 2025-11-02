use serde::{Deserialize, Serialize};

/// Test specification for CTN blocks
/// EBNF: test_specification ::= "TEST" space existence_check space item_check (space state_operator)? statement_end
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestSpecification {
    /// How to check for existence of items
    pub existence_check: ExistenceCheck,
    /// How to validate items against states
    pub item_check: ItemCheck,
    /// Optional operator for combining multiple states
    pub state_operator: Option<StateJoinOp>,
}

impl TestSpecification {
    /// Create a new test specification
    pub fn new(
        existence_check: ExistenceCheck,
        item_check: ItemCheck,
        state_operator: Option<StateJoinOp>,
    ) -> Self {
        Self {
            existence_check,
            item_check,
            state_operator,
        }
    }

    /// Create a simple test specification without state operator
    pub fn simple(existence_check: ExistenceCheck, item_check: ItemCheck) -> Self {
        Self {
            existence_check,
            item_check,
            state_operator: None,
        }
    }

    /// Check if this test has a state operator
    pub fn has_state_operator(&self) -> bool {
        self.state_operator.is_some()
    }
}

/// Existence check options for test specifications
/// EBNF: existence_check ::= "any" | "all" | "none" | "at_least_one" | "only_one"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExistenceCheck {
    Any,        // any
    All,        // all
    None,       // none
    AtLeastOne, // at_least_one
    OnlyOne,    // only_one
}

impl ExistenceCheck {
    /// Parse existence check from string (exact match, case-sensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // ICS syntax (from original file)
            "all" => Some(ExistenceCheck::All),
            "any" => Some(ExistenceCheck::Any),
            "none" => Some(ExistenceCheck::None),
            "at_least_one" => Some(ExistenceCheck::AtLeastOne),

            // JSON/Enum style (from parsed AST)
            "All" => Some(ExistenceCheck::All),
            "Any" => Some(ExistenceCheck::Any),
            "None" => Some(ExistenceCheck::None),
            "AtLeastOne" => Some(ExistenceCheck::AtLeastOne),
            "OnlyOne" => Some(ExistenceCheck::OnlyOne),

            _ => None,
        }
    }

    /// Get the check as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::All => "all",
            Self::None => "none",
            Self::AtLeastOne => "at_least_one",
            Self::OnlyOne => "only_one",
        }
    }

    /// Check if this existence check expects items to be found
    pub fn expects_items(&self) -> bool {
        match self {
            Self::Any | Self::All | Self::AtLeastOne | Self::OnlyOne => true,
            Self::None => false,
        }
    }

    /// Check if this existence check requires all items to be found
    pub fn requires_all_items(&self) -> bool {
        matches!(self, Self::All)
    }
}

/// Item check options for test specifications
/// EBNF: item_check ::= "all" | "at_least_one" | "only_one" | "none_satisfy"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemCheck {
    All,         // all
    AtLeastOne,  // at_least_one
    OnlyOne,     // only_one
    NoneSatisfy, // none_satisfy
}

impl ItemCheck {
    /// Parse item check from string (exact match, case-sensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "all" => Some(Self::All),
            "at_least_one" => Some(Self::AtLeastOne),
            "only_one" => Some(Self::OnlyOne),
            "none_satisfy" => Some(Self::NoneSatisfy),
            "All" => Some(Self::All),
            "AtLeastOne" => Some(Self::AtLeastOne),
            "OnlyOne" => Some(Self::OnlyOne),
            "NoneSatisfy" => Some(Self::NoneSatisfy),
            _ => None,
        }
    }

    /// Get the check as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::AtLeastOne => "at_least_one",
            Self::OnlyOne => "only_one",
            Self::NoneSatisfy => "none_satisfy",
        }
    }

    /// Check if this item check expects items to satisfy states
    pub fn expects_satisfaction(&self) -> bool {
        match self {
            Self::All | Self::AtLeastOne | Self::OnlyOne => true,
            Self::NoneSatisfy => false,
        }
    }

    /// Check if this item check requires all found items to satisfy
    pub fn requires_all_items(&self) -> bool {
        matches!(self, Self::All)
    }
}

/// State join operators for test specifications
/// EBNF: state_operator ::= "AND" | "OR" | "ONE"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateJoinOp {
    And, // AND - all states must pass
    Or,  // OR - at least one state must pass
    One, // ONE - exactly one state must pass
}

impl StateJoinOp {
    /// Parse state join operator from string (case-sensitive, uppercase)
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "AND" => Some(Self::And),
            "OR" => Some(Self::Or),
            "ONE" => Some(Self::One),
            "And" => Some(Self::And),
            "Or" => Some(Self::Or),
            "One" => Some(Self::One),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "one" => Some(Self::One),
            _ => None,
        }
    }

    /// Get the operator as it appears in ICS source
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
            Self::One => "ONE",
        }
    }

    /// Check if this operator requires all states to pass
    pub fn requires_all_states(&self) -> bool {
        matches!(self, Self::And)
    }

    /// Check if this operator allows multiple states to pass
    pub fn allows_multiple_states(&self) -> bool {
        match self {
            Self::And | Self::Or => true,
            Self::One => false,
        }
    }
}

// Display implementations for easy debugging and logging
impl std::fmt::Display for ExistenceCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for ItemCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for StateJoinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::fmt::Display for TestSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(state_op) = &self.state_operator {
            write!(
                f,
                "TEST {} {} {}",
                self.existence_check, self.item_check, state_op
            )
        } else {
            write!(f, "TEST {} {}", self.existence_check, self.item_check)
        }
    }
}

/// Validation logic for test specifications
impl TestSpecification {
    /// Validate that this test specification is logically consistent
    pub fn validate(&self) -> Result<(), String> {
        // If no items are expected to be found, item validation is irrelevant
        if !self.existence_check.expects_items() {
            if self.item_check.expects_satisfaction() {
                return Err(format!(
                    "Inconsistent test: existence_check '{}' expects no items, but item_check '{}' expects validation",
                    self.existence_check, self.item_check
                ));
            }
        }

        // State operator without multiple states doesn't make sense
        if self.state_operator.is_some() {
            // This would be validated at runtime when we know how many states are referenced
            // For now, just ensure the combination is theoretically valid
        }

        Ok(())
    }

    /// Check if this test specification is suitable for the given number of states
    pub fn is_suitable_for_state_count(&self, state_count: usize) -> bool {
        match self.state_operator {
            None => state_count <= 1, // No operator means single state or no states
            Some(StateJoinOp::And) | Some(StateJoinOp::Or) => state_count >= 2, // Requires multiple states
            Some(StateJoinOp::One) => state_count >= 2, // ONE requires multiple states to choose from
        }
    }
}
