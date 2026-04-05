//! In-memory storage for all 5 memory types + links

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
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub links: Vec<Link>,
}

/// A link between two records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub link_type: String,
    pub weight: f32,
    pub created_at: i64,
}

impl Link {
    pub fn new(source_id: &str, target_id: &str, link_type: &str, weight: Option<f32>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            link_type: link_type.to_string(),
            weight: weight.unwrap_or(1.0),
            created_at: Utc::now().timestamp_millis(),
        }
    }
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
            links: Vec::new(),
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

    /// Add a link to this record
    pub fn add_link(&mut self, link: Link) {
        self.links.push(link);
    }

    /// Get links of a specific type
    pub fn get_links_by_type(&self, link_type: &str) -> Vec<&Link> {
        self.links.iter().filter(|l| l.link_type == link_type).collect()
    }

    /// Get all links
    pub fn get_all_links(&self) -> &[Link] {
        &self.links
    }
}

/// Get nested field from JSON value using dot notation
pub fn get_nested_field<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
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
    pub links: usize,
}

/// In-memory store for all memory types
pub struct MemoryStore {
    pub working: HashMap<String, Record>,
    pub episodic: HashMap<String, Record>,
    pub procedural: HashMap<String, Record>,
    pub tools: HashMap<String, Record>,
    pub semantic: SemanticStore,
    /// Global link index: link_id -> Link
    pub links: HashMap<String, Link>,
    /// Index from source_id to link_ids
    pub links_by_source: HashMap<String, Vec<String>>,
    /// Index from target_id to link_ids
    pub links_by_target: HashMap<String, Vec<String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            working: HashMap::new(),
            episodic: HashMap::new(),
            procedural: HashMap::new(),
            tools: HashMap::new(),
            semantic: SemanticStore::new(),
            links: HashMap::new(),
            links_by_source: HashMap::new(),
            links_by_target: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.working.clear();
        self.episodic.clear();
        self.procedural.clear();
        self.tools.clear();
        self.links.clear();
        self.links_by_source.clear();
        self.links_by_target.clear();
        // Don't clear semantic - it's pre-seeded
    }

    pub fn stats(&self) -> StoreStats {
        StoreStats {
            working: self.working.len(),
            episodic: self.episodic.len(),
            procedural: self.procedural.len(),
            tools: self.tools.len(),
            semantic: self.semantic.len(),
            links: self.links.len(),
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

    /// Get record by ID from any memory type
    pub fn get_record(&self, id: &str) -> Option<&Record> {
        self.working.get(id)
            .or_else(|| self.episodic.get(id))
            .or_else(|| self.procedural.get(id))
            .or_else(|| self.tools.get(id))
    }

    /// Get mutable record by ID from any memory type
    pub fn get_record_mut(&mut self, id: &str) -> Option<&mut Record> {
        if self.working.contains_key(id) {
            return self.working.get_mut(id);
        }
        if self.episodic.contains_key(id) {
            return self.episodic.get_mut(id);
        }
        if self.procedural.contains_key(id) {
            return self.procedural.get_mut(id);
        }
        if self.tools.contains_key(id) {
            return self.tools.get_mut(id);
        }
        None
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
        // First collect IDs that were actually deleted from the store
        let deleted_ids: Vec<String> = if let Some(store) = self.get_store_mut(memory_type) {
            ids.into_iter()
                .filter(|id| store.remove(id).is_some())
                .collect()
        } else {
            return 0;
        };

        // Then remove links for deleted records (separate borrow)
        let count = deleted_ids.len();
        for id in deleted_ids {
            self.remove_links_for_record(&id);
        }
        count
    }

    /// Create a link between two records
    pub fn create_link(&mut self, source_id: &str, target_id: &str, link_type: &str, weight: Option<f32>) -> Result<Link, String> {
        // Verify source exists
        if self.get_record(source_id).is_none() {
            return Err(format!("Source record not found: {}", source_id));
        }
        // Verify target exists
        if self.get_record(target_id).is_none() {
            return Err(format!("Target record not found: {}", target_id));
        }

        let link = Link::new(source_id, target_id, link_type, weight);
        let link_id = link.id.clone();

        // Add to global link index
        self.links.insert(link_id.clone(), link.clone());

        // Add to source index
        self.links_by_source
            .entry(source_id.to_string())
            .or_insert_with(Vec::new)
            .push(link_id.clone());

        // Add to target index
        self.links_by_target
            .entry(target_id.to_string())
            .or_insert_with(Vec::new)
            .push(link_id.clone());

        // Add link to source record
        if let Some(record) = self.get_record_mut(source_id) {
            record.add_link(link.clone());
        }

        Ok(link)
    }

    /// Remove all links for a record (when it's deleted)
    fn remove_links_for_record(&mut self, record_id: &str) {
        // Get link IDs to remove
        let mut link_ids_to_remove = Vec::new();
        if let Some(ids) = self.links_by_source.remove(record_id) {
            link_ids_to_remove.extend(ids);
        }
        if let Some(ids) = self.links_by_target.remove(record_id) {
            link_ids_to_remove.extend(ids);
        }

        // Remove links
        for id in link_ids_to_remove {
            self.links.remove(&id);
        }
    }

    /// Get links from a source record
    pub fn get_links_from(&self, source_id: &str, link_type: Option<&str>) -> Vec<&Link> {
        if let Some(link_ids) = self.links_by_source.get(source_id) {
            link_ids.iter()
                .filter_map(|id| self.links.get(id))
                .filter(|link| link_type.map_or(true, |t| link.link_type == t))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get links to a target record
    pub fn get_links_to(&self, target_id: &str, link_type: Option<&str>) -> Vec<&Link> {
        if let Some(link_ids) = self.links_by_target.get(target_id) {
            link_ids.iter()
                .filter_map(|id| self.links.get(id))
                .filter(|link| link_type.map_or(true, |t| link.link_type == t))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Follow links from records to get target records
    pub fn follow_links(&self, source_ids: &[String], link_type: &str, depth: u32) -> Vec<&Record> {
        let mut visited = std::collections::HashSet::new();
        let mut current_ids: Vec<String> = source_ids.to_vec();
        let mut result = Vec::new();

        for _ in 0..depth {
            let mut next_ids = Vec::new();
            for source_id in &current_ids {
                if visited.contains(source_id) {
                    continue;
                }
                visited.insert(source_id.clone());

                for link in self.get_links_from(source_id, Some(link_type)) {
                    if !visited.contains(&link.target_id) {
                        if let Some(record) = self.get_record(&link.target_id) {
                            result.push(record);
                            next_ids.push(link.target_id.clone());
                        }
                    }
                }
            }
            if next_ids.is_empty() {
                break;
            }
            current_ids = next_ids;
        }

        result
    }

    /// Get all records from multiple memory types (for REFLECT FROM ALL)
    pub fn get_all_records(&self) -> Vec<&Record> {
        let mut records: Vec<&Record> = Vec::new();
        records.extend(self.working.values().filter(|r| !r.is_expired()));
        records.extend(self.episodic.values().filter(|r| !r.is_expired()));
        records.extend(self.procedural.values().filter(|r| !r.is_expired()));
        records.extend(self.tools.values().filter(|r| !r.is_expired()));
        records
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
