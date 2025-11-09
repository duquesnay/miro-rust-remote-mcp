use crate::auth::MiroOAuthClient;
use crate::config::Config;
use rmcp::{
    handler::server::tool::ToolRouter, model::*, tool, tool_router, ErrorData as McpError,
    ServerHandler,
};
use std::future::Future;
use std::sync::Arc;

/// MCP server for Miro
#[derive(Clone)]
pub struct MiroMcpServer {
    oauth_client: Arc<MiroOAuthClient>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MiroMcpServer {
    /// Create a new MCP server
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let oauth_client = Arc::new(MiroOAuthClient::new(config)?);

        Ok(Self {
            oauth_client,
            tool_router: Self::tool_router(),
        })
    }

    /// Start OAuth2 authentication flow
    #[tool(description = "Start OAuth2 authentication flow with Miro. Returns authorization URL.")]
    async fn start_auth(&self) -> Result<CallToolResult, McpError> {
        let (auth_url, csrf_token) = self
            .oauth_client
            .get_authorization_url()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let message = format!(
            "Authorization URL: {}\n\nState: {}\n\nInstructions: Open the authorization URL in your browser, authorize the application, and you will be redirected to the callback URL with a code parameter.",
            auth_url,
            csrf_token.secret()
        );

        Ok(CallToolResult::success(vec![Content::text(message)]))
    }
}

impl ServerHandler for MiroMcpServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability { list_changed: None }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "miro-mcp-server".into(),
                version: "0.1.0".into(),
            },
            instructions: Some("Miro MCP Server - OAuth2-enabled Miro board manipulation".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_config() -> Config {
        Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
            encryption_key: [0u8; 32],
            port: 3000,
        }
    }

    #[test]
    fn test_server_creation() {
        let config = get_test_config();
        let server = MiroMcpServer::new(&config);
        assert!(server.is_ok());
    }
}
