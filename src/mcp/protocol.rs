//! JSON-RPC 2.0 Protocol Implementation for MCP
//!
//! Implements the JSON-RPC 2.0 specification for MCP (Model Context Protocol)
//! communication over HTTP POST.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
/// Spec: https://www.jsonrpc.org/specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<String>, // "2.0"
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>, // String, Number, or null (null = notification)
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: Some("2.0".to_string()),
            method: method.into(),
            params: None,
            id: Some(Value::Number(1.into())),
        }
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.params = Some(params);
        self
    }

    pub fn with_id(mut self, id: Value) -> Self {
        self.id = Some(id);
        self
    }

    /// Check if this is a valid JSON-RPC 2.0 request
    pub fn validate(&self) -> Result<(), String> {
        if self.jsonrpc.as_deref() != Some("2.0") && self.jsonrpc.is_some() {
            return Err("Invalid jsonrpc version, must be '2.0'".to_string());
        }

        if self.method.is_empty() {
            return Err("method field is required".to_string());
        }

        Ok(())
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 Success Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String, // "2.0"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
}

impl JsonRpcResponse {
    /// Create success response
    pub fn success(result: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create error response
    pub fn error(error: JsonRpcError, id: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Invalid Request (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// Method not found (-32601)
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    /// Invalid params (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// Internal error (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }

    /// Server error (-32000 to -32099)
    pub fn server_error(code: i32, message: impl Into<String>) -> Self {
        if !(-32099..=-32000).contains(&code) {
            return Self {
                code: -32000,
                message: message.into(),
                data: None,
            };
        }
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

// ===================== MCP Protocol Messages =====================

/// MCP Initialize Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>, // "2024-11-05" or similar
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<ClientInfo>,
}

/// Client Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// MCP Server Capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Initialize Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Server Info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

// ===================== Tools List/Call Messages =====================

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>, // JSON Schema
}

/// Tools List Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

/// Tool Call Request Params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// Tool Call Result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolCallResult {
    Success {
        content: Vec<TextContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    Error {
        #[serde(rename = "error")]
        error_msg: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String, // "text"
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonrpc_request_validation() {
        let mut req = JsonRpcRequest::new("test_method");
        assert!(req.validate().is_ok());

        req.jsonrpc = Some("1.0".to_string());
        assert!(req.validate().is_err());

        req.method = String::new();
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let resp = JsonRpcResponse::success(json!({"data": "test"}), Some(Value::Number(1.into())));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        assert_eq!(resp.jsonrpc, "2.0");
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let error = JsonRpcError::method_not_found("unknown_method");
        let resp = JsonRpcResponse::error(error, Some(Value::Number(1.into())));
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_jsonrpc_error_codes() {
        assert_eq!(JsonRpcError::invalid_request("msg").code, -32600);
        assert_eq!(JsonRpcError::method_not_found("test").code, -32601);
        assert_eq!(JsonRpcError::invalid_params("msg").code, -32602);
        assert_eq!(JsonRpcError::internal_error("msg").code, -32603);
    }

    #[test]
    fn test_jsonrpc_notification() {
        let req = JsonRpcRequest {
            jsonrpc: Some("2.0".to_string()),
            method: "notify".to_string(),
            params: None,
            id: None,
        };
        assert!(req.is_notification());
    }

    #[test]
    fn test_tool_definition_serialization() {
        let tool = Tool {
            name: "list_boards".to_string(),
            description: "List all boards".to_string(),
            input_schema: None,
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("list_boards"));
    }
}
