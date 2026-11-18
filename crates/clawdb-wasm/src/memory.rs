//! In-memory storage for all 5 memory types + links

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Insertion order (created_at can tie within a millisecond)
    #[serde(skip)]
    pub seq: u64,
}

/// A link between two records. Read from either end (v0.13 §3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub from_type: String,
    pub to_type: String,
    pub link_type: String,
    pub weight: f64,
    pub created_at: i64,
}

/// A weight as written (0.7, not 0.699999988 from the f32 the AST carries).
pub fn weight_of(w: f32) -> f64 {
    format!("{w}").parse().unwrap_or(w as f64)
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
            version: 1,
            ttl_ms: None,
            namespace: None,
            scope: None,
            seq: 0,
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

    /// Check if record is expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl_ms {
            let now = Utc::now().timestamp_millis();
            now > self.created_at + ttl
        } else {
            false
        }
    }

    /// In `namespace` (no namespace named: every record)
    pub fn in_namespace(&self, namespace: Option<&str>) -> bool {
        namespace.is_none_or(|ns| self.namespace.as_deref() == Some(ns))
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
    /// The pre-seeded concepts `RECALL FROM SEMANTIC LIKE …` searches
    pub semantic_seeded: usize,
    pub links: usize,
}

/// In-memory store for all memory types
pub struct MemoryStore {
    pub working: HashMap<String, Record>,
    pub episodic: HashMap<String, Record>,
    pub procedural: HashMap<String, Record>,
    pub tools: HashMap<String, Record>,
    /// Stored SEMANTIC records (STORE / UPDATE / WHERE reads)
    pub semantic_records: HashMap<String, Record>,
    /// Pre-seeded concepts for `RECALL FROM SEMANTIC LIKE …`
    pub semantic: SemanticStore,
    /// Links, in creation order
    pub links: Vec<Link>,
    next_seq: u64,
}

pub const RECORD_TYPES: [aql_parser::MemoryType; 5] = [
    aql_parser::MemoryType::Working,
    aql_parser::MemoryType::Episodic,
    aql_parser::MemoryType::Procedural,
    aql_parser::MemoryType::Tools,
    aql_parser::MemoryType::Semantic,
];

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            working: HashMap::new(),
            episodic: HashMap::new(),
            procedural: HashMap::new(),
            tools: HashMap::new(),
            semantic_records: HashMap::new(),
            semantic: SemanticStore::new(),
            links: Vec::new(),
            next_seq: 0,
        }
    }

    pub fn clear(&mut self) {
        self.working.clear();
        self.episodic.clear();
        self.procedural.clear();
        self.tools.clear();
        self.semantic_records.clear();
        self.links.clear();
        // The seeded concepts stay
    }

    pub fn stats(&self) -> StoreStats {
        StoreStats {
            working: self.working.len(),
            episodic: self.episodic.len(),
            procedural: self.procedural.len(),
            tools: self.tools.len(),
            semantic: self.semantic_records.len(),
            semantic_seeded: self.semantic.len(),
            links: self.links.len(),
        }
    }

    pub fn dump(&self, memory_type: &str) -> Result<Vec<&Record>, String> {
        let mt = aql_parser::MemoryType::from_str(memory_type).ok_or_else(|| format!("Unknown memory type: {}", memory_type))?;
        let mut records: Vec<&Record> = match mt {
            aql_parser::MemoryType::All => RECORD_TYPES.iter().flat_map(|t| self.get_store(t).unwrap().values()).collect(),
            t => self.get_store(&t).unwrap().values().collect(),
        };
        records.sort_by_key(|r| r.seq);
        Ok(records)
    }

    pub fn get_store(&self, memory_type: &aql_parser::MemoryType) -> Option<&HashMap<String, Record>> {
        match memory_type {
            aql_parser::MemoryType::Working => Some(&self.working),
            aql_parser::MemoryType::Episodic => Some(&self.episodic),
            aql_parser::MemoryType::Procedural => Some(&self.procedural),
            aql_parser::MemoryType::Tools => Some(&self.tools),
            aql_parser::MemoryType::Semantic => Some(&self.semantic_records),
            aql_parser::MemoryType::All => None,
        }
    }

    pub fn get_store_mut(&mut self, memory_type: &aql_parser::MemoryType) -> Option<&mut HashMap<String, Record>> {
        match memory_type {
            aql_parser::MemoryType::Working => Some(&mut self.working),
            aql_parser::MemoryType::Episodic => Some(&mut self.episodic),
            aql_parser::MemoryType::Procedural => Some(&mut self.procedural),
            aql_parser::MemoryType::Tools => Some(&mut self.tools),
            aql_parser::MemoryType::Semantic => Some(&mut self.semantic_records),
            aql_parser::MemoryType::All => None,
        }
    }

    /// The stores a statement on `memory_type` reads (ALL: every one)
    pub fn stores_for(&self, memory_type: &aql_parser::MemoryType) -> Vec<&HashMap<String, Record>> {
        match self.get_store(memory_type) {
            Some(s) => vec![s],
            None => RECORD_TYPES.iter().filter_map(|t| self.get_store(t)).collect(),
        }
    }

    /// Get record by ID from any memory type
    pub fn get_record(&self, id: &str) -> Option<&Record> {
        RECORD_TYPES.iter().find_map(|t| self.get_store(t).and_then(|s| s.get(id)))
    }

    /// Store a record
    pub fn store(&mut self, memory_type: &aql_parser::MemoryType, mut record: Record) -> Result<Record, String> {
        self.next_seq += 1;
        record.seq = self.next_seq;
        let store = self.get_store_mut(memory_type).ok_or_else(|| format!("Cannot store to {:?}", memory_type))?;
        store.insert(record.id.clone(), record.clone());
        Ok(record)
    }

    /// Delete records by id, and their links
    pub fn forget(&mut self, memory_type: &aql_parser::MemoryType, ids: Vec<String>) -> usize {
        let types: Vec<aql_parser::MemoryType> = match memory_type {
            aql_parser::MemoryType::All => RECORD_TYPES.to_vec(),
            t => vec![*t],
        };
        let mut deleted = HashSet::new();
        for t in types {
            if let Some(store) = self.get_store_mut(&t) {
                for id in &ids {
                    if store.remove(id).is_some() {
                        deleted.insert(id.clone());
                    }
                }
            }
        }
        self.links.retain(|l| !deleted.contains(&l.source_id) && !deleted.contains(&l.target_id));
        deleted.len()
    }

    /// LINK: create the link, or set the weight of the one of this type between the
    /// two records (no WEIGHT keeps the weight).
    pub fn upsert_link(&mut self, source: &Record, target: &Record, link_type: &str, weight: Option<f32>) -> Link {
        if let Some(l) = self
            .links
            .iter_mut()
            .find(|l| l.source_id == source.id && l.target_id == target.id && l.link_type == link_type)
        {
            if let Some(w) = weight {
                l.weight = weight_of(w);
            }
            return l.clone();
        }
        let link = Link {
            id: Uuid::new_v4().to_string(),
            source_id: source.id.clone(),
            target_id: target.id.clone(),
            from_type: source.memory_type.clone(),
            to_type: target.memory_type.clone(),
            link_type: link_type.to_string(),
            weight: weight.map(weight_of).unwrap_or(1.0),
            created_at: Utc::now().timestamp_millis(),
        };
        self.links.push(link.clone());
        link
    }

    /// UNLINK: remove the links from `source_id` to `target_id` (of `link_type`, if given).
    pub fn unlink(&mut self, source_id: &str, target_id: &str, link_type: Option<&str>) -> usize {
        let before = self.links.len();
        self.links
            .retain(|l| !(l.source_id == source_id && l.target_id == target_id && link_type.is_none_or(|t| l.link_type == t)));
        before - self.links.len()
    }

    /// The links at either end of a record, in creation order
    pub fn links_of<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Link> + 'a {
        self.links.iter().filter(move |l| l.source_id == id || l.target_id == id)
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
