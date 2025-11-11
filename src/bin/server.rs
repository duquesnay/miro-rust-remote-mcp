//! Minimal HTTP server binary for testing ADR-002 Resource Server implementation
//!
//! This binary starts only the HTTP server with:
//! - OAuth metadata endpoint (AUTH6)
//! - Bearer token authentication middleware (AUTH7+AUTH8+AUTH9+AUTH10)
//! - MCP tools (list_boards, get_board)
//!
//! No stdio transport, no cookie-based OAuth (ADR-001 deprecated).

use miro_mcp_server::TokenValidator;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with configurable format
    // LOG_FORMAT=json for production (Scaleway Cockpit)
    // LOG_FORMAT=pretty (or unset) for development
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "pretty".to_string());

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "miro_mcp_server=info".into());

    match log_format.as_str() {
        "json" => {
            // JSON format for production (structured logs for Scaleway Cockpit)
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        _ => {
            // Pretty format for development (human-readable)
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
    }

    info!(
        log_format = %log_format,
        "Starting ADR-002 Resource Server (HTTP only)"
    );

    // Create token validator (AUTH8+AUTH9)
    let token_validator = Arc::new(TokenValidator::new());

    // Get port from environment or use default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3010);

    info!("Token validator initialized with LRU cache (5-min TTL, 100 capacity)");
    info!("Starting HTTP server on 0.0.0.0:{}", port);

    // Start HTTP server
    miro_mcp_server::run_server_adr002(port, token_validator).await?;

    Ok(())
}
