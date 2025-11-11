use crate::auth::token_validator::UserInfo;
use crate::miro::types::Board;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Tool response envelope
#[derive(Debug, Serialize)]
pub struct ToolResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ToolResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

/// Response for list_boards tool
#[derive(Debug, Serialize, Deserialize)]
pub struct ListBoardsResponse {
    pub boards: Vec<BoardInfo>,
    pub count: usize,
}

/// Board info in tool response
#[derive(Debug, Serialize, Deserialize)]
pub struct BoardInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
}

impl From<Board> for BoardInfo {
    fn from(board: Board) -> Self {
        BoardInfo {
            id: board.id,
            name: board.name,
            description: board.description,
            created_at: board.created_at,
        }
    }
}

/// Response for get_board tool
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBoardResponse {
    pub board: BoardInfo,
}

// ==================== Tool Handlers ====================

/// List accessible Miro boards
///
/// Extracts Bearer token from request extensions (provided by bearer_auth_middleware),
/// then calls Miro API to list boards accessible to the authenticated user.
///
/// # Arguments
///
/// * `token` - Bearer token from request extensions
/// * `user_info` - User info from token validation middleware
///
/// # Returns
///
/// JSON response with list of boards or error
pub async fn list_boards(
    Extension(token): Extension<Arc<String>>,
    Extension(user_info): Extension<Arc<UserInfo>>,
) -> Result<Json<ToolResponse<ListBoardsResponse>>, ToolError> {
    info!(
        user_id = %user_info.user_id,
        "Listing boards for user"
    );

    // Create Miro API client with reqwest
    let http_client = Client::new();

    // Call Miro API to list boards
    match fetch_boards_from_miro(&http_client, token.as_str()).await {
        Ok(boards) => {
            let count = boards.len();
            let board_infos: Vec<BoardInfo> = boards.into_iter().map(BoardInfo::from).collect();

            info!(
                user_id = %user_info.user_id,
                count = count,
                "Successfully listed boards"
            );

            Ok(Json(ToolResponse::ok(ListBoardsResponse {
                boards: board_infos,
                count,
            })))
        }
        Err(e) => {
            warn!(
                user_id = %user_info.user_id,
                error = %e,
                "Failed to list boards"
            );
            Err(ToolError::MiroApiError(e))
        }
    }
}

/// Get details of a specific Miro board
///
/// Extracts Bearer token from request extensions and board_id from path,
/// then calls Miro API to get board details.
///
/// # Arguments
///
/// * `token` - Bearer token from request extensions
/// * `user_info` - User info from token validation middleware
/// * `board_id` - Board ID from URL path
///
/// # Returns
///
/// JSON response with board details or error
pub async fn get_board(
    Extension(token): Extension<Arc<String>>,
    Extension(user_info): Extension<Arc<UserInfo>>,
    Path(board_id): Path<String>,
) -> Result<Json<ToolResponse<GetBoardResponse>>, ToolError> {
    info!(
        user_id = %user_info.user_id,
        board_id = %board_id,
        "Getting board details"
    );

    if board_id.is_empty() {
        warn!("Empty board_id provided");
        return Err(ToolError::InvalidInput(
            "board_id cannot be empty".to_string(),
        ));
    }

    // Create Miro API client with reqwest
    let http_client = Client::new();

    // Call Miro API to get board details
    match fetch_board_from_miro(&http_client, token.as_str(), &board_id).await {
        Ok(board) => {
            info!(
                user_id = %user_info.user_id,
                board_id = %board_id,
                board_name = %board.name,
                "Successfully retrieved board"
            );

            Ok(Json(ToolResponse::ok(GetBoardResponse {
                board: BoardInfo::from(board),
            })))
        }
        Err(e) => {
            warn!(
                user_id = %user_info.user_id,
                board_id = %board_id,
                error = %e,
                "Failed to get board"
            );
            Err(ToolError::MiroApiError(e))
        }
    }
}

// ==================== Helper Functions ====================

/// Fetch boards from Miro API using Bearer token
async fn fetch_boards_from_miro(
    http_client: &Client,
    bearer_token: &str,
) -> Result<Vec<Board>, String> {
    const MIRO_API_URL: &str = "https://api.miro.com/v2/boards";

    let response = http_client
        .get(MIRO_API_URL)
        .bearer_auth(bearer_token)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let boards_response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Extract the "data" array from response
            let boards = boards_response
                .get("data")
                .and_then(|v| v.as_array())
                .ok_or("Invalid response format: missing 'data' array")?;

            boards
                .iter()
                .map(|board_json| {
                    serde_json::from_value::<Board>(board_json.clone())
                        .map_err(|e| format!("Failed to parse board: {}", e))
                })
                .collect()
        }
        reqwest::StatusCode::UNAUTHORIZED => {
            Err("Bearer token is invalid or expired (401)".to_string())
        }
        reqwest::StatusCode::FORBIDDEN => {
            Err("Access forbidden - insufficient permissions (403)".to_string())
        }
        status => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!(
                "Miro API error {}: {}",
                status.as_u16(),
                error_text
            ))
        }
    }
}

/// Fetch a specific board from Miro API using Bearer token
async fn fetch_board_from_miro(
    http_client: &Client,
    bearer_token: &str,
    board_id: &str,
) -> Result<Board, String> {
    let url = format!("https://api.miro.com/v2/boards/{}", board_id);

    let response = http_client
        .get(&url)
        .bearer_auth(bearer_token)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    match response.status() {
        reqwest::StatusCode::OK => response
            .json::<Board>()
            .await
            .map_err(|e| format!("Failed to parse board response: {}", e)),
        reqwest::StatusCode::UNAUTHORIZED => {
            Err("Bearer token is invalid or expired (401)".to_string())
        }
        reqwest::StatusCode::FORBIDDEN => {
            Err("Access forbidden - insufficient permissions (403)".to_string())
        }
        reqwest::StatusCode::NOT_FOUND => Err(format!("Board not found: {}", board_id)),
        status => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(format!(
                "Miro API error {}: {}",
                status.as_u16(),
                error_text
            ))
        }
    }
}

// ==================== Error Handling ====================

/// Tool error types
#[derive(Debug)]
pub enum ToolError {
    Unauthorized,
    InvalidInput(String),
    MiroApiError(String),
    InternalError(String),
}

impl IntoResponse for ToolError {
    fn into_response(self) -> Response {
        match self {
            ToolError::Unauthorized => {
                error!("Tool access unauthorized");
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ToolResponse::<()>::err(
                        "Unauthorized - valid Bearer token required".to_string(),
                    )),
                )
                    .into_response()
            }
            ToolError::InvalidInput(msg) => {
                warn!("Invalid tool input: {}", msg);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ToolResponse::<()>::err(format!("Invalid input: {}", msg))),
                )
                    .into_response()
            }
            ToolError::MiroApiError(msg) => {
                error!("Miro API error: {}", msg);
                (
                    StatusCode::BAD_GATEWAY,
                    Json(ToolResponse::<()>::err(format!("Miro API error: {}", msg))),
                )
                    .into_response()
            }
            ToolError::InternalError(msg) => {
                error!("Internal tool error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ToolResponse::<()>::err(format!("Internal error: {}", msg))),
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_info_from_board() {
        let board = Board {
            id: "board-123".to_string(),
            name: "Test Board".to_string(),
            description: Some("A test board".to_string()),
            created_at: "2025-01-01T00:00:00Z".to_string(),
        };

        let board_info = BoardInfo::from(board);
        assert_eq!(board_info.id, "board-123");
        assert_eq!(board_info.name, "Test Board");
        assert_eq!(board_info.description, Some("A test board".to_string()));
    }

    #[test]
    fn test_tool_response_ok() {
        let response = ToolResponse::ok(42);
        assert!(response.success);
        assert_eq!(response.data, Some(42));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_tool_response_err() {
        let response: ToolResponse<()> = ToolResponse::err("Something went wrong".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Something went wrong".to_string()));
    }
}
