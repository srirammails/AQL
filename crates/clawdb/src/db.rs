//! ClawDB main database struct

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use lancedb::connect;

use crate::backends::{LanceBackend, MemoryBackend};
use crate::config::Config;
use crate::error::ClawResult;
use crate::executor::Executor;
use crate::types::{MemoryType, QueryResult};

/// A link between two records
#[derive(Debug, Clone)]
pub struct Link {
    pub source_id: String,
    pub source_type: MemoryType,
    pub target_id: String,
    pub target_type: MemoryType,
    pub link_type: String,
    pub weight: Option<f32>,
}

/// Links storage
pub type LinksStorage = Vec<Link>;

/// ClawDB - Single-node Agent Database
pub struct ClawDB {
    config: Config,
    backends: HashMap<MemoryType, Arc<LanceBackend>>,
    executor: Executor,
    links: Arc<RwLock<LinksStorage>>,
}

impl ClawDB {
    /// Open or create a ClawDB instance
    pub async fn open(config: Config) -> ClawResult<Self> {
        // Ensure database directory exists
        if !config.db_path.exists() {
            std::fs::create_dir_all(&config.db_path)?;
        }

        // Connect to LanceDB
        let conn = connect(config.db_path.to_string_lossy().as_ref())
            .execute()
            .await?;

        // Initialize backends for each memory type
        let mut backends = HashMap::new();

        for memory_type in [
            MemoryType::Working,
            MemoryType::Episodic,
            MemoryType::Procedural,
            MemoryType::Tools,
            MemoryType::Semantic,
        ] {
            let backend = LanceBackend::open(&conn, memory_type).await?;
            backends.insert(memory_type, Arc::new(backend));
        }

        let executor = Executor::new(config.clone());
        let links = Arc::new(RwLock::new(Vec::new()));

        Ok(Self {
            config,
            backends,
            executor,
            links,
        })
    }

    /// Open with default configuration at the given path
    pub async fn open_path(path: impl AsRef<Path>) -> ClawResult<Self> {
        let config = Config::new(path.as_ref());
        Self::open(config).await
    }

    /// Execute an AQL query string
    pub async fn query(&self, aql: &str) -> ClawResult<QueryResult> {
        self.executor.execute(aql, &self.backends, &self.links).await
    }

    /// Execute an AQL query with variables
    pub async fn query_with_vars(
        &self,
        aql: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> ClawResult<QueryResult> {
        self.executor
            .execute_with_vars(aql, &self.backends, &self.links, variables)
            .await
    }

    /// Get the links storage
    pub fn links(&self) -> Arc<RwLock<LinksStorage>> {
        self.links.clone()
    }

    /// Get a backend for a specific memory type
    pub fn backend(&self, memory_type: MemoryType) -> Option<Arc<LanceBackend>> {
        self.backends.get(&memory_type).cloned()
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get statistics about stored data
    pub async fn stats(&self) -> ClawResult<DbStats> {
        let mut stats = DbStats::default();

        for (memory_type, backend) in &self.backends {
            let records = backend.scan(&Default::default()).await?;
            match memory_type {
                MemoryType::Working => stats.working = records.len(),
                MemoryType::Episodic => stats.episodic = records.len(),
                MemoryType::Procedural => stats.procedural = records.len(),
                MemoryType::Tools => stats.tools = records.len(),
                MemoryType::Semantic => stats.semantic = records.len(),
            }
        }

        Ok(stats)
    }
}

/// Database statistics
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct DbStats {
    pub working: usize,
    pub episodic: usize,
    pub procedural: usize,
    pub tools: usize,
    pub semantic: usize,
}
