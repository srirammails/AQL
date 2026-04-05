//! Configuration for ClawDB

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// ClawDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to LanceDB database directory
    pub db_path: PathBuf,

    /// Default namespace for records
    #[serde(default = "default_namespace")]
    pub default_namespace: String,

    /// Default scope for records
    #[serde(default = "default_scope")]
    pub default_scope: String,

    /// Embedding dimension for semantic memory (0 = disabled)
    #[serde(default)]
    pub embedding_dim: usize,
}

fn default_namespace() -> String {
    "default".to_string()
}

fn default_scope() -> String {
    "private".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("./clawdb_data"),
            default_namespace: default_namespace(),
            default_scope: default_scope(),
            embedding_dim: 0,
        }
    }
}

impl Config {
    /// Create a new config with the given database path
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
            ..Default::default()
        }
    }

    /// Enable semantic embeddings with the given dimension
    pub fn with_embeddings(mut self, dim: usize) -> Self {
        self.embedding_dim = dim;
        self
    }

    /// Set default namespace
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.default_namespace = namespace.into();
        self
    }

    /// Set default scope
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.default_scope = scope.into();
        self
    }
}
