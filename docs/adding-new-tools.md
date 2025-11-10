# Adding New Tools to the MCP Server

This guide explains how to add new MCP tools to the server using the dynamic tool routing system.

## Overview

The server uses the `rmcp` framework's `tool_router!` macro for **declarative tool registration**. This means:

- ✅ No manual registration required
- ✅ No hardcoded match statements to modify
- ✅ Tools are automatically discovered and routed
- ✅ Type-safe parameter handling via the `#[tool]` attribute

## Architecture

**Tool Routing Flow**:
```
MCP Client → call_tool() → ToolRouter::call() → #[tool] method
```

**Key Components**:
- `#[tool_router]` macro on the impl block (line 135) - Generates router
- `#[tool]` attribute on methods - Registers individual tools
- `ToolRouter::call()` (line 406) - Dynamic dispatch to correct handler

**Open/Closed Principle**: Adding new tools does NOT require modifying the routing logic.

## Step-by-Step Guide

### 1. Define Parameter Struct (if needed)

```rust
/// Parameters for the new tool
#[derive(Debug, Serialize, Deserialize)]
pub struct MyNewToolParams {
    pub board_id: String,
    pub some_field: String,
    #[serde(default)]
    pub optional_field: Option<String>,
}
```

**Location**: Add to `src/mcp/server.rs` (with other param structs, lines 11-125)

### 2. Add Tool Method

```rust
#[tool_router]
impl MiroMcpServer {
    // ... existing tools ...

    /// Brief description of what the tool does
    #[tool(description = "Detailed description for MCP clients to understand usage")]
    async fn my_new_tool(&self) -> Result<CallToolResult, McpError> {
        let message = "my_new_tool registered. Use tool_call with parameters: { ... }".to_string();
        Ok(CallToolResult::success(vec![Content::text(message)]))
    }
}
```

**Location**: Add inside the `#[tool_router] impl MiroMcpServer` block (after line 352)

**Important**:
- The method signature must be `async fn(&self) -> Result<CallToolResult, McpError>`
- The `#[tool]` attribute makes it discoverable by the router
- The function name becomes the MCP tool name (snake_case)

### 3. Add Implementation Method (if parameters needed)

If your tool requires parameters, create an internal implementation method:

```rust
impl MiroMcpServer {
    /// Internal implementation with parameter support
    async fn my_new_tool_with_params(
        &self,
        params: MyNewToolParams,
    ) -> Result<CallToolResult, McpError> {
        // Actual implementation here
        let result = self.miro_client
            .some_operation(&params.board_id, &params.some_field)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            format!("Operation successful: {:?}", result)
        )]))
    }
}
```

**Location**: Add outside the `#[tool_router]` block (after line 353)

### 4. Add Parameter Routing (if needed)

If your tool uses the `_with_params` pattern, add routing in `call_tool`:

```rust
async fn call_tool(&self, params: CallToolRequestParam, ctx: RequestContext<RoleServer>)
    -> Result<CallToolResult, McpError> {
    // Add special handling for parameterized tools
    if params.name.as_ref() == "my_new_tool" {
        let args_value = serde_json::Value::Object(params.arguments.clone().unwrap_or_default());
        let tool_params: MyNewToolParams = serde_json::from_value(args_value)
            .map_err(|e| McpError::internal_error(format!("Invalid parameters: {}", e), None))?;
        return self.my_new_tool_with_params(tool_params).await;
    }

    // Existing list_items special case
    // ...

    // Default router for simple tools
    let tool_ctx = ToolCallContext::new(self, params, ctx);
    self.tool_router.call(tool_ctx).await
}
```

**Note**: Only needed if you're using the parameter struct pattern like `list_items` does.

### 5. Add Tests

Add unit and integration tests for the new tool:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_new_tool_params_serialization() {
        let params = MyNewToolParams {
            board_id: "board123".to_string(),
            some_field: "value".to_string(),
            optional_field: None,
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("board123"));
    }
}
```

**Integration test** (in `tests/` directory):

```rust
#[tokio::test]
async fn test_my_new_tool_operation() {
    // Test actual Miro API interaction
}
```

## Examples

### Example 1: Simple Tool (No Parameters)

```rust
/// Get server health status
#[tool(description = "Check MCP server health and Miro API connectivity")]
async fn health_check(&self) -> Result<CallToolResult, McpError> {
    let message = "Server is healthy".to_string();
    Ok(CallToolResult::success(vec![Content::text(message)]))
}
```

**That's it!** The tool is automatically registered and routed.

### Example 2: Tool with Parameters

```rust
// 1. Define params
#[derive(Debug, Serialize, Deserialize)]
pub struct GetBoardParams {
    pub board_id: String,
}

// 2. Add tool declaration
#[tool(description = "Get detailed information about a specific board")]
async fn get_board(&self) -> Result<CallToolResult, McpError> {
    let message = "get_board tool registered. Use with { board_id: ... }".to_string();
    Ok(CallToolResult::success(vec![Content::text(message)]))
}

// 3. Add implementation
async fn get_board_with_params(
    &self,
    params: GetBoardParams,
) -> Result<CallToolResult, McpError> {
    let board = self.miro_client
        .get_board(&params.board_id)
        .await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    let json = serde_json::to_string(&board)
        .unwrap_or_else(|_| "Failed to serialize board".to_string());
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// 4. Add routing in call_tool
if params.name.as_ref() == "get_board" {
    let args_value = serde_json::Value::Object(params.arguments.clone().unwrap_or_default());
    let board_params: GetBoardParams = serde_json::from_value(args_value)
        .map_err(|e| McpError::internal_error(format!("Invalid parameters: {}", e), None))?;
    return self.get_board_with_params(board_params).await;
}
```

## Verification Checklist

After adding a new tool, verify:

- [ ] `cargo clippy -- -D warnings` passes (zero warnings)
- [ ] `cargo test` passes (all existing tests still work)
- [ ] `cargo fmt --check` passes (formatting clean)
- [ ] Tool appears in `list_tools` output
- [ ] Tool description is clear and helpful
- [ ] Parameters are documented with JSON schema (if applicable)
- [ ] Error handling is comprehensive
- [ ] Integration tests added

## Common Patterns

### Pattern 1: CRUD Operations
```rust
create_X → POST /boards/{id}/X
get_X    → GET /boards/{id}/X/{item_id}
update_X → PATCH /boards/{id}/X/{item_id}
delete_X → DELETE /boards/{id}/X/{item_id}
list_X   → GET /boards/{id}/X
```

### Pattern 2: Bulk Operations
```rust
bulk_create_X → POST /boards/{id}/X/bulk (max 20 items)
bulk_update_X → PATCH /boards/{id}/X/bulk
bulk_delete_X → DELETE /boards/{id}/X/bulk
```

### Pattern 3: Nested Resources
```rust
get_board_members → GET /boards/{id}/members
add_board_member  → POST /boards/{id}/members
```

## Best Practices

1. **Tool Naming**: Use `snake_case` (becomes MCP tool name)
2. **Descriptions**: Be specific about what the tool does and when to use it
3. **Error Messages**: Include actionable information for debugging
4. **Parameters**: Use `#[serde(default)]` for optional fields
5. **Validation**: Validate parameters before making API calls
6. **Testing**: Add both unit and integration tests
7. **Documentation**: Update this guide with new patterns you discover

## Troubleshooting

**Tool not appearing in `list_tools`**:
- Verify `#[tool]` attribute is present
- Check the method is inside the `#[tool_router] impl MiroMcpServer` block
- Rebuild with `cargo clean && cargo build`

**"Tool not found" error**:
- If using parameters, ensure you added routing in `call_tool`
- Verify tool name matches exactly (case-sensitive)

**Parameter parsing errors**:
- Check struct fields match MCP client's arguments
- Use `#[serde(default)]` for optional fields
- Add debug logging: `dbg!(&params.arguments)`

## Architecture Decision

**Why use `tool_router!` macro instead of manual registration?**

The `rmcp` framework provides two approaches:
1. **Manual registration**: Explicit `router.add_route()` calls
2. **Declarative macro**: `#[tool_router]` + `#[tool]` attributes

We chose **declarative** because:
- **Open/Closed Principle**: Adding tools doesn't modify routing code
- **Type Safety**: Compile-time verification of tool signatures
- **Less Boilerplate**: No manual route registration needed
- **Framework-Native**: Follows rmcp best practices

**Trade-off**: Slight learning curve for the macro syntax, but significant long-term maintainability gains.

## Related Files

- `src/mcp/server.rs` - Main server implementation
- `src/miro/client.rs` - Miro API client
- `src/miro/types.rs` - Miro API type definitions
- `tests/` - Integration tests

## Further Reading

- [rmcp Documentation](https://docs.rs/rmcp/)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [Miro REST API Reference](https://developers.miro.com/reference)
