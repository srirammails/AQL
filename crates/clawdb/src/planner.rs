//! Query planner for ClawDB
//!
//! Currently a placeholder - direct execution is used.
//! Future: query optimization, index selection, etc.

use aql_parser::Statement;

/// Query plan (placeholder)
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub statement: Statement,
}

impl QueryPlan {
    /// Create a plan from a statement
    pub fn from_statement(statement: Statement) -> Self {
        Self { statement }
    }
}
