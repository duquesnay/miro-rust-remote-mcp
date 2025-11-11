use miro_mcp_server::{Config, MiroMcpServer};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "miro_mcp_server=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Miro MCP HTTP Server");

    // Load configuration
    let config = Arc::new(Config::from_env_or_file()?);
    let port = config.port;

    // Create StreamableHttpService for MCP protocol
    let config_for_factory = Arc::clone(&config);
    let mcp_service = StreamableHttpService::new(
        move || {
            MiroMcpServer::new(&config_for_factory)
                .map_err(|e| std::io::Error::other(e.to_string()))
        },
        Arc::new(LocalSessionManager::default()),
        Default::default(),
    );

    // TODO: Add OAuth 2.1 DCR endpoints
    // TODO: Merge with http_server.rs OAuth callback endpoint

    // Build Axum router
    let app = axum::Router::new().nest_service("/mcp", mcp_service);

    info!("Starting HTTP server on 0.0.0.0:{}", port);

    // Start server
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down HTTP server");
        })
        .await?;

    Ok(())
}
