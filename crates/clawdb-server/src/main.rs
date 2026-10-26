//! ClawDB Server - HTTP + gRPC + MCP/SSE server for Claude Desktop integration
//!
//! Provides:
//! - HTTP REST API for ClawFlow and other clients (port 3000)
//! - gRPC API for high-performance container-to-container calls (port 50051)
//! - MCP over SSE for Claude Desktop
//! - CLI for query execution

mod grpc;
mod mcp;
mod routes;

use std::path::PathBuf;
use std::sync::Arc;

use axum::{routing::{get, post}, Router};
use clap::{Parser, Subcommand};
use clawdb::{ClawDB, Config};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use mcp::{McpSessions, McpState};

#[derive(Parser)]
#[command(name = "clawdb")]
#[command(about = "ClawDB - Agent Memory Database Server", long_about = None)]
struct Cli {
    /// Database path
    #[arg(short, long, default_value = "./clawdb_data")]
    path: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the HTTP + gRPC servers with MCP/SSE support (default)
    Serve {
        /// HTTP port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// gRPC port to listen on
        #[arg(short, long, default_value = "50051")]
        grpc_port: u16,
    },
    /// Execute a single AQL query
    Query {
        /// The AQL query to execute
        query: String,
    },
    /// Interactive REPL mode
    Repl,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<ClawDB>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let is_serve = matches!(cli.command, None | Some(Commands::Serve { .. }));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| {
                    if is_serve { "info,tower_http=debug".into() } else { "warn".into() }
                }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_ansi(!is_serve)
        )
        .init();

    // Initialize ClawDB
    let config = Config::new(&cli.path);
    let db = ClawDB::open(config).await?;
    let db = Arc::new(db);

    match cli.command.unwrap_or(Commands::Serve { port: 3000, grpc_port: 50051 }) {
        Commands::Serve { port, grpc_port } => {
            run_server(db, port, grpc_port).await?;
        }
        Commands::Query { query } => {
            run_query(db, &query).await?;
        }
        Commands::Repl => {
            run_repl(db).await?;
        }
    }

    Ok(())
}

async fn run_server(db: Arc<ClawDB>, http_port: u16, grpc_port: u16) -> anyhow::Result<()> {
    let app_state = AppState { db };
    let mcp_state = McpState {
        app: app_state.clone(),
        sessions: McpSessions::new(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // REST API routes use AppState
    let rest_routes = Router::new()
        .route("/health", get(routes::health))
        .route("/query", post(routes::query))
        .route("/stats", get(routes::stats))
        .with_state(app_state.clone());

    // MCP routes use McpState (includes session management)
    let mcp_routes = Router::new()
        .route("/mcp", post(mcp::handle_mcp_request))
        .route("/mcp/sse", get(mcp::handle_mcp_sse))
        .route("/mcp/message", post(mcp::handle_mcp_message))
        .with_state(mcp_state);

    let app = Router::new()
        .merge(rest_routes)
        .merge(mcp_routes)
        .layer(cors);

    tracing::info!("ClawDB server starting...");
    tracing::info!("  HTTP:      http://0.0.0.0:{}", http_port);
    tracing::info!("  gRPC:      grpc://0.0.0.0:{}", grpc_port);
    tracing::info!("  REST API:  POST http://localhost:{}/query", http_port);
    tracing::info!("  MCP/SSE:   GET  http://localhost:{}/mcp/sse", http_port);

    // Start HTTP and gRPC servers concurrently
    let http_state = app_state.clone();
    let grpc_state = app_state;

    tokio::select! {
        result = async {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", http_port)).await?;
            axum::serve(listener, app).await
        } => {
            result?;
        }
        result = grpc::run_grpc_server(grpc_state, grpc_port) => {
            result?;
        }
    }

    Ok(())
}

async fn run_query(db: Arc<ClawDB>, query: &str) -> anyhow::Result<()> {
    match db.query(query).await {
        Ok(result) => {
            let json = serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn run_repl(db: Arc<ClawDB>) -> anyhow::Result<()> {
    use std::io::{self, BufRead, Write};

    println!("ClawDB REPL - Enter AQL queries (Ctrl+D to exit)");
    println!("Type 'help' for examples\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("aql> ");
        stdout.flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            println!("\nGoodbye!");
            break;
        }

        let query = line.trim();
        if query.is_empty() {
            continue;
        }

        if query == "help" {
            print_help();
            continue;
        }

        if query == "stats" {
            match db.stats().await {
                Ok(stats) => println!("{}", serde_json::to_string_pretty(&stats)?),
                Err(e) => eprintln!("Error: {}", e),
            }
            continue;
        }

        match db.query(query).await {
            Ok(result) => {
                let json = serde_json::to_string_pretty(&result)?;
                println!("{}\n", json);
            }
            Err(e) => {
                eprintln!("Error: {}\n", e);
            }
        }
    }

    Ok(())
}

fn print_help() {
    println!(r#"
ClawDB AQL Examples:

  STORE:
    STORE INTO WORKING (name = "alice", score = 95)
    STORE INTO EPISODIC (event = "login", user = "bob") SCOPE private

  RECALL:
    RECALL FROM WORKING WHERE score > 80
    RECALL FROM EPISODIC ORDER BY created_at DESC LIMIT 5

  SCAN:
    SCAN FROM WORKING WINDOW LAST 10

  AGGREGATE:
    RECALL FROM WORKING AGGREGATE COUNT(*), AVG(score)

  LINK:
    LINK FROM WORKING WHERE name = "alice" TO EPISODIC WHERE event = "login" AS "performed"

  PIPELINE:
    PIPELINE test TIMEOUT 100ms {{ STORE INTO WORKING (x = 1) }} THEN {{ RECALL FROM WORKING }}

Commands:
    stats  - Show database statistics
    help   - Show this help
    Ctrl+D - Exit REPL
"#);
}
