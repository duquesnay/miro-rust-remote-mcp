use miro_mcp_server::{Config, MiroMcpServer};
use rmcp::handler::server::router::Router;
use rmcp::ServiceExt;
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

    info!("Starting Miro MCP Server");

    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded successfully");

    // Create MCP server
    let mcp_server = MiroMcpServer::new(&config)?;
    let router = Router::new(mcp_server);

    info!("MCP server initialized");

    // Run server with stdio transport
    let transport = rmcp::transport::stdio();
    router.serve(transport).await?;

    Ok(())
}
