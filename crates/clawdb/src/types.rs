//! Core types for ClawDB

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Memory type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MemoryType {
    Working,
    Episodic,
    Procedural,
    Tools,
    Semantic,
}

impl MemoryType {
    /// Get the LanceDB table name for this memory type
    pub fn table_name(&self) -> &'static str {
        match self {
            Self::Working => "working",
            Self::Episodic => "episodic",
            Self::Procedural => "procedural",
            Self::Tools => "tools",
            Self::Semantic => "semantic",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "WORKING" => Some(Self::Working),
            "EPISODIC" => Some(Self::Episodic),
            "PROCEDURAL" => Some(Self::Procedural),
            "TOOLS" => Some(Self::Tools),
            "SEMANTIC" => Some(Self::Semantic),
            _ => None,
        }
    }
}

impl From<aql_parser::MemoryType> for MemoryType {
    fn from(mt: aql_parser::MemoryType) -> Self {
        match mt {
            aql_parser::MemoryType::Working => Self::Working,
            aql_parser::MemoryType::Episodic => Self::Episodic,
            aql_parser::MemoryType::Procedural => Self::Procedural,
            aql_parser::MemoryType::Tools => Self::Tools,
            aql_parser::MemoryType::Semantic => Self::Semantic,
            aql_parser::MemoryType::All => Self::Working, // Default fallback
        }
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Working => write!(f, "WORKING"),
            Self::Episodic => write!(f, "EPISODIC"),
            Self::Procedural => write!(f, "PROCEDURAL"),
            Self::Tools => write!(f, "TOOLS"),
            Self::Semantic => write!(f, "SEMANTIC"),
        }
    }
}

/// Metadata for a memory record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Namespace for isolation
    pub namespace: String,
    /// Scope (private, shared, cluster)
    pub scope: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last access timestamp
    pub accessed_at: DateTime<Utc>,
    /// Version number
    pub version: i64,
    /// Time-to-live in milliseconds (None = no expiry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<i64>,
}

impl Default for Metadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            namespace: "default".to_string(),
            scope: "private".to_string(),
            created_at: now,
            accessed_at: now,
            version: 1,
            ttl_ms: None,
        }
    }
}

impl Metadata {
    /// Create new metadata with the given namespace
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            ..Default::default()
        }
    }

    /// Check if the record is expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_ms {
            let expires_at = self.created_at + chrono::Duration::milliseconds(ttl);
            Utc::now() > expires_at
        } else {
            false
        }
    }
}

/// A memory record stored in LanceDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Unique identifier
    pub id: String,
    /// Memory type
    pub memory_type: MemoryType,
    /// Record data (JSON)
    pub data: Value,
    /// Metadata
    pub metadata: Metadata,
    /// Optional embedding vector for semantic memory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl MemoryRecord {
    /// Create a new memory record
    pub fn new(memory_type: MemoryType, data: Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            memory_type,
            data,
            metadata: Metadata::default(),
            embedding: None,
        }
    }

    /// Set the namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.metadata.namespace = namespace.into();
        self
    }

    /// Set the scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.metadata.scope = scope.into();
        self
    }

    /// Set the TTL
    pub fn with_ttl_ms(mut self, ttl_ms: i64) -> Self {
        self.metadata.ttl_ms = Some(ttl_ms);
        self
    }

    /// Set the embedding
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Get a field value from the record data (cloned)
    pub fn get_field(&self, path: &str) -> Option<Value> {
        self.resolve_field(path)
    }

    /// Resolve a field path to a JSON value (for condition matching)
    pub fn resolve_field(&self, path: &str) -> Option<Value> {
        // Handle special fields
        match path {
            "id" => return Some(Value::String(self.id.clone())),
            "memory_type" => return Some(Value::String(self.memory_type.to_string())),
            _ => {}
        }

        // Handle metadata fields
        if path == "namespace" || path == "metadata.namespace" {
            return Some(Value::String(self.metadata.namespace.clone()));
        }
        if path == "scope" || path == "metadata.scope" {
            return Some(Value::String(self.metadata.scope.clone()));
        }
        if path == "created_at" || path == "metadata.created_at" {
            return Some(Value::Number(self.metadata.created_at.timestamp_millis().into()));
        }

        // Handle data fields
        let field_path = if path.starts_with("data.") {
            &path[5..]
        } else {
            path
        };

        get_nested_field(&self.data, field_path).cloned()
    }
}

/// Get a nested field from a JSON value using dot notation
fn get_nested_field<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

/// Query result from ClawDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Whether the query succeeded
    pub success: bool,
    /// Result data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<QueryData>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution metadata
    pub metadata: QueryMetadata,
}

/// Query result data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueryData {
    /// Single record stored
    Stored { record: MemoryRecord },
    /// Multiple records returned
    Records { records: Vec<MemoryRecord>, count: usize },
    /// Count of affected records
    Affected { count: usize },
    /// Message result
    Message { message: String },
}

/// Query execution metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryMetadata {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Number of records scanned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_scanned: Option<usize>,
    /// Number of records returned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub records_returned: Option<usize>,
}
