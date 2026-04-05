//! ClawDB WASM - In-memory AQL playground for browsers
//!
//! Supports all 5 memory types:
//! - Working, Episodic, Procedural, Tools: Full STORE/QUERY support
//! - Semantic: Pre-seeded read-only with vector similarity

mod memory;
mod executor;
mod semantic;

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use once_cell::sync::Lazy;

use memory::MemoryStore;
use executor::execute_statement;

// Global memory store (WASM is single-threaded)
static STORE: Lazy<Mutex<MemoryStore>> = Lazy::new(|| {
    Mutex::new(MemoryStore::new())
});

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Result type for WASM operations
#[derive(Serialize, Deserialize)]
pub struct QueryResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl QueryResult {
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn ok_msg(message: &str) -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            message: Some(message.to_string()),
        }
    }

    pub fn err(error: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.to_string()),
            message: None,
        }
    }
}

/// Execute an AQL query and return JSON result
#[wasm_bindgen]
pub fn execute(query: &str) -> String {
    let result = execute_query_internal(query);
    serde_json::to_string(&result).unwrap_or_else(|e| {
        format!(r#"{{"success":false,"error":"Serialization error: {}"}}"#, e)
    })
}

/// Parse an AQL query and return AST as JSON (for debugging)
#[wasm_bindgen]
pub fn parse(query: &str) -> String {
    match aql_parser::parse(query) {
        Ok(stmt) => {
            let result = QueryResult::ok(serde_json::to_value(&stmt).unwrap_or(serde_json::Value::Null));
            serde_json::to_string(&result).unwrap()
        }
        Err(e) => {
            let result = QueryResult::err(&format!("Parse error: {}", e));
            serde_json::to_string(&result).unwrap()
        }
    }
}

/// Clear all data from memory
#[wasm_bindgen]
pub fn clear() -> String {
    let mut store = STORE.lock().unwrap();
    store.clear();
    let result = QueryResult::ok_msg("All memory cleared");
    serde_json::to_string(&result).unwrap()
}

/// Get statistics about stored data
#[wasm_bindgen]
pub fn stats() -> String {
    let store = STORE.lock().unwrap();
    let stats = store.stats();
    let result = QueryResult::ok(serde_json::to_value(&stats).unwrap());
    serde_json::to_string(&result).unwrap()
}

/// Dump all records from a memory type
#[wasm_bindgen]
pub fn dump(memory_type: &str) -> String {
    let store = STORE.lock().unwrap();
    match store.dump(memory_type) {
        Ok(records) => {
            let result = QueryResult::ok(serde_json::to_value(&records).unwrap());
            serde_json::to_string(&result).unwrap()
        }
        Err(e) => {
            let result = QueryResult::err(&e);
            serde_json::to_string(&result).unwrap()
        }
    }
}

fn execute_query_internal(query: &str) -> QueryResult {
    // Parse the query
    let stmt = match aql_parser::parse(query) {
        Ok(s) => s,
        Err(e) => return QueryResult::err(&format!("Parse error: {}", e)),
    };

    // Execute against the store
    let mut store = STORE.lock().unwrap();
    execute_statement(&stmt, &mut store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_recall() {
        clear();

        let result = execute(r#"STORE INTO WORKING (name = "test", value = 42)"#);
        assert!(result.contains("success\":true"));

        let result = execute(r#"RECALL FROM WORKING WHERE name = "test""#);
        assert!(result.contains("test"));
        assert!(result.contains("42"));
    }
}
