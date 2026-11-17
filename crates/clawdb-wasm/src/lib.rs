//! ClawDB WASM - In-memory AQL playground for browsers
//!
//! AQL v0.13 on all 5 memory types, in memory. SEMANTIC records are stored like the
//! others; `RECALL FROM SEMANTIC LIKE …` searches a pre-seeded set of concepts.
//! A query is a script: statements separated by `;`, `USE "ns"` naming the namespace
//! of the statements after it; the answer is the last statement's.

mod memory;
mod executor;
mod script;
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

/// Parse an AQL query (a statement or a script) and return the AST as JSON (for
/// debugging): one statement, or an array for a script of several. USE statements are
/// applied to the statements after them.
#[wasm_bindgen]
pub fn parse(query: &str) -> String {
    let result = match aql_parser::parse_script(query, None) {
        Ok(stmts) if stmts.len() == 1 => QueryResult::ok(serde_json::to_value(&stmts[0]).unwrap_or(serde_json::Value::Null)),
        Ok(stmts) => QueryResult::ok(serde_json::to_value(&stmts).unwrap_or(serde_json::Value::Null)),
        Err(e) => QueryResult::err(&script::parse_failure(query, &e)),
    };
    serde_json::to_string(&result).unwrap()
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

/// Run a script: its statements in order; the answer is the last one's. A failing
/// statement stops the script ("statement N: …", N counting USE statements); the
/// statements before it stay done.
fn execute_query_internal(query: &str) -> QueryResult {
    let stmts = match aql_parser::parse_script(query, None) {
        Ok(s) => s,
        Err(e) => return QueryResult::err(&script::parse_failure(query, &e)),
    };
    let (positions, written) = script::positions(query, stmts.len());

    let mut store = STORE.lock().unwrap();
    let mut answer = QueryResult::ok_msg("USE names the namespace of the statements after it in the same script");
    for (stmt, n) in stmts.iter().zip(positions) {
        answer = execute_statement(stmt, &mut store);
        if !answer.success {
            if written > 1 {
                answer.error = answer.error.map(|e| format!("statement {n}: {e}"));
            }
            return answer;
        }
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;

    // The store is global: one test at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn test_store_and_recall() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        clear();

        let result = execute(r#"STORE INTO WORKING (name = "test", value = 42)"#);
        assert!(result.contains("success\":true"));

        let result = execute(r#"RECALL FROM WORKING WHERE name = "test""#);
        assert!(result.contains("test"));
        assert!(result.contains("42"));
    }

    fn run(q: &str) -> serde_json::Value {
        serde_json::from_str(&execute(q)).unwrap()
    }

    #[test]
    fn test_v013_core() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        clear();
        // A script: USE gives the namespace; the answer is the last statement's
        let r = run(r#"USE "t-ns"; STORE INTO SEMANTIC (key = "a", n = 1); STORE INTO SEMANTIC (key = "b", n = 5); RECALL FROM SEMANTIC WHERE n BETWEEN 0 AND 3"#);
        assert_eq!(r["data"]["count"], 1, "{r}");
        assert_eq!(r["data"]["records"][0]["namespace"], "t-ns");
        // An exact read by key, in the namespace
        assert_eq!(run(r#"RECALL FROM SEMANTIC WHERE key = "b" NAMESPACE "t-ns""#)["data"]["count"], 1);
        assert_eq!(run(r#"RECALL FROM SEMANTIC WHERE key = "b" NAMESPACE "other""#)["data"]["count"], 0);
        // LINK creates, then sets the weight; read from either end
        let link = r#"USE "t-ns"; LINK FROM SEMANTIC WHERE key = "a" INTO SEMANTIC WHERE key = "b" TYPE "rel" WEIGHT 0.5"#;
        assert_eq!(run(link)["data"]["count"], 1);
        run(&link.replace("0.5", "0.7"));
        let r = run(r#"RECALL FROM SEMANTIC WHERE key = "b" WITH LINKS"#);
        assert_eq!(r["data"]["records"][0]["links"].as_array().unwrap().len(), 1);
        assert_eq!(r["data"]["records"][0]["links"][0]["weight"], 0.7);
        let r = run(r#"RECALL FROM SEMANTIC WHERE key = "b" FOLLOW LINKS TYPE "rel""#);
        assert_eq!(r["data"]["records"][0]["data"]["key"], "a");
        // UNLINK
        assert_eq!(run(r#"UNLINK FROM SEMANTIC WHERE key = "a" INTO SEMANTIC WHERE key = "b""#)["data"]["count"], 1);
        // IN with a list
        assert_eq!(run(r#"RECALL FROM SEMANTIC WHERE key IN ("a", "b", "z")"#)["data"]["count"], 2);
        // FORGET without WHERE: everything in the namespace
        assert_eq!(run(r#"FORGET FROM SEMANTIC NAMESPACE "t-ns""#)["data"]["count"], 2);
    }

    #[test]
    fn test_v013_paths_and_refusals() {
        let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        clear();
        run(r#"STORE INTO PROCEDURAL (name = "retry with exponential backoff", steps = "wait, retry")"#);
        run(r#"STORE INTO PROCEDURAL (pattern = "pod OOMKilled restart", steps = "raise limits")"#);
        let r = run(r#"RECALL FROM PROCEDURAL LIKE "retry with backoff" CONFIDENCE 0.5 LIMIT 3"#);
        assert_eq!(r["data"]["count"], 1, "{r}");
        let r = run(r#"RECALL FROM PROCEDURAL LIKE "pod OOMKilled""#);
        assert_eq!(r["data"]["records"][0]["data"]["pattern"], "pod OOMKilled restart");
        // WORKING: newest first, WINDOW LAST n
        run(r#"STORE INTO WORKING (n = 1)"#);
        run(r#"STORE INTO WORKING (n = 2)"#);
        let r = run(r#"RECALL FROM WORKING WINDOW LAST 1"#);
        assert_eq!(r["data"]["records"][0]["data"]["n"], 2);
        // TOOLS: ranked
        run(r#"STORE INTO TOOLS (tool_id = "lo", ranking = 0.1)"#);
        run(r#"STORE INTO TOOLS (tool_id = "hi", ranking = 0.9)"#);
        assert_eq!(run(r#"RECALL FROM TOOLS LIMIT 1"#)["data"]["records"][0]["data"]["tool_id"], "hi");
        // PIPELINE: {var} from an earlier stage, write last
        let r = run(r#"PIPELINE p RECALL FROM TOOLS WHERE tool_id = "hi" | STORE INTO EPISODIC (best = {tool_id})"#);
        assert_eq!(r["data"]["final_result"]["data"]["record"]["data"]["best"], "hi", "{r}");
        // Refusals name the feature and ADB
        for q in [
            r#"BRANCH FROM "a" INTO "b""#,
            r#"MERGE FROM "b" INTO "a""#,
            r#"PUBLISH TOPIC "t" (x = 1)"#,
            r#"RECALL FROM SEMANTIC WHERE key = "a" NAMESPACE ("a", "b")"#,
            r#"RECALL FROM SEMANTIC WHERE key = "a" AS OF "2026-01-01T00:00:00Z""#,
        ] {
            let r = run(q);
            assert_eq!(r["success"], false, "{q}");
            assert!(r["error"].as_str().unwrap().contains("ADB implements it"), "{q}: {r}");
        }
        // A failing statement names its position, USE counted
        let r = run(r#"USE "x"; RECALL FROM WORKING; BRANCH FROM "a" INTO "b""#);
        assert!(r["error"].as_str().unwrap().starts_with("statement 3: BRANCH"), "{r}");
    }
}
