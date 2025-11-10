use crate::auth::{extract_bearer_token, TokenValidator};
use crate::mcp::{oauth_metadata, JsonRpcRequest, JsonRpcResponse, JsonRpcError};
use crate::mcp::{handle_initialize, handle_tools_list, handle_tools_call};
use crate::auth::token_validator::UserInfo;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router, Json,
};
use std::sync::Arc;
use tracing::{info, warn, error};

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// MCP Protocol endpoint for JSON-RPC 2.0 requests
///
/// Handles MCP methods:
/// - initialize: Handshake and capability negotiation
/// - tools/list: List available tools
/// - tools/call: Execute a tool
///
/// Requires Bearer token authentication (provided by middleware).
/// Token and user info are extracted from request extensions.
async fn mcp_endpoint(
    axum::Extension(token): axum::Extension<Arc<String>>,
    axum::Extension(user_info): axum::Extension<Arc<UserInfo>>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Validate JSON-RPC request format
    if let Err(e) = req.validate() {
        error!("Invalid JSON-RPC request: {}", e);
        return (
            StatusCode::BAD_REQUEST,
            Json(JsonRpcResponse::error(
                JsonRpcError::invalid_request(e),
                req.id.clone(),
            )),
        );
    }

    info!(
        method = %req.method,
        user_id = %user_info.user_id,
        "Processing MCP request"
    );

    // Route to appropriate handler
    let response = match req.method.as_str() {
        "initialize" => {
            info!("Handling initialize request");
            handle_initialize(&req, &user_info)
        }
        "tools/list" => {
            info!("Handling tools/list request");
            handle_tools_list(&req, &user_info)
        }
        "tools/call" => {
            info!("Handling tools/call request");
            handle_tools_call(&req, &user_info, &token).await
        }
        method => {
            warn!(method = %method, "Unknown MCP method");
            JsonRpcResponse::error(
                JsonRpcError::method_not_found(method),
                req.id.clone(),
            )
        }
    };

    (StatusCode::OK, Json(response))
}

//
// ============================================================================
// ADR-002 Resource Server Implementation (OAuth client removed)
// ============================================================================
//

/// Simplified application state for ADR-002 Resource Server
/// No OAuth client, no cookie managers - only token validation
#[derive(Clone)]
pub struct AppStateADR002 {
    token_validator: Arc<TokenValidator>,
}

/// Bearer token validation middleware for ADR-002
/// Simplified version without OAuth client dependencies
async fn bearer_auth_middleware_adr002(
    State(state): State<AppStateADR002>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Bearer token from Authorization header
    let token = match extract_bearer_token(request.headers()) {
        Ok(token) => token,
        Err(e) => {
            warn!("Bearer token extraction failed: {}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate token with Miro API (with caching)
    let user_info = match state.token_validator.validate_token(&token).await {
        Ok(user_info) => user_info,
        Err(e) => {
            warn!("Token validation failed: {}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    info!("Request authenticated for user: {}", user_info.user_id);

    // Store both token and user_info in request extensions for handlers
    request.extensions_mut().insert(Arc::new(token));
    request.extensions_mut().insert(Arc::new(user_info));

    Ok(next.run(request).await)
}

/// Create HTTP server for ADR-002 Resource Server pattern
/// Only includes:
/// - OAuth metadata endpoint (AUTH6)
/// - Bearer token authentication (AUTH7+AUTH8+AUTH9)
/// - MCP tools (list_boards, get_board)
pub fn create_app_adr002(token_validator: Arc<TokenValidator>) -> Router {
    let state = AppStateADR002 { token_validator };

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/.well-known/oauth-protected-resource", get(oauth_metadata));

    // Protected routes (Bearer token required)
    let protected_routes = Router::new()
        .route("/mcp", axum::routing::post(mcp_endpoint))
        .route("/mcp/list_boards", axum::routing::post(crate::mcp::tools::list_boards))
        .route("/mcp/get_board/:board_id", axum::routing::post(crate::mcp::tools::get_board))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            bearer_auth_middleware_adr002,
        ));

    // Merge routes
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
}

/// Run HTTP server with ADR-002 Resource Server pattern
/// No OAuth client code - only Bearer token validation
pub async fn run_server_adr002(
    port: u16,
    token_validator: Arc<TokenValidator>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app_adr002(token_validator);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("ADR-002 Resource Server listening on {}", addr);
    info!("OAuth metadata endpoint: http://{}/.well-known/oauth-protected-resource", addr);
    info!("Protected endpoints require Bearer token validation");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down HTTP server");
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_app_adr002() {
        let token_validator = Arc::new(TokenValidator::new());
        let app = create_app_adr002(token_validator);
        assert!(std::mem::size_of_val(&app) > 0);
    }
}
