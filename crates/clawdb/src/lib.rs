//! ClawDB - Single-node Agent Database with direct LanceDB storage
//!
//! ClawDB provides persistent storage for all 5 memory types:
//! - Working: Short-term scratch space with TTL
//! - Episodic: Time-series events and experiences
//! - Procedural: Patterns, procedures, and skills
//! - Tools: Tool registry with rankings
//! - Semantic: Knowledge with vector embeddings
//!
//! All data is stored directly in LanceDB tables with no in-memory caching.

pub mod backends;
pub mod config;
pub mod db;
pub mod error;
pub mod executor;
pub mod planner;
pub mod types;

// Re-exports
pub use config::Config;
pub use db::ClawDB;
pub use error::{ClawError, ClawResult};
pub use types::{MemoryRecord, Metadata};

/// Prelude for common imports
pub mod prelude {
    pub use crate::db::ClawDB;
    pub use crate::error::{ClawError, ClawResult};
    pub use crate::types::{MemoryRecord, Metadata};
    pub use crate::Config;
}
