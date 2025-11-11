//! MCP Protocol Method Handlers for JSON-RPC 2.0
//!
//! Implements handlers for MCP methods:
//! - initialize: Handshake and capability negotiation
//! - tools/list: List available tools
//! - tools/call: Execute a tool

use super::protocol::*;
use crate::auth::token_validator::UserInfo;
use crate::mcp::tools::{BoardInfo, GetBoardResponse, ListBoardsResponse};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Handle the initialize method
///
/// Returns server capabilities and protocol version
pub fn handle_initialize(req: &JsonRpcRequest, _user_info: &Arc<UserInfo>) -> JsonRpcResponse {
    info!("Handling initialize request");

    let server_capabilities = ServerCapabilities {
        tools: Some(ToolsCapability {
            list_changed: Some(false),
        }),
        resources: None,
        prompts: None,
    };

    let result = InitializeResult {
        protocol_version: Some("2024-11-05".to_string()),
        capabilities: server_capabilities,
        server_info: ServerInfo {
            name: "Miro MCP Server".to_string(),
            version: Some("0.1.0".to_string()),
        },
    };

    JsonRpcResponse::success(
        serde_json::to_value(result).unwrap_or_else(|_| json!({})),
        req.id.clone(),
    )
}

/// Handle the tools/list method
///
/// Returns list of available tools (list_boards, get_board)
pub fn handle_tools_list(req: &JsonRpcRequest, _user_info: &Arc<UserInfo>) -> JsonRpcResponse {
    info!("Handling tools/list request");

    let tools = vec![
        Tool {
            name: "list_boards".to_string(),
            description: "List all Miro boards accessible to the authenticated user".to_string(),
            input_schema: Some(json!({
                "type": "object",
                "properties": {},
                "required": []
            })),
        },
        Tool {
            name: "get_board".to_string(),
            description: "Get details of a specific Miro board by ID".to_string(),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "board_id": {
                        "type": "string",
                        "description": "The ID of the board to retrieve"
                    }
                },
                "required": ["board_id"]
            })),
        },
    ];

    let result = ToolsListResult { tools };

    JsonRpcResponse::success(
        serde_json::to_value(result).unwrap_or_else(|_| json!({})),
        req.id.clone(),
    )
}

/// Handle the tools/call method
///
/// Executes a tool (list_boards or get_board)
///
/// # Arguments
///
/// * `req` - JSON-RPC request containing tool name and arguments
/// * `user_info` - User info with Bearer token for API calls
/// * `token` - Bearer token for Miro API authentication
///
/// # Returns
///
/// JSON-RPC response with tool result or error
pub async fn handle_tools_call(
    req: &JsonRpcRequest,
    user_info: &Arc<UserInfo>,
    token: &Arc<String>,
) -> JsonRpcResponse {
    // Parse tool call parameters
    let params = match req.params.as_ref() {
        Some(params) => params,
        None => {
            warn!("tools/call missing params field");
            return JsonRpcResponse::error(
                JsonRpcError::invalid_params("params field is required for tools/call"),
                req.id.clone(),
            );
        }
    };

    let tool_call_params = match serde_json::from_value::<ToolCallParams>(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            warn!("Failed to parse tool call params: {}", e);
            return JsonRpcResponse::error(
                JsonRpcError::invalid_params(format!("Invalid tool call params: {}", e)),
                req.id.clone(),
            );
        }
    };

    let tool_name = &tool_call_params.name;
    info!(
        tool_name = %tool_name,
        user_id = %user_info.user_id,
        "Executing tool"
    );

    match tool_name.as_str() {
        "list_boards" => handle_list_boards_call(req, user_info, token).await,
        "get_board" => handle_get_board_call(req, user_info, token, &tool_call_params).await,
        _ => {
            warn!(tool_name = %tool_name, "Unknown tool requested");
            JsonRpcResponse::error(
                JsonRpcError::method_not_found(tool_name.clone()),
                req.id.clone(),
            )
        }
    }
}

/// Handle list_boards tool call
async fn handle_list_boards_call(
    req: &JsonRpcRequest,
    user_info: &Arc<UserInfo>,
    token: &Arc<String>,
) -> JsonRpcResponse {
    use reqwest::Client;

    let http_client = Client::new();
    const MIRO_API_URL: &str = "https://api.miro.com/v2/boards";

    match http_client
        .get(MIRO_API_URL)
        .bearer_auth(token.as_str())
        .send()
        .await
    {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => match response.json::<serde_json::Value>().await {
                Ok(boards_response) => {
                    match boards_response.get("data").and_then(|v| v.as_array()) {
                        Some(boards) => {
                            let board_infos: Vec<BoardInfo> = boards
                                .iter()
                                .filter_map(|board_json| {
                                    serde_json::from_value::<crate::miro::types::Board>(
                                        board_json.clone(),
                                    )
                                    .ok()
                                    .map(BoardInfo::from)
                                })
                                .collect();

                            let count = board_infos.len();
                            let list_boards_result = ListBoardsResponse {
                                boards: board_infos,
                                count,
                            };

                            info!(
                                user_id = %user_info.user_id,
                                count = count,
                                "Successfully listed boards via MCP"
                            );

                            let result = ToolCallResult::Success {
                                content: vec![TextContent {
                                    content_type: "text".to_string(),
                                    text: serde_json::to_string(&list_boards_result)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                }],
                                is_error: Some(false),
                            };

                            JsonRpcResponse::success(
                                serde_json::to_value(result).unwrap_or_else(|_| json!({})),
                                req.id.clone(),
                            )
                        }
                        None => {
                            error!("Miro API response missing data array");
                            JsonRpcResponse::error(
                                JsonRpcError::internal_error("Invalid Miro API response format"),
                                req.id.clone(),
                            )
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to parse Miro API response");
                    JsonRpcResponse::error(
                        JsonRpcError::internal_error(format!(
                            "Failed to parse API response: {}",
                            e
                        )),
                        req.id.clone(),
                    )
                }
            },
            reqwest::StatusCode::UNAUTHORIZED => {
                warn!("Bearer token invalid or expired");
                JsonRpcResponse::error(
                    JsonRpcError::server_error(-32001, "Bearer token invalid or expired (401)"),
                    req.id.clone(),
                )
            }
            status => {
                error!(status = ?status, "Miro API returned error");
                JsonRpcResponse::error(
                    JsonRpcError::server_error(
                        -32001,
                        format!("Miro API error: {}", status.as_u16()),
                    ),
                    req.id.clone(),
                )
            }
        },
        Err(e) => {
            error!(error = %e, "HTTP request to Miro API failed");
            JsonRpcResponse::error(
                JsonRpcError::internal_error(format!("HTTP request failed: {}", e)),
                req.id.clone(),
            )
        }
    }
}

/// Handle get_board tool call
async fn handle_get_board_call(
    req: &JsonRpcRequest,
    user_info: &Arc<UserInfo>,
    token: &Arc<String>,
    tool_params: &ToolCallParams,
) -> JsonRpcResponse {
    use reqwest::Client;

    let board_id = match tool_params
        .arguments
        .as_ref()
        .and_then(|a| a.get("board_id"))
    {
        Some(Value::String(id)) => id.clone(),
        _ => {
            warn!("get_board missing board_id argument");
            return JsonRpcResponse::error(
                JsonRpcError::invalid_params("board_id is required for get_board tool"),
                req.id.clone(),
            );
        }
    };

    if board_id.is_empty() {
        warn!("Empty board_id provided");
        return JsonRpcResponse::error(
            JsonRpcError::invalid_params("board_id cannot be empty"),
            req.id.clone(),
        );
    }

    let http_client = Client::new();
    let url = format!("https://api.miro.com/v2/boards/{}", board_id);

    match http_client
        .get(&url)
        .bearer_auth(token.as_str())
        .send()
        .await
    {
        Ok(response) => match response.status() {
            reqwest::StatusCode::OK => match response.json::<crate::miro::types::Board>().await {
                Ok(board) => {
                    info!(
                        user_id = %user_info.user_id,
                        board_id = %board_id,
                        board_name = %board.name,
                        "Successfully retrieved board via MCP"
                    );

                    let get_board_result = GetBoardResponse {
                        board: BoardInfo::from(board),
                    };

                    let result = ToolCallResult::Success {
                        content: vec![TextContent {
                            content_type: "text".to_string(),
                            text: serde_json::to_string(&get_board_result)
                                .unwrap_or_else(|_| "{}".to_string()),
                        }],
                        is_error: Some(false),
                    };

                    JsonRpcResponse::success(
                        serde_json::to_value(result).unwrap_or_else(|_| json!({})),
                        req.id.clone(),
                    )
                }
                Err(e) => {
                    error!(error = %e, "Failed to parse board response");
                    JsonRpcResponse::error(
                        JsonRpcError::internal_error(format!("Failed to parse board: {}", e)),
                        req.id.clone(),
                    )
                }
            },
            reqwest::StatusCode::NOT_FOUND => {
                warn!(board_id = %board_id, "Board not found");
                JsonRpcResponse::error(
                    JsonRpcError::server_error(-32002, format!("Board not found: {}", board_id)),
                    req.id.clone(),
                )
            }
            reqwest::StatusCode::UNAUTHORIZED => {
                warn!("Bearer token invalid or expired");
                JsonRpcResponse::error(
                    JsonRpcError::server_error(-32001, "Bearer token invalid or expired (401)"),
                    req.id.clone(),
                )
            }
            status => {
                error!(status = ?status, "Miro API returned error");
                JsonRpcResponse::error(
                    JsonRpcError::server_error(
                        -32001,
                        format!("Miro API error: {}", status.as_u16()),
                    ),
                    req.id.clone(),
                )
            }
        },
        Err(e) => {
            error!(error = %e, "HTTP request to Miro API failed");
            JsonRpcResponse::error(
                JsonRpcError::internal_error(format!("HTTP request failed: {}", e)),
                req.id.clone(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_initialize() {
        let req = JsonRpcRequest::new("initialize").with_id(Value::Number(1.into()));
        let user_info = Arc::new(UserInfo::new(
            "test-user".to_string(),
            "test-team".to_string(),
            vec![],
        ));

        let response = handle_initialize(&req, &user_info);

        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert_eq!(response.id, Some(Value::Number(1.into())));
    }

    #[test]
    fn test_handle_tools_list() {
        let req = JsonRpcRequest::new("tools/list").with_id(Value::Number(1.into()));
        let user_info = Arc::new(UserInfo::new(
            "test-user".to_string(),
            "test-team".to_string(),
            vec![],
        ));

        let response = handle_tools_list(&req, &user_info);

        assert!(response.result.is_some());
        assert!(response.error.is_none());

        if let Some(Value::Object(result)) = response.result {
            assert!(result.contains_key("tools"));
        }
    }

    #[test]
    fn test_handle_tools_call_missing_params() {
        let req = JsonRpcRequest::new("tools/call").with_id(Value::Number(1.into()));
        let user_info = Arc::new(UserInfo::new(
            "test-user".to_string(),
            "test-team".to_string(),
            vec![],
        ));
        let token = Arc::new("test-token".to_string());

        // Use block_on to run async function in sync test
        let response = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { handle_tools_call(&req, &user_info, &token).await });

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32602);
    }

    #[test]
    fn test_handle_tools_call_unknown_tool() {
        let req = JsonRpcRequest::new("tools/call")
            .with_id(Value::Number(1.into()))
            .with_params(json!({
                "name": "unknown_tool",
                "arguments": {}
            }));
        let user_info = Arc::new(UserInfo::new(
            "test-user".to_string(),
            "test-team".to_string(),
            vec![],
        ));
        let token = Arc::new("test-token".to_string());

        let response = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { handle_tools_call(&req, &user_info, &token).await });

        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, -32601);
    }
}
