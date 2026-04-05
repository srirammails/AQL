//! AST types for AQL (Agent Query Language)
//!
//! These types represent the parsed structure of AQL statements.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A complete AQL statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// PIPELINE name TIMEOUT duration stage | stage | ...
    Pipeline(PipelineStmt),
    /// REFLECT FROM ... WITH LINKS ... THEN ...
    Reflect(ReflectStmt),
    /// SCAN FROM WORKING ...
    Scan(ScanStmt),
    /// RECALL FROM memory_type ...
    Recall(RecallStmt),
    /// LOOKUP FROM memory_type ...
    Lookup(LookupStmt),
    /// LOAD FROM TOOLS ...
    Load(LoadStmt),
    /// STORE INTO memory_type (...)
    Store(StoreStmt),
    /// UPDATE INTO memory_type WHERE ... (...)
    Update(UpdateStmt),
    /// FORGET FROM memory_type WHERE ...
    Forget(ForgetStmt),
    /// LINK FROM ... TO ... TYPE ...
    Link(LinkStmt),
}

/// Memory type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoryType {
    Working,
    Tools,
    Procedural,
    Semantic,
    Episodic,
    All,
}

impl MemoryType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "WORKING" => Some(Self::Working),
            "TOOLS" => Some(Self::Tools),
            "PROCEDURAL" => Some(Self::Procedural),
            "SEMANTIC" => Some(Self::Semantic),
            "EPISODIC" => Some(Self::Episodic),
            "ALL" => Some(Self::All),
            _ => None,
        }
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Working => write!(f, "WORKING"),
            Self::Tools => write!(f, "TOOLS"),
            Self::Procedural => write!(f, "PROCEDURAL"),
            Self::Semantic => write!(f, "SEMANTIC"),
            Self::Episodic => write!(f, "EPISODIC"),
            Self::All => write!(f, "ALL"),
        }
    }
}

/// PIPELINE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineStmt {
    /// Pipeline name
    pub name: String,
    /// Optional timeout
    pub timeout: Option<Duration>,
    /// Pipeline stages
    pub stages: Vec<Statement>,
}

/// REFLECT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectStmt {
    /// Sources to reflect from
    pub sources: Vec<ReflectSource>,
    /// WITH LINKS modifier
    pub with_links: Option<WithLinks>,
    /// FOLLOW LINKS modifier
    pub follow_links: Option<FollowLinks>,
    /// THEN clause (optional store/update)
    pub then_clause: Option<Box<Statement>>,
}

/// Source for REFLECT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectSource {
    /// Memory type
    pub memory_type: MemoryType,
    /// Optional predicate
    pub predicate: Option<Predicate>,
}

/// SCAN statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanStmt {
    /// Window modifier (SCAN only supports WORKING)
    pub window: Option<Window>,
    /// Other modifiers
    pub modifiers: Modifiers,
}

/// RECALL statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallStmt {
    /// Memory type to recall from
    pub memory_type: MemoryType,
    /// Predicate
    pub predicate: Predicate,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// LOOKUP statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LookupStmt {
    /// Memory type to lookup from
    pub memory_type: MemoryType,
    /// Predicate
    pub predicate: Predicate,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// LOAD statement (TOOLS only)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoadStmt {
    /// Predicate
    pub predicate: Predicate,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// STORE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreStmt {
    /// Memory type to store into
    pub memory_type: MemoryType,
    /// Payload (field assignments)
    pub payload: Vec<FieldAssignment>,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStmt {
    /// Memory type to update
    pub memory_type: MemoryType,
    /// WHERE conditions
    pub conditions: Vec<Condition>,
    /// Payload (field assignments)
    pub payload: Vec<FieldAssignment>,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// FORGET statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgetStmt {
    /// Memory type to forget from
    pub memory_type: MemoryType,
    /// WHERE conditions
    pub conditions: Vec<Condition>,
    /// Modifiers
    pub modifiers: Modifiers,
}

/// LINK statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkStmt {
    /// Source memory type
    pub from_type: MemoryType,
    /// Source conditions
    pub from_conditions: Vec<Condition>,
    /// Target memory type
    pub to_type: MemoryType,
    /// Target conditions
    pub to_conditions: Vec<Condition>,
    /// Link type
    pub link_type: String,
    /// Link weight
    pub weight: Option<f32>,
}

/// Query predicate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Predicate {
    /// WHERE condition AND condition ...
    Where { conditions: Vec<Condition> },
    /// KEY field = value
    Key { field: String, value: Value },
    /// LIKE $embedding_var
    Like { variable: String },
    /// PATTERN $pattern_var THRESHOLD 0.7
    Pattern {
        variable: String,
        threshold: Option<f32>,
    },
    /// ALL (match everything)
    All,
}

/// Logical operators for combining conditions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOp {
    #[default]
    And,
    Or,
}

impl LogicalOp {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "AND" => Some(Self::And),
            "OR" => Some(Self::Or),
            _ => None,
        }
    }
}

/// A condition in WHERE clause - can be simple or grouped
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    /// Simple condition: field op value
    Simple {
        field: String,
        operator: Operator,
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        logical_op: Option<LogicalOp>,
    },
    /// Grouped conditions (parenthesized) - evaluated as a unit
    Group {
        conditions: Vec<Condition>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logical_op: Option<LogicalOp>,
    },
}

impl Condition {
    /// Create a simple condition
    pub fn simple(field: String, operator: Operator, value: Value) -> Self {
        Condition::Simple {
            field,
            operator,
            value,
            logical_op: None,
        }
    }

    /// Create a group condition
    pub fn group(conditions: Vec<Condition>) -> Self {
        Condition::Group {
            conditions,
            logical_op: None,
        }
    }

    /// Get the logical operator
    pub fn logical_op(&self) -> Option<LogicalOp> {
        match self {
            Condition::Simple { logical_op, .. } => *logical_op,
            Condition::Group { logical_op, .. } => *logical_op,
        }
    }

    /// Set the logical operator
    pub fn set_logical_op(&mut self, op: Option<LogicalOp>) {
        match self {
            Condition::Simple { logical_op, .. } => *logical_op = op,
            Condition::Group { logical_op, .. } => *logical_op = op,
        }
    }

    /// Get field name (only for Simple conditions)
    pub fn field(&self) -> Option<&str> {
        match self {
            Condition::Simple { field, .. } => Some(field),
            Condition::Group { .. } => None,
        }
    }

    /// Get operator (only for Simple conditions)
    pub fn operator(&self) -> Option<Operator> {
        match self {
            Condition::Simple { operator, .. } => Some(*operator),
            Condition::Group { .. } => None,
        }
    }

    /// Get value (only for Simple conditions)
    pub fn value(&self) -> Option<&Value> {
        match self {
            Condition::Simple { value, .. } => Some(value),
            Condition::Group { .. } => None,
        }
    }
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    StartsWith,
    EndsWith,
    In,
}

impl Operator {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "=" => Some(Self::Eq),
            "!=" | "<>" => Some(Self::Ne),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Gte),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Lte),
            s if s.eq_ignore_ascii_case("CONTAINS") => Some(Self::Contains),
            s if s.eq_ignore_ascii_case("STARTS_WITH") => Some(Self::StartsWith),
            s if s.eq_ignore_ascii_case("ENDS_WITH") => Some(Self::EndsWith),
            s if s.eq_ignore_ascii_case("IN") => Some(Self::In),
            _ => None,
        }
    }
}

/// A typed value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Variable(String),
    Array(Vec<Value>),
}

impl Value {
    /// Convert to JSON value
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(*b),
            Self::Int(i) => serde_json::Value::Number((*i).into()),
            Self::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Self::String(s) => serde_json::Value::String(s.clone()),
            Self::Variable(v) => serde_json::Value::String(format!("${}", v)),
            Self::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect())
            }
        }
    }
}

/// Field assignment (for STORE/UPDATE)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAssignment {
    /// Field name
    pub field: String,
    /// Value
    pub value: Value,
}

/// Query modifiers
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Modifiers {
    /// LIMIT n
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    /// ORDER BY field ASC|DESC
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<OrderBy>,
    /// RETURN field1, field2, ...
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_fields: Option<Vec<String>>,
    /// TIMEOUT duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,
    /// MIN_CONFIDENCE float
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_confidence: Option<f32>,
    /// SCOPE private|shared|cluster
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    /// NAMESPACE "name"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// TTL duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<Duration>,
    /// AGGREGATE functions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<Vec<AggregateFunc>>,
    /// HAVING conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub having: Option<Vec<Condition>>,
    /// WITH LINKS modifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_links: Option<WithLinks>,
    /// FOLLOW LINKS modifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_links: Option<FollowLinks>,
    /// WINDOW modifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<Window>,
}

/// ORDER BY specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBy {
    /// Field to order by
    pub field: String,
    /// True for ascending, false for descending
    pub ascending: bool,
}

/// Scope values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Private,
    Shared,
    Cluster,
}

/// Aggregate function
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateFunc {
    /// Function name
    pub func: AggregateFuncType,
    /// Field (None for COUNT(*))
    pub field: Option<String>,
    /// Alias
    pub alias: Option<String>,
}

/// Aggregate function types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AggregateFuncType {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggregateFuncType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "COUNT" => Some(Self::Count),
            "SUM" => Some(Self::Sum),
            "AVG" => Some(Self::Avg),
            "MIN" => Some(Self::Min),
            "MAX" => Some(Self::Max),
            _ => None,
        }
    }
}

/// WITH LINKS modifier
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WithLinks {
    /// Include all links
    All,
    /// Include links of specific type
    Type { link_type: String },
}

/// FOLLOW LINKS modifier
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FollowLinks {
    /// Link type to follow
    pub link_type: String,
    /// Maximum depth
    pub depth: Option<u32>,
}

/// Window specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Window {
    /// WINDOW LAST n
    LastN { count: usize },
    /// WINDOW LAST duration
    LastDuration { duration: Duration },
    /// WINDOW TOP n BY field
    TopBy { count: usize, field: String },
    /// WINDOW SINCE condition
    Since { condition: Condition },
}
