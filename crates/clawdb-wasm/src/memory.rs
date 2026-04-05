//! In-memory storage for all 5 memory types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

use crate::semantic::SemanticStore;

/// A memory record stored in any memory type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub memory_type: String,
    pub data: serde_json::Value,
    pub created_at: i64,
    pub accessed_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

impl Record {
    pub fn new(memory_type: &str, data: serde_json::Value) -> Self {
        let now = Utc::now().timestamp_millis();
        Self {
            id: Uuid::new_v4().to_string(),
            memory_type: memory_type.to_string(),
            data,
            created_at: now,
            accessed_at: now,
            ttl_ms: None,
            namespace: None,
            scope: None,
        }
    }

    pub fn with_ttl(mut self, ttl_ms: i64) -> Self {
        self.ttl_ms = Some(ttl_ms);
        self
    }

    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub fn with_scope(mut self, scope: String) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Check if record is expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_ms {
            let now = Utc::now().timestamp_millis();
            now > self.created_at + ttl
        } else {
            false
        }
    }

    /// Get a field value from the record (supports dotted paths)
    pub fn get_field(&self, path: &str) -> Option<&serde_json::Value> {
        // Handle metadata fields
        if path.starts_with("metadata.") {
            let field = &path[9..];
            return match field {
                "namespace" => self.namespace.as_ref().map(|s| {
                    // Return as borrowed reference - need static storage
                    // For simplicity, check in data
                    None
                }).flatten(),
                "scope" => None, // Similar issue
                "created_at" => None,
                _ => None,
            };
        }

        // Handle data fields
        let field_path = if path.starts_with("data.") {
            &path[5..]
        } else {
            path
        };

        get_nested_field(&self.data, field_path)
    }
}

/// Get nested field from JSON value using dot notation
fn get_nested_field<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

/// Memory store statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct StoreStats {
    pub working: usize,
    pub episodic: usize,
    pub procedural: usize,
    pub tools: usize,
    pub semantic: usize,
}

/// In-memory store for all memory types
pub struct MemoryStore {
    pub working: HashMap<String, Record>,
    pub episodic: HashMap<String, Record>,
    pub procedural: HashMap<String, Record>,
    pub tools: HashMap<String, Record>,
    pub semantic: SemanticStore,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            working: HashMap::new(),
            episodic: HashMap::new(),
            procedural: HashMap::new(),
            tools: HashMap::new(),
            semantic: SemanticStore::new(),
        }
    }

    pub fn clear(&mut self) {
        self.working.clear();
        self.episodic.clear();
        self.procedural.clear();
        self.tools.clear();
        // Don't clear semantic - it's pre-seeded
    }

    pub fn stats(&self) -> StoreStats {
        StoreStats {
            working: self.working.len(),
            episodic: self.episodic.len(),
            procedural: self.procedural.len(),
            tools: self.tools.len(),
            semantic: self.semantic.len(),
        }
    }

    pub fn dump(&self, memory_type: &str) -> Result<Vec<&Record>, String> {
        match memory_type.to_uppercase().as_str() {
            "WORKING" => Ok(self.working.values().collect()),
            "EPISODIC" => Ok(self.episodic.values().collect()),
            "PROCEDURAL" => Ok(self.procedural.values().collect()),
            "TOOLS" => Ok(self.tools.values().collect()),
            "SEMANTIC" => Err("Use RECALL FROM SEMANTIC LIKE to query semantic memory".to_string()),
            _ => Err(format!("Unknown memory type: {}", memory_type)),
        }
    }

    pub fn get_store(&self, memory_type: &aql_parser::MemoryType) -> Option<&HashMap<String, Record>> {
        match memory_type {
            aql_parser::MemoryType::Working => Some(&self.working),
            aql_parser::MemoryType::Episodic => Some(&self.episodic),
            aql_parser::MemoryType::Procedural => Some(&self.procedural),
            aql_parser::MemoryType::Tools => Some(&self.tools),
            _ => None,
        }
    }

    pub fn get_store_mut(&mut self, memory_type: &aql_parser::MemoryType) -> Option<&mut HashMap<String, Record>> {
        match memory_type {
            aql_parser::MemoryType::Working => Some(&mut self.working),
            aql_parser::MemoryType::Episodic => Some(&mut self.episodic),
            aql_parser::MemoryType::Procedural => Some(&mut self.procedural),
            aql_parser::MemoryType::Tools => Some(&mut self.tools),
            _ => None,
        }
    }

    /// Store a record
    pub fn store(&mut self, memory_type: &aql_parser::MemoryType, record: Record) -> Result<Record, String> {
        if matches!(memory_type, aql_parser::MemoryType::Semantic) {
            return Err("Semantic memory is read-only in playground. Pre-seeded data available for queries.".to_string());
        }

        let store = self.get_store_mut(memory_type)
            .ok_or_else(|| format!("Cannot store to {:?}", memory_type))?;

        let id = record.id.clone();
        store.insert(id, record.clone());
        Ok(record)
    }

    /// Delete records matching conditions
    pub fn forget(&mut self, memory_type: &aql_parser::MemoryType, ids: Vec<String>) -> usize {
        if let Some(store) = self.get_store_mut(memory_type) {
            let mut deleted = 0;
            for id in ids {
                if store.remove(&id).is_some() {
                    deleted += 1;
                }
            }
            deleted
        } else {
            0
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
