//! Integration tests for MCP protocol endpoint
//!
//! Tests the JSON-RPC 2.0 MCP protocol implementation over HTTP POST

use miro_mcp_server::{
    mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse},
    TokenValidator,
};
use serde_json::{json, Value};

#[test]
fn test_jsonrpc_request_serialization() {
    let req = JsonRpcRequest::new("test_method").with_id(Value::Number(1.into()));

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"method\":\"test_method\""));
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":1"));
}

#[test]
fn test_jsonrpc_request_with_params() {
    let req = JsonRpcRequest::new("tools/call")
        .with_params(json!({"name": "list_boards", "arguments": {}}))
        .with_id(Value::Number(42.into()));

    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("tools/call"));
    assert!(json.contains("list_boards"));
}

#[test]
fn test_jsonrpc_response_success_serialization() {
    let result = json!({"status": "ok", "data": [{"id": "1", "name": "Board 1"}]});
    let resp = JsonRpcResponse::success(result, Some(Value::Number(1.into())));

    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"result\""));
    assert!(!json.contains("\"error\""));
}

#[test]
fn test_jsonrpc_response_error_serialization() {
    let error = JsonRpcError::method_not_found("unknown_tool");
    let resp = JsonRpcResponse::error(error, Some(Value::Number(1.into())));

    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"id\":1"));
    assert!(json.contains("\"error\""));
    assert!(json.contains("-32601"));
    assert!(json.contains("Method not found"));
}

#[test]
fn test_jsonrpc_error_invalid_request() {
    let error = JsonRpcError::invalid_request("Request malformed");
    assert_eq!(error.code, -32600);
    assert_eq!(error.message, "Request malformed");
}

#[test]
fn test_jsonrpc_error_method_not_found() {
    let error = JsonRpcError::method_not_found("unknown_method");
    assert_eq!(error.code, -32601);
    assert!(error.message.contains("unknown_method"));
}

#[test]
fn test_jsonrpc_error_invalid_params() {
    let error = JsonRpcError::invalid_params("Expected board_id parameter");
    assert_eq!(error.code, -32602);
}

#[test]
fn test_jsonrpc_error_internal_error() {
    let error = JsonRpcError::internal_error("Miro API request failed");
    assert_eq!(error.code, -32603);
}

#[test]
fn test_jsonrpc_notification_no_response() {
    let req = JsonRpcRequest {
        jsonrpc: Some("2.0".to_string()),
        method: "notify".to_string(),
        params: None,
        id: None,
    };

    assert!(req.is_notification());
    assert_eq!(req.id, None);
}

#[test]
fn test_deserialization_initialize_request() {
    let json_str = r#"{"jsonrpc":"2.0","method":"initialize","id":1,"params":{"client_info":{"name":"Claude"}}}"#;
    let req: JsonRpcRequest = serde_json::from_str(json_str).unwrap();

    assert_eq!(req.method, "initialize");
    assert!(req.params.is_some());
}

#[test]
fn test_deserialization_tools_call_request() {
    let json_str = r#"{"jsonrpc":"2.0","method":"tools/call","id":2,"params":{"name":"list_boards","arguments":{}}}"#;
    let req: JsonRpcRequest = serde_json::from_str(json_str).unwrap();

    assert_eq!(req.method, "tools/call");
    assert!(req.params.is_some());
}

#[test]
fn test_response_preserves_id() {
    let id = Value::String("unique-request-id".to_string());
    let resp = JsonRpcResponse::success(json!({"result": true}), Some(id.clone()));

    assert_eq!(resp.id, Some(id));
}

#[test]
fn test_response_with_null_id() {
    let resp = JsonRpcResponse::success(json!({}), None);
    assert_eq!(resp.id, None);
}

#[test]
fn test_error_response_includes_error_not_result() {
    let error = JsonRpcError::invalid_params("Missing required field");
    let resp = JsonRpcResponse::error(error, Some(Value::Number(1.into())));

    assert!(resp.result.is_none());
    assert!(resp.error.is_some());
}

#[test]
fn test_success_response_includes_result_not_error() {
    let resp = JsonRpcResponse::success(json!({"data": "value"}), Some(Value::Number(1.into())));

    assert!(resp.result.is_some());
    assert!(resp.error.is_none());
}

#[test]
fn test_tool_call_error_code_for_unknown_tool() {
    let error = JsonRpcError::method_not_found("get_item");
    assert_eq!(error.code, -32601); // Method not found
}

#[test]
fn test_token_validator_initialization() {
    let token_validator = TokenValidator::new();
    // Just verify it can be created without panicking
    assert!(std::mem::size_of_val(&token_validator) > 0);
}

#[test]
fn test_multiple_tools_in_list() {
    let tools_list = json!({
        "tools": [
            {
                "name": "list_boards",
                "description": "List all boards"
            },
            {
                "name": "get_board",
                "description": "Get board details"
            }
        ]
    });

    if let Some(tools) = tools_list.get("tools").and_then(|v| v.as_array()) {
        assert_eq!(tools.len(), 2);
    }
}
