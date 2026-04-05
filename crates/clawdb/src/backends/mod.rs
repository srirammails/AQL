//! LanceDB backends for all memory types

mod lance;

pub use lance::LanceBackend;

use async_trait::async_trait;
use crate::error::ClawResult;
use crate::types::{MemoryRecord, MemoryType};
use aql_parser::{Condition, Modifiers};

/// Backend trait for memory storage
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Store a record
    async fn store(&self, record: MemoryRecord) -> ClawResult<MemoryRecord>;

    /// Query records with conditions and modifiers
    async fn query(
        &self,
        conditions: &[Condition],
        modifiers: &Modifiers,
    ) -> ClawResult<Vec<MemoryRecord>>;

    /// Delete records matching conditions
    async fn delete(&self, conditions: &[Condition]) -> ClawResult<usize>;

    /// Update records matching conditions
    async fn update(
        &self,
        conditions: &[Condition],
        updates: serde_json::Value,
    ) -> ClawResult<usize>;

    /// Scan all records with optional window
    async fn scan(&self, modifiers: &Modifiers) -> ClawResult<Vec<MemoryRecord>>;

    /// Get record by ID
    async fn get_by_id(&self, id: &str) -> ClawResult<Option<MemoryRecord>>;

    /// Get the memory type this backend handles
    fn memory_type(&self) -> MemoryType;
}
