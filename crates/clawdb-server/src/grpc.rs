//! gRPC service implementation for ClawDB
//!
//! Provides high-performance gRPC endpoint for container-to-container communication.
//! Latency: ~50-100μs (vs ~100-500μs for HTTP)

use std::collections::HashMap;

use tonic::{Request, Response, Status};
use tracing::{debug, info, instrument};

use crate::AppState;

// Include generated protobuf code
pub mod pb {
    tonic::include_proto!("clawdb");
}

use pb::claw_db_server::{ClawDb, ClawDbServer};
use pb::{
    HealthRequest, HealthResponse, QueryRequest, QueryResponse, StatsRequest, StatsResponse,
};

/// gRPC service implementation
pub struct ClawDbService {
    state: AppState,
}

impl ClawDbService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    /// Create a tonic Server for this service
    pub fn into_server(self) -> ClawDbServer<Self> {
        ClawDbServer::new(self)
    }
}

#[tonic::async_trait]
impl ClawDb for ClawDbService {
    #[instrument(skip(self, request), fields(query_len))]
    async fn query(&self, request: Request<QueryRequest>) -> Result<Response<QueryResponse>, Status> {
        let req = request.into_inner();
        let start = std::time::Instant::now();

        debug!(
            query_preview = %req.query.chars().take(80).collect::<String>(),
            "gRPC query request"
        );

        // Parse variables if provided
        let variables: Option<HashMap<String, serde_json::Value>> = if req.variables.is_empty() {
            None
        } else {
            match serde_json::from_str(&req.variables) {
                Ok(vars) => Some(vars),
                Err(e) => {
                    return Ok(Response::new(QueryResponse {
                        success: false,
                        data: String::new(),
                        error: format!("Invalid variables JSON: {}", e),
                        affected: 0,
                        execution_time_us: start.elapsed().as_micros() as i64,
                    }));
                }
            }
        };

        // Execute query
        let result = if let Some(vars) = variables {
            self.state.db.query_with_vars(&req.query, vars).await
        } else {
            self.state.db.query(&req.query).await
        };

        let execution_time_us = start.elapsed().as_micros() as i64;

        match result {
            Ok(query_result) => {
                let data = serde_json::to_string(&query_result).unwrap_or_default();
                let affected = query_result.metadata.records_returned.unwrap_or(0) as i64;

                debug!(
                    success = true,
                    execution_time_us = execution_time_us,
                    "gRPC query complete"
                );

                Ok(Response::new(QueryResponse {
                    success: query_result.success,
                    data,
                    error: query_result.error.unwrap_or_default(),
                    affected,
                    execution_time_us,
                }))
            }
            Err(e) => {
                debug!(
                    success = false,
                    error = %e,
                    "gRPC query failed"
                );

                Ok(Response::new(QueryResponse {
                    success: false,
                    data: String::new(),
                    error: e.to_string(),
                    affected: 0,
                    execution_time_us,
                }))
            }
        }
    }

    type QueryStreamStream = futures::stream::BoxStream<'static, Result<QueryResponse, Status>>;

    async fn query_stream(
        &self,
        request: Request<QueryRequest>,
    ) -> Result<Response<Self::QueryStreamStream>, Status> {
        // For now, just execute normally and return single result
        // TODO: Implement true streaming for large result sets
        let response = self.query(request).await?;
        let inner = response.into_inner();

        let stream = futures::stream::once(async move { Ok(inner) });

        Ok(Response::new(Box::pin(stream)))
    }

    async fn health(&self, _request: Request<HealthRequest>) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }

    async fn stats(&self, _request: Request<StatsRequest>) -> Result<Response<StatsResponse>, Status> {
        match self.state.db.stats().await {
            Ok(stats) => Ok(Response::new(StatsResponse {
                working_count: stats.working as i64,
                episodic_count: stats.episodic as i64,
                procedural_count: stats.procedural as i64,
                semantic_count: stats.semantic as i64,
                tools_count: stats.tools as i64,
            })),
            Err(e) => Err(Status::internal(format!("Failed to get stats: {}", e))),
        }
    }
}

/// Start the gRPC server
pub async fn run_grpc_server(state: AppState, port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port).parse()?;
    let service = ClawDbService::new(state);

    info!("ClawDB gRPC server listening on {}", addr);

    tonic::transport::Server::builder()
        .add_service(service.into_server())
        .serve(addr)
        .await?;

    Ok(())
}
