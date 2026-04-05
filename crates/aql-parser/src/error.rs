//! Parse error types for AQL

use thiserror::Error;

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    /// Pest grammar error
    #[error("Parse error: {0}")]
    Pest(String),

    /// Empty input or missing expected content
    #[error("Empty or missing content")]
    Empty,

    /// Missing required element
    #[error("Missing required element: {0}")]
    Missing(&'static str),

    /// Unexpected grammar rule encountered
    #[error("Unexpected rule: {0}")]
    UnexpectedRule(String),

    /// Invalid memory type
    #[error("Invalid memory type: {0}")]
    InvalidMemoryType(String),

    /// Invalid operator
    #[error("Invalid operator: {0}")]
    InvalidOperator(String),

    /// Invalid float literal
    #[error("Invalid float: {0}")]
    InvalidFloat(String),

    /// Invalid integer literal
    #[error("Invalid integer: {0}")]
    InvalidInteger(String),

    /// Invalid duration literal
    #[error("Invalid duration: {0}")]
    InvalidDuration(String),

    /// Invalid scope value
    #[error("Invalid scope: {0}")]
    InvalidScope(String),

    /// Invalid aggregate function
    #[error("Invalid aggregate function: {0}")]
    InvalidAggregateFunc(String),
}

impl ParseError {
    /// Create a ParseError from a pest error
    pub fn from_pest<R: pest::RuleType>(err: pest::error::Error<R>) -> Self {
        ParseError::Pest(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ParseError::Missing("field");
        assert_eq!(format!("{}", err), "Missing required element: field");

        let err = ParseError::InvalidMemoryType("UNKNOWN".to_string());
        assert_eq!(format!("{}", err), "Invalid memory type: UNKNOWN");
    }

    #[test]
    fn test_error_empty() {
        let err = ParseError::Empty;
        assert_eq!(format!("{}", err), "Empty or missing content");
    }
}
