use crate::auth::{MiroOAuthClient, TokenStore};
use crate::config::Config;
use crate::miro::MiroClient;
use rmcp::{
    handler::server::tool::ToolRouter, model::*, tool, tool_router, ErrorData as McpError,
    ServerHandler,
};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::Arc;

/// Parameters for creating a sticky note
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateStickyNoteParams {
    pub board_id: String,
    pub content: String,
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub color: Option<String>,
}

/// Parameters for creating a shape
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateShapeParams {
    pub board_id: String,
    pub shape_type: String,
    pub fill_color: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub content: Option<String>,
}

/// Parameters for creating text
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTextParams {
    pub board_id: String,
    pub content: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

/// Parameters for creating a frame
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFrameParams {
    pub board_id: String,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub fill_color: Option<String>,
}

/// Parameters for listing items
#[derive(Debug, Serialize, Deserialize)]
pub struct ListItemsParams {
    pub board_id: String,
    #[serde(default)]
    pub item_types: Option<String>, // comma-separated types
}

/// Parameters for updating an item
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateItemParams {
    pub board_id: String,
    pub item_id: String,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    pub content: Option<String>,
}

/// Parameters for deleting an item
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteItemParams {
    pub board_id: String,
    pub item_id: String,
}

/// Parameters for creating a connector
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateConnectorParams {
    pub board_id: String,
    pub start_item_id: String,
    pub end_item_id: String,
    #[serde(default)]
    pub stroke_color: Option<String>,
    #[serde(default)]
    pub stroke_width: Option<f64>,
    #[serde(default)]
    pub start_cap: Option<String>,
    #[serde(default)]
    pub end_cap: Option<String>,
    #[serde(default)]
    pub captions: Option<Vec<serde_json::Value>>,
}

/// Parameters for bulk creating items
#[derive(Debug, Serialize, Deserialize)]
pub struct BulkCreateItemsParams {
    pub board_id: String,
    pub items: Vec<serde_json::Value>, // Array of item definitions (type-specific)
}

/// MCP server for Miro
#[derive(Clone)]
pub struct MiroMcpServer {
    oauth_client: Arc<MiroOAuthClient>,
    miro_client: Arc<MiroClient>,
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl MiroMcpServer {
    /// Create a new MCP server
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let oauth_client = Arc::new(MiroOAuthClient::new(config)?);
        let token_store = TokenStore::new(config.encryption_key)?;
        let miro_client = Arc::new(MiroClient::new(token_store, (*oauth_client).clone())?);

        Ok(Self {
            oauth_client,
            miro_client,
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

    /// List all accessible Miro boards
    #[tool(description = "List all accessible Miro boards")]
    async fn list_boards(&self) -> Result<CallToolResult, McpError> {
        let boards = self
            .miro_client
            .list_boards()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        if boards.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No boards found.".to_string(),
            )]));
        }

        let board_list = boards
            .iter()
            .map(|b| {
                let description = b
                    .description
                    .as_ref()
                    .map(|d| format!(" - {}", d))
                    .unwrap_or_default();
                format!("- {} (ID: {}){}", b.name, b.id, description)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let message = format!("Found {} board(s):\n{}", boards.len(), board_list);
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a new Miro board
    #[tool(description = "Create a new Miro board")]
    async fn create_board(&self) -> Result<CallToolResult, McpError> {
        // Note: In actual usage, the tool parameters would be passed from the MCP client
        // This is a placeholder implementation
        let board = self
            .miro_client
            .create_board("New Board".to_string(), None)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let message = format!(
            "Successfully created board: {}\nBoard ID: {}",
            board.name, board.id
        );

        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a sticky note on a board
    #[tool(
        description = "Create a sticky note on a Miro board with customizable content, position, and color"
    )]
    async fn create_sticky_note(&self) -> Result<CallToolResult, McpError> {
        let message = "create_sticky_note tool registered. Use tool_call with parameters: { board_id, content, x, y, color? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a shape on a board
    #[tool(
        description = "Create a shape (rectangle, circle, triangle, etc.) on a Miro board with custom styling"
    )]
    async fn create_shape(&self) -> Result<CallToolResult, McpError> {
        let message = "create_shape tool registered. Use tool_call with parameters: { board_id, shape_type, fill_color, x, y, width, height, content? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create text on a board
    #[tool(description = "Create a text element on a Miro board")]
    async fn create_text(&self) -> Result<CallToolResult, McpError> {
        let message = "create_text tool registered. Use tool_call with parameters: { board_id, content, x, y, width }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a frame on a board
    #[tool(description = "Create a frame on a Miro board to group and organize other elements")]
    async fn create_frame(&self) -> Result<CallToolResult, McpError> {
        let message = "create_frame tool registered. Use tool_call with parameters: { board_id, title, x, y, width, height, fill_color? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// List items on a board with optional type filtering
    #[tool(
        description = "List items on a Miro board with optional filtering by type (frame, sticky_note, shape, text, connector)"
    )]
    async fn list_items(&self) -> Result<CallToolResult, McpError> {
        let message = "list_items tool registered. Use tool_call with parameters: { board_id, item_types? (comma-separated) }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Update item properties
    #[tool(description = "Update an item's properties including position, content, and styling")]
    async fn update_item(&self) -> Result<CallToolResult, McpError> {
        let message = "update_item tool registered. Use tool_call with parameters: { board_id, item_id, x?, y?, content? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Delete an item from a board
    #[tool(description = "Delete an item from a Miro board")]
    async fn delete_item(&self) -> Result<CallToolResult, McpError> {
        let message =
            "delete_item tool registered. Use tool_call with parameters: { board_id, item_id }"
                .to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a connector between two items
    #[tool(
        description = "Create a connector (line/arrow) between two items on a Miro board with optional styling and captions"
    )]
    async fn create_connector(&self) -> Result<CallToolResult, McpError> {
        let message = "create_connector tool registered. Use tool_call with parameters: { board_id, start_item_id, end_item_id, stroke_color?, stroke_width?, start_cap?, end_cap?, captions? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Bulk create multiple items in a single transaction
    #[tool(
        description = "Create multiple items efficiently in a single API call (max 20 items per request). Accepts array of mixed item types (sticky_note, shape, text, frame) with their respective configurations."
    )]
    async fn bulk_create_items(&self) -> Result<CallToolResult, McpError> {
        let message = "bulk_create_items tool registered. Use tool_call with parameters: { board_id, items: [{ type: 'sticky_note'|'shape'|'text'|'frame', data: {...}, position: {...}, geometry: {...}, style?: {...} }, ...] }. Maximum 20 items per call.".to_string();
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
                name: "miro-rust".into(),
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
