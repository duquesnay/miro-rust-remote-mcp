use miro_mcp_server::{
    Config, CookieStateManager, CookieTokenManager, MiroMcpServer, MiroOAuthClient, TokenStore,
};
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (for local development)
    // Silently ignore if .env file doesn't exist (production uses env vars directly)
    let _ = dotenvy::dotenv();

    // Initialize tracing - write to stderr to keep stdout clean for MCP JSON protocol
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "miro_mcp_server=info".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr) // Write logs to stderr
                .with_ansi(false), // Disable ANSI colors for clean output
        )
        .init();

    info!("Starting Miro MCP Server");

    // Load configuration (env vars first, then config file)
    let config = Config::from_env_or_file()?;
    info!("Configuration loaded successfully");

    // Create shared OAuth components for HTTP server
    let oauth_client = Arc::new(MiroOAuthClient::new(&config)?);
    let token_store = Arc::new(RwLock::new(TokenStore::new(config.encryption_key)?));
    let cookie_state_manager = CookieStateManager::from_config(config.encryption_key);
    let cookie_token_manager = CookieTokenManager::from_config(config.encryption_key);

    // Start OAuth HTTP server in background task
    let http_oauth_client = Arc::clone(&oauth_client);
    let http_token_store = Arc::clone(&token_store);
    let http_cookie_state_manager = cookie_state_manager.clone();
    let http_cookie_token_manager = cookie_token_manager.clone();
    let http_port = config.port;

    tokio::spawn(async move {
        if let Err(e) = miro_mcp_server::run_server(
            http_port,
            http_oauth_client,
            http_token_store,
            http_cookie_state_manager,
            http_cookie_token_manager,
        )
        .await
        {
            eprintln!("HTTP server error: {}", e);
        }
    });

    info!("OAuth HTTP server started on port {}", http_port);

    // Create MCP server
    let mcp_server = MiroMcpServer::new(&config)?;
    info!("MCP server initialized");

    // Run MCP server with stdio transport and wait
    let service = mcp_server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
