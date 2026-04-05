//! Error types for ClawDB

use thiserror::Error;

/// ClawDB error type
#[derive(Error, Debug)]
pub enum ClawError {
    #[error("LanceDB error: {0}")]
    Lance(#[from] lancedb::Error),

    #[error("LanceDB error: {0}")]
    LanceDb(String),

    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    #[error("Parse error: {0}")]
    Parse(#[from] aql_parser::ParseError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid memory type: {0}")]
    InvalidMemoryType(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Semantic memory requires embeddings")]
    EmbeddingRequired,

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Result type for ClawDB operations
pub type ClawResult<T> = Result<T, ClawError>;
