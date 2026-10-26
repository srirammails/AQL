//! MCP (Model Context Protocol) server implementation with HTTP+SSE transport
//!
//! Implements the MCP HTTP+SSE transport specification for Claude Desktop integration.
//! - GET /mcp/sse - SSE connection that sends an `endpoint` event with POST URL
//! - POST /mcp/message - Receives JSON-RPC requests, responses sent via SSE

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::AppState;

// ============================================================================
// Session Management
// ============================================================================

/// Manages MCP sessions for HTTP+SSE transport
#[derive(Clone, Default)]
pub struct McpSessions {
    sessions: Arc<RwLock<HashMap<String, broadcast::Sender<String>>>>,
}

impl McpSessions {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new session and return (session_id, receiver)
    pub async fn create(&self) -> (String, broadcast::Receiver<String>) {
        let session_id = Uuid::new_v4().to_string();
        let (tx, rx) = broadcast::channel(100);
        self.sessions.write().await.insert(session_id.clone(), tx);
        (session_id, rx)
    }

    /// Send a message to a session
    pub async fn send(&self, session_id: &str, message: String) -> bool {
        if let Some(tx) = self.sessions.read().await.get(session_id) {
            tx.send(message).is_ok()
        } else {
            false
        }
    }

    /// Remove a session
    pub async fn remove(&self, session_id: &str) {
        self.sessions.write().await.remove(session_id);
    }
}

// ============================================================================
// Extended App State with Sessions
// ============================================================================

#[derive(Clone)]
pub struct McpState {
    pub app: AppState,
    pub sessions: McpSessions,
}

// ============================================================================
// MCP Protocol Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: &'static str,
    pub text: String,
}

impl ToolCallResult {
    fn text(text: &str) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text",
                text: text.to_string(),
            }],
            is_error: None,
        }
    }

    fn json(value: &Value) -> Self {
        Self::text(&serde_json::to_string_pretty(value).unwrap_or_default())
    }

    fn error(message: &str) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text",
                text: message.to_string(),
            }],
            is_error: Some(true),
        }
    }
}

// ============================================================================
// Tool Definitions
// ============================================================================

fn get_tools() -> Value {
    json!({
        "tools": [
            {
                "name": "aql_query",
                "description": "Execute an AQL (Agent Query Language) query against ClawDB. Use this for all memory operations including STORE, RECALL, SCAN, LOOKUP, LOAD, FORGET, UPDATE, LINK, REFLECT, and PIPELINE.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "The AQL query to execute"
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "store_memory",
                "description": "Store data in agent memory. Shortcut for STORE INTO queries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "memory_type": {
                            "type": "string",
                            "enum": ["WORKING", "EPISODIC", "PROCEDURAL", "SEMANTIC", "TOOLS"],
                            "description": "Type of memory to store in"
                        },
                        "data": {
                            "type": "object",
                            "description": "Key-value pairs to store"
                        },
                        "scope": {
                            "type": "string",
                            "enum": ["private", "shared"],
                            "description": "Visibility scope (optional)"
                        },
                        "namespace": {
                            "type": "string",
                            "description": "Namespace for isolation (optional)"
                        }
                    },
                    "required": ["memory_type", "data"]
                }
            },
            {
                "name": "recall_memory",
                "description": "Recall data from agent memory. Shortcut for RECALL FROM queries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "memory_type": {
                            "type": "string",
                            "enum": ["WORKING", "EPISODIC", "PROCEDURAL", "SEMANTIC", "TOOLS"],
                            "description": "Type of memory to recall from"
                        },
                        "filter": {
                            "type": "object",
                            "description": "Field conditions to filter by (optional)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of records to return (optional)"
                        },
                        "order_by": {
                            "type": "string",
                            "description": "Field to order by (optional)"
                        },
                        "order_desc": {
                            "type": "boolean",
                            "description": "Order descending (optional, default false)"
                        }
                    },
                    "required": ["memory_type"]
                }
            },
            {
                "name": "get_stats",
                "description": "Get ClawDB statistics including record counts per memory type.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

// ============================================================================
// MCP Request Handler (Direct POST - for non-SSE clients)
// ============================================================================

pub async fn handle_mcp_request(
    State(state): State<McpState>,
    Json(request): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    let response = process_request(&state.app, request).await;
    (StatusCode::OK, Json(response))
}

async fn process_request(state: &AppState, request: JsonRpcRequest) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => {
            JsonRpcResponse::success(request.id, json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "clawdb-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }))
        }
        "initialized" => {
            JsonRpcResponse::success(request.id, json!({}))
        }
        "notifications/initialized" => {
            // Client notification that it has initialized - no response needed for notifications
            JsonRpcResponse::success(request.id, json!({}))
        }
        "tools/list" => {
            JsonRpcResponse::success(request.id, get_tools())
        }
        "tools/call" => {
            handle_tool_call(state, request.id, request.params).await
        }
        "ping" => {
            JsonRpcResponse::success(request.id, json!({}))
        }
        "shutdown" => {
            JsonRpcResponse::success(request.id, json!(null))
        }
        method if method.starts_with("notifications/") => {
            // Notifications don't require responses, but we'll acknowledge them
            JsonRpcResponse::success(request.id, json!({}))
        }
        _ => {
            JsonRpcResponse::error(
                request.id,
                -32601,
                format!("Method not found: {}", request.method),
            )
        }
    }
}

async fn handle_tool_call(
    state: &AppState,
    id: Option<Value>,
    params: Option<Value>,
) -> JsonRpcResponse {
    let params: ToolCallParams = match params {
        Some(p) => match serde_json::from_value(p) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(id, -32602, format!("Invalid params: {}", e));
            }
        },
        None => {
            return JsonRpcResponse::error(id, -32602, "Missing params".to_string());
        }
    };

    let result = match params.name.as_str() {
        "aql_query" => handle_aql_query(state, params.arguments).await,
        "store_memory" => handle_store_memory(state, params.arguments).await,
        "recall_memory" => handle_recall_memory(state, params.arguments).await,
        "get_stats" => handle_get_stats(state).await,
        _ => ToolCallResult::error(&format!("Unknown tool: {}", params.name)),
    };

    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap())
}

// ============================================================================
// Tool Implementations
// ============================================================================

async fn handle_aql_query(state: &AppState, args: Option<Value>) -> ToolCallResult {
    let query = match args.and_then(|a| a.get("query").and_then(|q| q.as_str()).map(String::from)) {
        Some(q) => q,
        None => return ToolCallResult::error("Missing required parameter: query"),
    };

    match state.db.query(&query).await {
        Ok(result) => ToolCallResult::json(&serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => ToolCallResult::error(&format!("Query failed: {}", e)),
    }
}

async fn handle_store_memory(state: &AppState, args: Option<Value>) -> ToolCallResult {
    let args = match args {
        Some(a) => a,
        None => return ToolCallResult::error("Missing arguments"),
    };

    let memory_type = match args.get("memory_type").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => return ToolCallResult::error("Missing required: memory_type"),
    };

    let data = match args.get("data") {
        Some(d) if d.is_object() => d.as_object().unwrap(),
        _ => return ToolCallResult::error("Missing or invalid: data (must be object)"),
    };

    // Build AQL query
    let fields: Vec<String> = data
        .iter()
        .map(|(k, v)| {
            let val = match v {
                Value::String(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => format!("\"{}\"", v.to_string().replace("\"", "\\\"")),
            };
            format!("{} = {}", k, val)
        })
        .collect();

    let mut query = format!("STORE INTO {} ({})", memory_type, fields.join(", "));

    if let Some(scope) = args.get("scope").and_then(|s| s.as_str()) {
        query.push_str(&format!(" SCOPE {}", scope));
    }

    if let Some(ns) = args.get("namespace").and_then(|n| n.as_str()) {
        query.push_str(&format!(" NAMESPACE \"{}\"", ns));
    }

    match state.db.query(&query).await {
        Ok(result) => ToolCallResult::json(&serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => ToolCallResult::error(&format!("Store failed: {}", e)),
    }
}

async fn handle_recall_memory(state: &AppState, args: Option<Value>) -> ToolCallResult {
    let args = match args {
        Some(a) => a,
        None => return ToolCallResult::error("Missing arguments"),
    };

    let memory_type = match args.get("memory_type").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => return ToolCallResult::error("Missing required: memory_type"),
    };

    let mut query = format!("RECALL FROM {}", memory_type);

    // Build WHERE clause
    if let Some(filter) = args.get("filter").and_then(|f| f.as_object()) {
        if !filter.is_empty() {
            let conditions: Vec<String> = filter
                .iter()
                .map(|(k, v)| {
                    let val = match v {
                        Value::String(s) => format!("\"{}\"", s),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => format!("\"{}\"", v.to_string()),
                    };
                    format!("{} = {}", k, val)
                })
                .collect();
            query.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }
    }

    // ORDER BY
    if let Some(order_by) = args.get("order_by").and_then(|o| o.as_str()) {
        let desc = args.get("order_desc").and_then(|d| d.as_bool()).unwrap_or(false);
        query.push_str(&format!(" ORDER BY {} {}", order_by, if desc { "DESC" } else { "ASC" }));
    }

    // LIMIT
    if let Some(limit) = args.get("limit").and_then(|l| l.as_u64()) {
        query.push_str(&format!(" LIMIT {}", limit));
    }

    match state.db.query(&query).await {
        Ok(result) => ToolCallResult::json(&serde_json::to_value(&result).unwrap_or_default()),
        Err(e) => ToolCallResult::error(&format!("Recall failed: {}", e)),
    }
}

async fn handle_get_stats(state: &AppState) -> ToolCallResult {
    match state.db.stats().await {
        Ok(stats) => ToolCallResult::json(&serde_json::to_value(&stats).unwrap_or_default()),
        Err(e) => ToolCallResult::error(&format!("Stats failed: {}", e)),
    }
}

// ============================================================================
// MCP over HTTP+SSE Transport
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// SSE endpoint - establishes connection and sends endpoint URL
pub async fn handle_mcp_sse(
    State(state): State<McpState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Create a new session
    let (session_id, mut rx) = state.sessions.create().await;
    let sessions = state.sessions.clone();
    let session_id_for_cleanup = session_id.clone();

    tracing::info!("MCP SSE session created: {}", session_id);

    // Build the endpoint URL for the client to POST messages to
    let endpoint_url = format!("/mcp/message?sessionId={}", session_id);

    // Create the stream
    let stream = stream::unfold(
        (Some(endpoint_url), rx, sessions, session_id_for_cleanup),
        |(endpoint_opt, mut rx, sessions, session_id)| async move {
            // First iteration: send endpoint event
            if let Some(endpoint) = endpoint_opt {
                let event = Event::default()
                    .event("endpoint")
                    .data(endpoint);
                return Some((Ok(event), (None, rx, sessions, session_id)));
            }

            // Subsequent iterations: wait for messages or send keepalive
            loop {
                tokio::select! {
                    result = rx.recv() => {
                        match result {
                            Ok(message) => {
                                let event = Event::default()
                                    .event("message")
                                    .data(message);
                                return Some((Ok(event), (None, rx, sessions, session_id)));
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                // Channel closed, end stream
                                sessions.remove(&session_id).await;
                                return None;
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                // Missed some messages, continue
                                continue;
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_secs(30)) => {
                        // Send keepalive ping
                        let event = Event::default().comment("ping");
                        return Some((Ok(event), (None, rx, sessions, session_id)));
                    }
                }
            }
        },
    );

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Message endpoint - receives JSON-RPC requests and sends responses via SSE
pub async fn handle_mcp_message(
    State(state): State<McpState>,
    Query(query): Query<MessageQuery>,
    Json(request): Json<JsonRpcRequest>,
) -> StatusCode {
    let session_id = &query.session_id;

    tracing::debug!("MCP message for session {}: method={}", session_id, request.method);

    // Process the request
    let response = process_request(&state.app, request).await;

    // Serialize and send via SSE
    let response_json = serde_json::to_string(&response).unwrap_or_default();

    if state.sessions.send(session_id, response_json).await {
        StatusCode::ACCEPTED
    } else {
        tracing::warn!("Session not found: {}", session_id);
        StatusCode::NOT_FOUND
    }
}
