//! Core SDK validation rules - not user-configurable
//! These define the fundamental operation compatibility for the ICS system

use crate::types::common::{DataType, Operation};

/// Core SDK operation compatibility rules
/// These are immutable and define what operations are fundamentally valid
impl DataType {
    /// Get the SDK-defined valid operations for this data type
    /// This is not user-configurable - it defines core ICS semantics
    pub fn sdk_valid_operations(&self) -> Vec<Operation> {
        use Operation::*;
        match self {
            DataType::String => vec![
                Equals,
                NotEqual,
                Contains,
                NotContains,
                StartsWith,
                EndsWith,
                PatternMatch,
            ],
            DataType::Int | DataType::Float => vec![
                Equals,
                NotEqual,
                GreaterThan,
                LessThan,
                GreaterThanOrEqual,
                LessThanOrEqual,
            ],
            DataType::Boolean => vec![Equals, NotEqual],
            DataType::Version | DataType::EvrString => vec![
                Equals,
                NotEqual,
                GreaterThan,
                LessThan,
                GreaterThanOrEqual,
                LessThanOrEqual,
            ],
            DataType::Binary => vec![Equals, NotEqual],
            DataType::RecordData => vec![
                Equals,
                NotEqual, // RecordData might support additional operations like Contains
                         // for field-level searches, but start conservative
            ],
        }
    }

    /// Check if an operation is valid according to core SDK rules
    /// This validation cannot be overridden by users
    pub fn sdk_supports_operation(&self, operation: &Operation) -> bool {
        self.sdk_valid_operations().contains(operation)
    }
}
