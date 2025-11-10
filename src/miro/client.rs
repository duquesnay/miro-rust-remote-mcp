use crate::auth::{AuthError, MiroOAuthClient, TokenStore};
use crate::miro::types::{
    Board, BoardsResponse, BulkCreateRequest, BulkCreateResponse, Caption, ConnectorResponse,
    ConnectorStyle, CreateBoardRequest, CreateBoardResponse, CreateConnectorRequest,
    CreateFrameRequest, CreateShapeRequest, CreateStickyNoteRequest, CreateTextRequest,
    FrameResponse, Geometry, Item, ItemsResponse, Parent, Position, ShapeResponse,
    StickyNoteResponse, TextResponse, UpdateItemRequest,
};
use reqwest::StatusCode;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error types for Miro API operations
#[derive(Debug, thiserror::Error)]
pub enum MiroError {
    #[error("Authentication error: {0}")]
    AuthError(#[from] AuthError),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Unauthorized - token may be expired")]
    Unauthorized,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid bulk operation: {0}")]
    BulkOperationError(String),
}

/// Miro API client with automatic token refresh
pub struct MiroClient {
    http_client: reqwest::Client,
    token_store: Arc<RwLock<TokenStore>>,
    oauth_client: Arc<MiroOAuthClient>,
}

impl MiroClient {
    /// Create a new Miro API client
    pub fn new(token_store: TokenStore, oauth_client: MiroOAuthClient) -> Result<Self, MiroError> {
        let http_client = reqwest::Client::builder()
            .user_agent("miro-mcp-server/0.1.0")
            .build()?;

        Ok(Self {
            http_client,
            token_store: Arc::new(RwLock::new(token_store)),
            oauth_client: Arc::new(oauth_client),
        })
    }

    /// Helper to construct Parent from optional parent_id
    fn make_parent(parent_id: Option<String>) -> Option<Parent> {
        parent_id.map(|id| Parent { id })
    }

    /// Get a valid access token, refreshing if necessary
    async fn get_valid_token(&self) -> Result<String, MiroError> {
        let token_store = self.token_store.read().await;
        let tokens = token_store.load()?;

        // Check if token is expired
        if tokens.is_expired() {
            drop(token_store); // Release read lock

            // Refresh the token
            let refresh_token = tokens.refresh_token.ok_or(AuthError::NoToken)?;

            let new_tokens = self
                .oauth_client
                .refresh_access_token(refresh_token)
                .await?;

            // Save the new tokens
            let token_store = self.token_store.write().await;
            token_store.save(&new_tokens)?;

            Ok(new_tokens.access_token)
        } else {
            Ok(tokens.access_token)
        }
    }

    /// Make an authenticated GET request to Miro API
    pub async fn get(&self, path: &str) -> Result<Value, MiroError> {
        self.request("GET", path, None).await
    }

    /// Make an authenticated POST request to Miro API
    pub async fn post(&self, path: &str, body: Option<Value>) -> Result<Value, MiroError> {
        self.request("POST", path, body).await
    }

    /// Make an authenticated PATCH request to Miro API
    pub async fn patch(&self, path: &str, body: Option<Value>) -> Result<Value, MiroError> {
        self.request("PATCH", path, body).await
    }

    /// Make an authenticated DELETE request to Miro API
    pub async fn delete(&self, path: &str) -> Result<Value, MiroError> {
        self.request("DELETE", path, None).await
    }

    /// List all accessible Miro boards
    pub async fn list_boards(&self) -> Result<Vec<Board>, MiroError> {
        let response = self.get("/boards").await?;
        let boards_response: BoardsResponse = serde_json::from_value(response)?;
        Ok(boards_response.data)
    }

    /// Create a new Miro board
    pub async fn create_board(
        &self,
        name: String,
        description: Option<String>,
    ) -> Result<Board, MiroError> {
        let request_body = CreateBoardRequest { name, description };
        let json_body = serde_json::to_value(&request_body)?;
        let response = self.post("/boards", Some(json_body)).await?;
        let board: CreateBoardResponse = serde_json::from_value(response)?;
        Ok(Board {
            id: board.id,
            name: board.name,
            description: board.description,
            created_at: board.created_at,
        })
    }

    /// Create a sticky note on a board
    pub async fn create_sticky_note(
        &self,
        board_id: &str,
        content: String,
        x: f64,
        y: f64,
        color: String,
        parent_id: Option<String>,
    ) -> Result<StickyNoteResponse, MiroError> {
        let request_body = CreateStickyNoteRequest {
            data: crate::miro::types::StickyNoteData {
                content,
                shape: Some("square".to_string()),
            },
            style: crate::miro::types::StickyNoteStyle { fill_color: color },
            position: Position {
                x,
                y,
                origin: Some("center".to_string()),
            },
            geometry: Geometry {
                width: 200.0,
                height: None,
            },
            parent: Self::make_parent(parent_id),
        };
        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/sticky_notes", board_id);
        let response = self.post(&path, Some(json_body)).await?;
        let note: StickyNoteResponse = serde_json::from_value(response)?;
        Ok(note)
    }

    /// Create a shape on a board
    #[allow(clippy::too_many_arguments)]
    pub async fn create_shape(
        &self,
        board_id: &str,
        shape_type: String,
        fill_color: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        content: Option<String>,
        parent_id: Option<String>,
    ) -> Result<ShapeResponse, MiroError> {
        let shape_data = crate::miro::types::ShapeData {
            content,
            shape: shape_type,
        };
        let shape_style = crate::miro::types::ShapeStyle {
            fill_color,
            border_color: None,
            border_width: None,
        };
        let position = Position { x, y, origin: None };
        let geometry = Geometry {
            width,
            height: Some(height),
        };

        let request_body = CreateShapeRequest {
            data: shape_data,
            style: shape_style,
            position,
            geometry,
            parent: Self::make_parent(parent_id),
        };

        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/shapes", board_id);
        let response = self.post(&path, Some(json_body)).await?;
        let shape: ShapeResponse = serde_json::from_value(response)?;
        Ok(shape)
    }

    /// Create text on a board
    pub async fn create_text(
        &self,
        board_id: &str,
        content: String,
        x: f64,
        y: f64,
        width: f64,
        parent_id: Option<String>,
    ) -> Result<TextResponse, MiroError> {
        let request_body = CreateTextRequest {
            data: crate::miro::types::TextData { content },
            position: Position { x, y, origin: None },
            geometry: Geometry {
                width,
                height: None,
            },
            parent: Self::make_parent(parent_id),
        };
        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/texts", board_id);
        let response = self.post(&path, Some(json_body)).await?;
        let text: TextResponse = serde_json::from_value(response)?;
        Ok(text)
    }

    /// Create a frame on a board
    #[allow(clippy::too_many_arguments)]
    pub async fn create_frame(
        &self,
        board_id: &str,
        title: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill_color: Option<String>,
        parent_id: Option<String>,
    ) -> Result<FrameResponse, MiroError> {
        let frame_data = crate::miro::types::FrameData {
            title,
            frame_type: "frame".to_string(),
        };
        let frame_style = crate::miro::types::FrameStyle {
            fill_color: fill_color.unwrap_or_else(|| "light_gray".to_string()),
        };
        let position = Position { x, y, origin: None };
        let geometry = Geometry {
            width,
            height: Some(height),
        };

        let request_body = CreateFrameRequest {
            data: frame_data,
            style: frame_style,
            position,
            geometry,
            parent: Self::make_parent(parent_id),
        };

        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/frames", board_id);
        let response = self.post(&path, Some(json_body)).await?;
        let frame: FrameResponse = serde_json::from_value(response)?;
        Ok(frame)
    }

    /// Create a connector between two items on a board
    #[allow(clippy::too_many_arguments)]
    pub async fn create_connector(
        &self,
        board_id: &str,
        start_item_id: String,
        end_item_id: String,
        stroke_color: Option<String>,
        stroke_width: Option<f64>,
        start_cap: Option<String>,
        end_cap: Option<String>,
        captions: Option<Vec<Caption>>,
    ) -> Result<ConnectorResponse, MiroError> {
        let style = if stroke_color.is_some()
            || stroke_width.is_some()
            || start_cap.is_some()
            || end_cap.is_some()
        {
            Some(ConnectorStyle {
                stroke_color,
                stroke_width,
                start_cap,
                end_cap,
            })
        } else {
            None
        };

        let request_body = CreateConnectorRequest {
            start_item: start_item_id,
            end_item: end_item_id,
            style,
            captions,
        };

        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/connectors", board_id);
        let response = self.post(&path, Some(json_body)).await?;
        let connector: ConnectorResponse = serde_json::from_value(response)?;
        Ok(connector)
    }

    /// List items on a board with optional type filtering and parent filtering
    pub async fn list_items(
        &self,
        board_id: &str,
        item_types: Option<Vec<&str>>,
        parent_id: Option<&str>,
    ) -> Result<Vec<Item>, MiroError> {
        let mut path = format!("/boards/{}/items", board_id);
        let mut query_params = Vec::new();

        // Add type filter if provided
        if let Some(types) = item_types {
            let type_filter = types.join(",");
            query_params.push(format!("type={}", type_filter));
        }

        // Add parent filter if provided
        if let Some(parent) = parent_id {
            query_params.push(format!("parent.id={}", parent));
        }

        // Append query string if there are parameters
        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        let response = self.get(&path).await?;
        let items_response: ItemsResponse = serde_json::from_value(response)?;
        Ok(items_response.data)
    }

    /// Update item properties (position, content, style, geometry, parent)
    #[allow(clippy::too_many_arguments)]
    pub async fn update_item(
        &self,
        board_id: &str,
        item_id: &str,
        position: Option<Position>,
        data: Option<Value>,
        style: Option<Value>,
        geometry: Option<Geometry>,
        parent_id: Option<String>,
    ) -> Result<Item, MiroError> {
        let request_body = UpdateItemRequest {
            position,
            data,
            style,
            geometry,
            parent: Self::make_parent(parent_id),
        };

        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/items/{}", board_id, item_id);
        let response = self.patch(&path, Some(json_body)).await?;
        let item: Item = serde_json::from_value(response)?;
        Ok(item)
    }

    /// Delete an item from a board
    pub async fn delete_item(&self, board_id: &str, item_id: &str) -> Result<(), MiroError> {
        let path = format!("/boards/{}/items/{}", board_id, item_id);
        let _response = self.delete(&path).await?;
        Ok(())
    }

    /// Bulk create multiple items in a single API call (max 20 items per request)
    pub async fn bulk_create_items(
        &self,
        board_id: &str,
        items: Vec<crate::miro::types::BulkItemRequest>,
    ) -> Result<Vec<Item>, MiroError> {
        // Validate item count (API limit is 20 items per request)
        const MAX_BULK_ITEMS: usize = 20;
        if items.is_empty() {
            return Err(MiroError::BulkOperationError(
                "Items array cannot be empty".to_string(),
            ));
        }
        if items.len() > MAX_BULK_ITEMS {
            return Err(MiroError::BulkOperationError(format!(
                "Too many items in bulk request: {} (maximum is {})",
                items.len(),
                MAX_BULK_ITEMS
            )));
        }

        let request_body = BulkCreateRequest { items };
        let json_body = serde_json::to_value(&request_body)?;
        let path = format!("/boards/{}/items", board_id);
        let response = self.post(&path, Some(json_body)).await?;
        let bulk_response: BulkCreateResponse = serde_json::from_value(response)?;
        Ok(bulk_response.data)
    }

    /// Make an authenticated request with automatic retry on 401
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, MiroError> {
        let url = format!("https://api.miro.com/v2{}", path);

        // First attempt
        match self.execute_request(method, &url, body.clone()).await {
            Ok(response) => Ok(response),
            Err(MiroError::Unauthorized) => {
                // Token might be expired, force refresh and retry once
                let token_store = self.token_store.read().await;
                let tokens = token_store.load()?;
                drop(token_store);

                if let Some(refresh_token) = tokens.refresh_token {
                    let new_tokens = self
                        .oauth_client
                        .refresh_access_token(refresh_token)
                        .await?;

                    let token_store = self.token_store.write().await;
                    token_store.save(&new_tokens)?;
                    drop(token_store);

                    // Retry the request with new token
                    self.execute_request(method, &url, body).await
                } else {
                    Err(MiroError::Unauthorized)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Execute a single HTTP request
    async fn execute_request(
        &self,
        method: &str,
        url: &str,
        body: Option<Value>,
    ) -> Result<Value, MiroError> {
        let token = self.get_valid_token().await?;

        let mut request = match method {
            "GET" => self.http_client.get(url),
            "POST" => self.http_client.post(url),
            "PATCH" => self.http_client.patch(url),
            "DELETE" => self.http_client.delete(url),
            _ => {
                return Err(MiroError::ApiError {
                    status: 400,
                    message: format!("Unsupported HTTP method: {}", method),
                })
            }
        };

        request = request.bearer_auth(&token);

        if let Some(body_value) = body {
            request = request.json(&body_value);
        }

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let json = response.json().await?;
                Ok(json)
            }
            StatusCode::NO_CONTENT => Ok(Value::Null),
            StatusCode::UNAUTHORIZED => Err(MiroError::Unauthorized),
            StatusCode::TOO_MANY_REQUESTS => Err(MiroError::RateLimitExceeded),
            status => {
                let message = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                Err(MiroError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

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
    fn test_client_creation() {
        let config = get_test_config();
        let token_store = TokenStore::new(config.encryption_key).unwrap();
        let oauth_client = MiroOAuthClient::new(&config).unwrap();

        let result = MiroClient::new(token_store, oauth_client);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sticky_note_request_construction() {
        let position = Position {
            x: 100.0,
            y: 200.0,
            origin: Some("center".to_string()),
        };
        let geometry = Geometry {
            width: 200.0,
            height: None,
        };

        assert_eq!(position.x, 100.0);
        assert_eq!(position.y, 200.0);
        assert_eq!(geometry.width, 200.0);
    }

    #[test]
    fn test_shape_response_deserialization() {
        let json = r#"{
            "id": "shape-456",
            "data": {
                "content": "<p>Shape content</p>",
                "shape": "rectangle"
            },
            "style": {
                "fillColor": "light_blue",
                "borderColor": "blue",
                "borderWidth": "2"
            }
        }"#;

        let response: ShapeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "shape-456");
    }

    #[test]
    fn test_text_response_deserialization() {
        let json = r#"{
            "id": "text-789",
            "data": {
                "content": "Sample text"
            }
        }"#;

        let response: TextResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "text-789");
    }

    #[test]
    fn test_frame_response_deserialization() {
        let json = r#"{
            "id": "frame-012",
            "data": {
                "title": "My Frame",
                "type": "frame"
            },
            "style": {
                "fillColor": "light_gray"
            }
        }"#;

        let response: FrameResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "frame-012");
    }

    #[test]
    fn test_item_update_request_construction() {
        let position = Position {
            x: 150.0,
            y: 250.0,
            origin: None,
        };
        let geometry = Geometry {
            width: 300.0,
            height: Some(200.0),
        };

        let request = UpdateItemRequest {
            position: Some(position),
            data: None,
            style: None,
            geometry: Some(geometry),
            parent: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("150"));
        assert!(json.contains("250"));
        assert!(json.contains("300"));
        assert!(json.contains("200"));
    }

    #[test]
    fn test_connector_request_construction_with_style() {
        let style = ConnectorStyle {
            stroke_color: Some("blue".to_string()),
            stroke_width: Some(2.5),
            start_cap: Some("none".to_string()),
            end_cap: Some("arrow".to_string()),
        };

        let captions = vec![Caption {
            content: "depends on".to_string(),
            position: Some(0.5),
        }];

        let request = CreateConnectorRequest {
            start_item: "shape-1".to_string(),
            end_item: "shape-2".to_string(),
            style: Some(style),
            captions: Some(captions),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"startItem\":\"shape-1\""));
        assert!(json.contains("\"endItem\":\"shape-2\""));
        assert!(json.contains("\"strokeColor\":\"blue\""));
    }

    #[test]
    fn test_connector_response_deserialization() {
        let json = r#"{
            "id": "connector-999",
            "startItem": "node-a",
            "endItem": "node-b"
        }"#;

        let response: ConnectorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "connector-999");
        assert_eq!(response.start_item, Some("node-a".to_string()));
        assert_eq!(response.end_item, Some("node-b".to_string()));
    }

    #[test]
    fn test_bulk_create_validation_empty_items() {
        let config = get_test_config();
        let token_store = TokenStore::new(config.encryption_key).unwrap();
        let oauth_client = MiroOAuthClient::new(&config).unwrap();
        let client = MiroClient::new(token_store, oauth_client).unwrap();

        // Test validation: empty items array should fail
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async { client.bulk_create_items("board-123", vec![]).await });

        assert!(result.is_err());
        match result {
            Err(MiroError::BulkOperationError(msg)) => {
                assert!(msg.contains("cannot be empty"));
            }
            _ => panic!("Expected BulkOperationError"),
        }
    }

    #[test]
    fn test_bulk_create_validation_too_many_items() {
        let config = get_test_config();
        let token_store = TokenStore::new(config.encryption_key).unwrap();
        let oauth_client = MiroOAuthClient::new(&config).unwrap();
        let client = MiroClient::new(token_store, oauth_client).unwrap();

        // Create 21 items (exceeds limit of 20)
        let items: Vec<_> = (0..21)
            .map(|i| {
                use crate::miro::types::{BulkItemRequest, Geometry, Position, TextData};

                BulkItemRequest::Text {
                    item_type: "text".to_string(),
                    data: TextData {
                        content: format!("Text {}", i),
                    },
                    position: Position {
                        x: i as f64 * 100.0,
                        y: 0.0,
                        origin: None,
                    },
                    geometry: Geometry {
                        width: 100.0,
                        height: None,
                    },
                    parent: None,
                }
            })
            .collect();

        // Test validation: >20 items should fail
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async { client.bulk_create_items("board-123", items).await });

        assert!(result.is_err());
        match result {
            Err(MiroError::BulkOperationError(msg)) => {
                assert!(msg.contains("Too many items"));
                assert!(msg.contains("21"));
                assert!(msg.contains("20"));
            }
            _ => panic!("Expected BulkOperationError"),
        }
    }
}
