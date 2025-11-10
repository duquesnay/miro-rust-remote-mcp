use miro_mcp_server::{Config, MiroMcpServer};
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    // Create MCP server
    let mcp_server = MiroMcpServer::new(&config)?;

    info!("MCP server initialized");

    // Run server with stdio transport and wait
    let service = mcp_server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
