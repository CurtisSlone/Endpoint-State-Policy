use serde::{Deserialize, Serialize};

// For arithmetic in RUN operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArithmeticOperator {
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Modulus,  // %
}

// For comparison operations in states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,             // =
    NotEquals,          // !=
    GreaterThan,        // >
    LessThan,           //
    GreaterThanOrEqual, // >=
    LessThanOrEqual,    // <=
}

// For string operations in states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StringOperator {
    CaseInsensitiveEquals,    // ieq
    CaseInsensitiveNotEquals, // ine
    Contains,                 // contains
    StartsWith,               // starts
    EndsWith,                 // ends
    NotContains,              // not_contains
    NotStartsWith,            // not_starts
    NotEndsWith,              // not_ends
}
