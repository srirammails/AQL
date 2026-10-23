//! HTTP REST API routes

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::AppState;

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// Query request
#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
    #[serde(default)]
    pub variables: Option<std::collections::HashMap<String, Value>>,
}

/// Query response
#[derive(Serialize)]
pub struct QueryResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_ms: Option<u64>,
}

/// Health check endpoint
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Execute AQL query
pub async fn query(
    State(state): State<AppState>,
    Json(payload): Json<QueryRequest>,
) -> (StatusCode, Json<QueryResponse>) {
    let start = std::time::Instant::now();

    let result = if let Some(vars) = payload.variables {
        state.db.query_with_vars(&payload.query, vars).await
    } else {
        state.db.query(&payload.query).await
    };

    let execution_time_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(query_result) => {
            let data = serde_json::to_value(&query_result).ok();
            (
                StatusCode::OK,
                Json(QueryResponse {
                    success: query_result.success,
                    data,
                    error: query_result.error,
                    execution_time_ms: Some(execution_time_ms),
                }),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(QueryResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
                execution_time_ms: Some(execution_time_ms),
            }),
        ),
    }
}

/// Get database statistics
pub async fn stats(State(state): State<AppState>) -> (StatusCode, Json<QueryResponse>) {
    match state.db.stats().await {
        Ok(stats) => {
            let data = serde_json::to_value(&stats).ok();
            (
                StatusCode::OK,
                Json(QueryResponse {
                    success: true,
                    data,
                    error: None,
                    execution_time_ms: None,
                }),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(QueryResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
                execution_time_ms: None,
            }),
        ),
    }
}
