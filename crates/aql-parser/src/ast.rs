//! AST types for AQL v0.13 (`spec/AQL_SPEC_v0.13.md`).
//!
//! 13 verbs: RECALL, STORE, UPDATE, FORGET, LINK, UNLINK (memory); BRANCH, FORK,
//! MERGE (namespaces); PIPELINE, SUBSCRIBE, UNSUBSCRIBE, PUBLISH (flow).

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A complete AQL statement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// USE "ns": the namespace of the following statements that name none
    Use(String),
    /// PIPELINE [name] [TIMEOUT d] stage | stage | …
    Pipeline(PipelineStmt),
    /// RECALL FROM <memory> [WHERE … | LIKE …] … — every read
    Recall(RecallStmt),
    /// STORE INTO <memory> (…) …
    Store(StoreStmt),
    /// UPDATE INTO <memory> WHERE … (…) …
    Update(UpdateStmt),
    /// FORGET FROM <memory> [WHERE …] …
    Forget(ForgetStmt),
    /// LINK FROM … INTO … TYPE "t" [WEIGHT w] — create, or set the weight
    Link(LinkStmt),
    /// UNLINK FROM … INTO … [TYPE "t"]
    Unlink(UnlinkStmt),
    /// BRANCH FROM "ns" INTO "ns2" [AS OF t]
    Branch(NamespaceStmt),
    /// FORK FROM "ns" INTO "ns2" [AS OF t]
    Fork(NamespaceStmt),
    /// MERGE FROM "ns2" INTO "ns" [PREFER SOURCE|TARGET]
    Merge(MergeStmt),
    /// SUBSCRIBE TOPIC "t" … | SUBSCRIBE RECALL …
    Subscribe(SubscribeStmt),
    /// UNSUBSCRIBE "id"
    Unsubscribe(UnsubscribeStmt),
    /// PUBLISH TOPIC "t" (…) …
    Publish(PublishStmt),
}

impl Statement {
    /// `self` with `namespace` wherever it names none (a USE in force). BRANCH, FORK,
    /// MERGE and UNSUBSCRIBE name their own, and are unchanged.
    pub fn with_default_namespace(mut self, namespace: &str) -> Self {
        fn fill(slot: &mut Option<String>, ns: &str) {
            if slot.is_none() {
                *slot = Some(ns.to_string());
            }
        }
        // A read that names a list of namespaces names them all.
        fn fill_read(m: &mut Modifiers, ns: &str) {
            if m.namespaces.is_empty() {
                fill(&mut m.namespace, ns);
            }
        }
        match &mut self {
            Statement::Pipeline(p) => {
                p.stages = std::mem::take(&mut p.stages).into_iter().map(|s| s.with_default_namespace(namespace)).collect();
            }
            Statement::Recall(r) => fill_read(&mut r.modifiers, namespace),
            Statement::Store(s) => fill(&mut s.modifiers.namespace, namespace),
            Statement::Update(u) => fill(&mut u.modifiers.namespace, namespace),
            Statement::Forget(f) => fill(&mut f.modifiers.namespace, namespace),
            Statement::Link(l) => fill(&mut l.namespace, namespace),
            Statement::Unlink(u) => fill(&mut u.namespace, namespace),
            Statement::Publish(p) => fill(&mut p.modifiers.namespace, namespace),
            Statement::Subscribe(s) => match &mut s.target {
                SubscribeTarget::Query(r) => fill_read(&mut r.modifiers, namespace),
                SubscribeTarget::Topic(_) => fill(&mut s.namespace, namespace),
            },
            Statement::Use(_) | Statement::Branch(_) | Statement::Fork(_) | Statement::Merge(_) | Statement::Unsubscribe(_) => {}
        }
        self
    }
}

/// Memory types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// PIPELINE: stages in order within an optional time budget; the last may write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineStmt {
    /// Pipeline name (empty when anonymous)
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Duration>,
    pub stages: Vec<Statement>,
}

/// RECALL — every read. The engine picks the access path from what is asked: an
/// exact lookup (`WHERE key = …`), a filtered read, a similarity search (`LIKE`), a
/// tool ranking (`FROM TOOLS`), the session's state (`FROM WORKING`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecallStmt {
    pub memory_type: MemoryType,
    pub predicate: Predicate,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoreStmt {
    pub memory_type: MemoryType,
    pub payload: Vec<FieldAssignment>,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStmt {
    pub memory_type: MemoryType,
    pub conditions: Vec<Condition>,
    pub payload: Vec<FieldAssignment>,
    pub modifiers: Modifiers,
}

/// FORGET; no conditions forgets everything in the memory type (and namespace).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForgetStmt {
    pub memory_type: MemoryType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<Condition>,
    pub modifiers: Modifiers,
}

/// LINK: a link of `link_type` between the matched records (create, or set the
/// weight of the one that exists).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkStmt {
    pub from_type: MemoryType,
    pub from_conditions: Vec<Condition>,
    pub to_type: MemoryType,
    pub to_conditions: Vec<Condition>,
    pub link_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnlinkStmt {
    pub from_type: MemoryType,
    pub from_conditions: Vec<Condition>,
    pub to_type: MemoryType,
    pub to_conditions: Vec<Condition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

/// BRANCH / FORK: `into` is a new namespace from `from` at a point (now, or `as_of`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamespaceStmt {
    pub from: String,
    pub into: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<TimePoint>,
}

/// MERGE: `from`'s changes since the two parted, into `into`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeStmt {
    pub from: String,
    pub into: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer: Option<Prefer>,
}

/// Which side wins a MERGE conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Prefer {
    Source,
    Target,
}

/// What a subscription follows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscribeTarget {
    /// A topic's messages
    Topic(String),
    /// A live query: the RECALL's result as it changes
    Query(Box<RecallStmt>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribeStmt {
    pub target: SubscribeTarget,
    /// A topic subscription's WHERE
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<Vec<Condition>>,
    /// A topic subscription's namespace (a query's is in its modifiers)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsubscribeStmt {
    pub subscription: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublishStmt {
    pub topic: String,
    pub payload: Vec<FieldAssignment>,
    pub modifiers: Modifiers,
}

/// What a RECALL matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Predicate {
    /// WHERE conditions
    Where { conditions: Vec<Condition> },
    /// LIKE $var or LIKE "text": similarity. `variable` names the variable (or repeats
    /// the text); `text` holds a literal.
    Like {
        variable: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    /// No WHERE: everything
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
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "AND" => Some(Self::And),
            "OR" => Some(Self::Or),
            _ => None,
        }
    }
}

/// A condition in a WHERE clause: simple or a parenthesized group. A condition's
/// `logical_op` joins it to the one before.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Simple {
        field: String,
        operator: Operator,
        value: Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        logical_op: Option<LogicalOp>,
    },
    Group {
        conditions: Vec<Condition>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logical_op: Option<LogicalOp>,
    },
}

impl Condition {
    pub fn simple(field: String, operator: Operator, value: Value) -> Self {
        Condition::Simple { field, operator, value, logical_op: None }
    }

    pub fn group(conditions: Vec<Condition>) -> Self {
        Condition::Group { conditions, logical_op: None }
    }

    pub fn logical_op(&self) -> Option<LogicalOp> {
        match self {
            Condition::Simple { logical_op, .. } | Condition::Group { logical_op, .. } => *logical_op,
        }
    }

    pub fn set_logical_op(&mut self, op: Option<LogicalOp>) {
        match self {
            Condition::Simple { logical_op, .. } | Condition::Group { logical_op, .. } => *logical_op = op,
        }
    }

    pub fn field(&self) -> Option<&str> {
        match self {
            Condition::Simple { field, .. } => Some(field),
            Condition::Group { .. } => None,
        }
    }

    pub fn operator(&self) -> Option<Operator> {
        match self {
            Condition::Simple { operator, .. } => Some(*operator),
            Condition::Group { .. } => None,
        }
    }

    pub fn value(&self) -> Option<&Value> {
        match self {
            Condition::Simple { value, .. } => Some(value),
            Condition::Group { .. } => None,
        }
    }
}

/// Comparison operators. `Between`'s value is a two-element array; `In`'s a list.
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
    Between,
}

impl Operator {
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
            s if s.eq_ignore_ascii_case("BETWEEN") => Some(Self::Between),
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
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(b) => serde_json::Value::Bool(*b),
            Self::Int(i) => serde_json::Value::Number((*i).into()),
            Self::Float(f) => serde_json::Number::from_f64(*f).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null),
            Self::String(s) => serde_json::Value::String(s.clone()),
            Self::Variable(v) => serde_json::Value::String(format!("${}", v)),
            Self::Array(arr) => serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect()),
        }
    }
}

/// Field assignment (STORE / UPDATE / PUBLISH)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldAssignment {
    pub field: String,
    pub value: Value,
}

/// Modifiers (spec §8).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Modifiers {
    /// NAMESPACE "ns"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// NAMESPACE ("a", "b"): a read across these namespaces (two or more; RECALL only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub namespaces: Vec<String>,
    /// IN SCOPE "team:billing": a shared group within the namespace
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_scope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_by: Option<OrderBy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_fields: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<Duration>,
    /// CONFIDENCE c: at least this similar (LIKE)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<Vec<AggregateFunc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub having: Option<Vec<Condition>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<Window>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_links: Option<LinkSelect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub follow_links: Option<FollowLinks>,
    /// AS OF t: the records as they were at t
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of: Option<TimePoint>,
    /// WITH HISTORY [BETWEEN t1 AND t2]: every version (recorded in the range)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_history: Option<History>,
}

/// A point in time: an RFC 3339 timestamp or date string, or a variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TimePoint {
    At { time: String },
    Variable { name: String },
}

/// WITH HISTORY, optionally limited to versions recorded between two times.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub between: Option<(TimePoint, TimePoint)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBy {
    pub field: String,
    pub ascending: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateFunc {
    pub func: AggregateFuncType,
    /// None for COUNT(*)
    pub field: Option<String>,
    pub alias: Option<String>,
}

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

/// Which links WITH LINKS shows (both directions): any type, or these types; at
/// least `min_weight`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkSelect {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_weight: Option<f64>,
}

/// FOLLOW LINKS: the records reached through the selected links, up to `depth` hops.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FollowLinks {
    #[serde(flatten)]
    pub select: LinkSelect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

/// WINDOW LAST n | WINDOW LAST duration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Window {
    LastN { count: usize },
    LastDuration { duration: Duration },
}
