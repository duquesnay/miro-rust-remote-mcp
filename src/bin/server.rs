//! HTTP server binary for ADR-002 Resource Server + ADR-004 Proxy OAuth
//!
//! This binary starts the HTTP server with:
//! - OAuth metadata endpoint (AUTH14 - updated for proxy pattern)
//! - OAuth proxy endpoints (AUTH11 - authorize, callback, token)
//! - Bearer token authentication middleware (AUTH7+AUTH8+AUTH9)
//! - MCP tools (list_boards, get_board)

use miro_mcp_server::{Config, TokenValidator};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "oauth-proxy")]
use miro_mcp_server::oauth::{cookie_manager::CookieManager, proxy_provider::MiroOAuthProvider};

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
        "Starting ADR-002 Resource Server + ADR-004 Proxy OAuth (HTTP only)"
    );

    // Load configuration from environment variables
    let config = Arc::new(Config::from_env_or_file()?);
    info!("Configuration loaded from environment");

    // Create token validator (AUTH8+AUTH9)
    let token_validator = Arc::new(TokenValidator::new());

    // Create OAuth provider and cookie manager (AUTH10+AUTH12)
    #[cfg(feature = "oauth-proxy")]
    let oauth_provider = Arc::new(MiroOAuthProvider::new(
        config.client_id.clone(),
        config.client_secret.clone(),
        config.redirect_uri.clone(),
    ));

    #[cfg(feature = "oauth-proxy")]
    let cookie_manager = Arc::new(CookieManager::new(&config.encryption_key));

    // Get port from environment or use config default
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(config.port);

    info!("Token validator initialized with LRU cache (5-min TTL, 100 capacity)");
    #[cfg(feature = "oauth-proxy")]
    info!("OAuth proxy components initialized (provider + cookie manager)");
    info!("Starting HTTP server on 0.0.0.0:{}", port);

    // Start HTTP server
    #[cfg(feature = "oauth-proxy")]
    miro_mcp_server::run_server_adr002(
        port,
        token_validator,
        config,
        oauth_provider,
        cookie_manager,
    )
    .await?;

    #[cfg(not(feature = "oauth-proxy"))]
    miro_mcp_server::run_server_adr002(port, token_validator, config).await?;

    Ok(())
}
