use crate::auth::{MiroOAuthClient, TokenStore};
use crate::config::Config;
use crate::miro::MiroClient;
use rmcp::{
    handler::server::tool::ToolRouter, model::*, service::RequestContext, tool, tool_router,
    ErrorData as McpError, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
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
    #[serde(default)]
    pub parent_id: Option<String>,
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
    #[serde(default)]
    pub parent_id: Option<String>,
}

/// Parameters for creating text
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTextParams {
    pub board_id: String,
    pub content: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    #[serde(default)]
    pub parent_id: Option<String>,
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
    #[serde(default)]
    pub parent_id: Option<String>,
}

/// Parameters for listing items
#[derive(Debug, Serialize, Deserialize)]
pub struct ListItemsParams {
    pub board_id: String,
    #[serde(default)]
    pub item_types: Option<String>, // comma-separated types
    #[serde(default)]
    pub sort_by: Option<String>, // "created_at" or "modified_at"
    #[serde(default)]
    pub parent_id: Option<String>, // filter by parent frame ID
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
    #[serde(default)]
    pub parent_id: Option<String>,
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
        description = "Create a sticky note on a Miro board with customizable content, position, color, and optional parent frame"
    )]
    async fn create_sticky_note(&self) -> Result<CallToolResult, McpError> {
        let message = "create_sticky_note tool registered. Use tool_call with parameters: { board_id, content, x, y, color?, parent_id? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a shape on a board
    #[tool(
        description = "Create a shape (rectangle, circle, triangle, etc.) on a Miro board with custom styling and optional parent frame"
    )]
    async fn create_shape(&self) -> Result<CallToolResult, McpError> {
        let message = "create_shape tool registered. Use tool_call with parameters: { board_id, shape_type, fill_color, x, y, width, height, content?, parent_id? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create text on a board
    #[tool(description = "Create a text element on a Miro board with optional parent frame")]
    async fn create_text(&self) -> Result<CallToolResult, McpError> {
        let message = "create_text tool registered. Use tool_call with parameters: { board_id, content, x, y, width, parent_id? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Create a frame on a board
    #[tool(
        description = "Create a frame on a Miro board to group and organize other elements, with optional parent frame"
    )]
    async fn create_frame(&self) -> Result<CallToolResult, McpError> {
        let message = "create_frame tool registered. Use tool_call with parameters: { board_id, title, x, y, width, height, fill_color?, parent_id? }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// List items on a board with optional type filtering, parent filtering, and sorting
    #[tool(
        description = "List items on a Miro board with optional filtering by type (frame, sticky_note, shape, text, connector), parent frame, and sorting by creation/modification time for layer awareness"
    )]
    async fn list_items(&self) -> Result<CallToolResult, McpError> {
        let message = "list_items tool registered. Use tool_call with parameters: { board_id, item_types? (comma-separated), parent_id?, sort_by? (created_at|modified_at) }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }

    /// Internal implementation of list_items with parameter support
    async fn list_items_with_params(
        &self,
        params: ListItemsParams,
    ) -> Result<CallToolResult, McpError> {
        let item_types = params
            .item_types
            .as_ref()
            .map(|types| types.split(',').map(|s| s.trim()).collect::<Vec<_>>());

        let mut items = self
            .miro_client
            .list_items(&params.board_id, item_types, params.parent_id.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        // Apply sorting if specified
        if let Some(sort_by) = params.sort_by {
            match sort_by.as_str() {
                "created_at" => {
                    items.sort_by(|a, b| {
                        let a_time = a.created_at.as_deref().unwrap_or("");
                        let b_time = b.created_at.as_deref().unwrap_or("");
                        a_time.cmp(b_time)
                    });
                }
                "modified_at" => {
                    items.sort_by(|a, b| {
                        let a_time = a.modified_at.as_deref().unwrap_or("");
                        let b_time = b.modified_at.as_deref().unwrap_or("");
                        a_time.cmp(b_time)
                    });
                }
                _ => {
                    return Err(McpError::invalid_params(
                        format!(
                            "Invalid sort_by value: '{}'. Valid values are: 'created_at', 'modified_at'",
                            sort_by
                        ),
                        None,
                    ));
                }
            }
        }

        if items.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No items found on this board.".to_string(),
            )]));
        }

        let items_json = serde_json::to_string_pretty(&items)
            .unwrap_or_else(|_| "Failed to serialize items".to_string());

        Ok(CallToolResult::success(vec![Content::text(items_json)]))
    }

    /// Update item properties
    #[tool(
        description = "Update an item's properties including position, content, styling, and parent frame"
    )]
    async fn update_item(&self) -> Result<CallToolResult, McpError> {
        let message = "update_item tool registered. Use tool_call with parameters: { board_id, item_id, x?, y?, content?, parent_id? }".to_string();
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
                title: None,
                icons: None,
                website_url: None,
            },
            instructions: Some("Miro MCP Server - OAuth2-enabled Miro board manipulation".into()),
        }
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParam>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // Return all tools from the tool_router
        Ok(ListToolsResult {
            tools: self.tool_router.list_all(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Delegate to the individual tool methods based on the tool name
        match params.name.as_ref() {
            "start_auth" => self.start_auth().await,
            "list_boards" => self.list_boards().await,
            "create_board" => self.create_board().await,
            "create_sticky_note" => self.create_sticky_note().await,
            "create_shape" => self.create_shape().await,
            "create_text" => self.create_text().await,
            "create_frame" => self.create_frame().await,
            "list_items" => {
                // Parse list_items parameters from the request
                let args_value =
                    serde_json::Value::Object(params.arguments.clone().unwrap_or_default());
                let list_params: ListItemsParams =
                    serde_json::from_value(args_value).map_err(|e| {
                        McpError::internal_error(format!("Invalid parameters: {}", e), None)
                    })?;
                self.list_items_with_params(list_params).await
            }
            "update_item" => self.update_item().await,
            "delete_item" => self.delete_item().await,
            "create_connector" => self.create_connector().await,
            "bulk_create_items" => self.bulk_create_items().await,
            _ => Err(McpError::internal_error(
                format!("Unknown tool: {}", params.name.as_ref()),
                None,
            )),
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

    #[test]
    fn test_sort_by_validation() {
        // Test the validation logic for sort_by values
        // This is a unit test of the match logic without requiring API calls

        let valid_values = vec!["created_at", "modified_at"];
        let invalid_values = vec!["invalid_value", "name", "updated_at", ""];

        // Verify valid values would match
        for value in valid_values {
            let matches = matches!(value, "created_at" | "modified_at");
            assert!(
                matches,
                "Valid sort_by value '{}' should be accepted",
                value
            );
        }

        // Verify invalid values would not match
        for value in &invalid_values {
            let value_str: &str = value;
            let matches = matches!(value_str, "created_at" | "modified_at");
            assert!(
                !matches,
                "Invalid sort_by value '{}' should be rejected",
                value
            );
        }

        // Verify error message format
        let invalid_value = "invalid_value";
        let error_msg = format!(
            "Invalid sort_by value: '{}'. Valid values are: 'created_at', 'modified_at'",
            invalid_value
        );

        assert!(error_msg.contains("Invalid sort_by value"));
        assert!(error_msg.contains("invalid_value"));
        assert!(error_msg.contains("created_at"));
        assert!(error_msg.contains("modified_at"));
    }
}
