#[cfg(feature = "stdio-mcp")]
use miro_mcp_server::MiroMcpServer;
use miro_mcp_server::{run_server_adr002, Config, TokenValidator};
#[cfg(feature = "stdio-mcp")]
use rmcp::transport::stdio;
#[cfg(feature = "stdio-mcp")]
use rmcp::ServiceExt;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "oauth-proxy")]
use miro_mcp_server::oauth::{cookie_manager::CookieManager, proxy_provider::MiroOAuthProvider};

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
    let config = Arc::new(Config::from_env_or_file()?);
    info!("Configuration loaded successfully");

    // Create token validator for HTTP server
    let token_validator = Arc::new(TokenValidator::new());

    // Start ADR-002 Resource Server HTTP server in background task
    let http_token_validator = Arc::clone(&token_validator);
    let http_config = Arc::clone(&config);
    let http_port = config.port;

    tokio::spawn(async move {
        #[cfg(feature = "oauth-proxy")]
        {
            let oauth_provider = Arc::new(MiroOAuthProvider::new(
                http_config.client_id.clone(),
                http_config.client_secret.clone(),
                http_config.redirect_uri.clone(),
            ));
            let cookie_manager = Arc::new(CookieManager::new(&http_config.encryption_key));

            if let Err(e) = run_server_adr002(
                http_port,
                http_token_validator,
                http_config,
                oauth_provider,
                cookie_manager,
            )
            .await
            {
                eprintln!("HTTP server error: {}", e);
            }
        }

        #[cfg(not(feature = "oauth-proxy"))]
        {
            if let Err(e) = run_server_adr002(http_port, http_token_validator, http_config).await {
                eprintln!("HTTP server error: {}", e);
            }
        }
    });

    info!("ADR-002 Resource Server HTTP started on port {}", http_port);

    // Run stdio MCP server (if enabled)
    #[cfg(feature = "stdio-mcp")]
    {
        let mcp_server = MiroMcpServer::new(&config)?;
        info!("MCP server initialized");

        // Run MCP server with stdio transport and wait
        let service = mcp_server.serve(stdio()).await?;
        service.waiting().await?;
    }

    #[cfg(not(feature = "stdio-mcp"))]
    {
        info!("stdio-mcp feature not enabled, keeping HTTP server running");
        // Keep the HTTP server running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    }

    Ok(())
}
