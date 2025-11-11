use crate::auth::token_validator::UserInfo;
use crate::auth::{extract_bearer_token, TokenValidator};
use crate::config::Config;
use crate::mcp::{handle_initialize, handle_tools_call, handle_tools_list};
use crate::mcp::{
    oauth_authorization_server_metadata, oauth_metadata, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse,
};
use axum::{
    extract::State,
    http::{HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};
use uuid::Uuid;

#[cfg(feature = "oauth-proxy")]
use crate::oauth::{
    authorize_handler, callback_handler, cookie_manager::CookieManager, dcr::ClientRegistry,
    proxy_provider::MiroOAuthProvider, register_handler, token_handler,
};

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
            JsonRpcResponse::error(JsonRpcError::method_not_found(method), req.id.clone())
        }
    };

    (StatusCode::OK, Json(response))
}

//
// ============================================================================
// ADR-002 Resource Server Implementation (OAuth client removed)
// ============================================================================
//

/// Correlation ID for request tracing
#[derive(Clone)]
pub struct RequestId(pub String);

/// Correlation ID middleware - adds unique request_id to all requests
/// This enables tracing requests across the entire lifecycle for debugging
async fn correlation_id_middleware(mut request: Request<axum::body::Body>, next: Next) -> Response {
    // Generate unique request ID
    let request_id = Uuid::new_v4().to_string();

    // Create tracing span with request_id for all subsequent logs
    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
    );

    // Store request_id in extensions for access in handlers
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    // Execute request within the span
    let _enter = span.enter();

    info!("Request started");
    let response = next.run(request).await;
    info!("Request completed");

    response
}

/// Application state for ADR-002 Resource Server with ADR-004 Proxy OAuth + DCR
/// Includes token validation + OAuth proxy components + Dynamic Client Registration
#[derive(Clone)]
pub struct AppStateADR002 {
    pub token_validator: Arc<TokenValidator>,
    pub config: Arc<Config>,
    #[cfg(feature = "oauth-proxy")]
    pub oauth_provider: Arc<MiroOAuthProvider>,
    #[cfg(feature = "oauth-proxy")]
    pub cookie_manager: Arc<CookieManager>,
    #[cfg(feature = "oauth-proxy")]
    pub client_registry: ClientRegistry,
}

/// Bearer token validation middleware for ADR-002
/// Simplified version without OAuth client dependencies
async fn bearer_auth_middleware_adr002(
    State(state): State<AppStateADR002>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, Response> {
    // Extract request_id from extensions for structured logging
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|rid| rid.0.clone())
        .unwrap_or_else(|| "unknown".to_string());

    // Extract Bearer token from Authorization header
    let token = match extract_bearer_token(request.headers()) {
        Ok(token) => token,
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                auth_stage = "token_extraction",
                "Bearer token extraction failed"
            );
            // Return 401 with WWW-Authenticate header per RFC 6750
            return Ok((
                StatusCode::UNAUTHORIZED,
                [(
                    axum::http::header::WWW_AUTHENTICATE,
                    "Bearer realm=\"miro-mcp-server\"",
                )],
            )
                .into_response());
        }
    };

    // Validate token with Miro API (with caching)
    let user_info = match state.token_validator.validate_token(&token).await {
        Ok(user_info) => user_info,
        Err(e) => {
            warn!(
                request_id = %request_id,
                error = %e,
                auth_stage = "token_validation",
                "Token validation failed"
            );
            // Return 401 with WWW-Authenticate header per RFC 6750
            return Ok((
                StatusCode::UNAUTHORIZED,
                [(
                    axum::http::header::WWW_AUTHENTICATE,
                    "Bearer realm=\"miro-mcp-server\", error=\"invalid_token\"",
                )],
            )
                .into_response());
        }
    };

    info!(
        request_id = %request_id,
        user_id = %user_info.user_id,
        team_id = %user_info.team_id,
        scopes = ?user_info.scopes,
        "Request authenticated successfully"
    );

    // Store both token and user_info in request extensions for handlers
    request.extensions_mut().insert(Arc::new(token));
    request.extensions_mut().insert(Arc::new(user_info));

    Ok(next.run(request).await)
}

/// Create HTTP server for ADR-002 Resource Server with ADR-004 Proxy OAuth
/// Includes:
/// - Correlation ID middleware (OBS1)
/// - OAuth metadata endpoint (AUTH14 - updated for proxy pattern)
/// - OAuth proxy endpoints (AUTH11 - authorize, callback, token)
/// - Bearer token authentication (AUTH7+AUTH8+AUTH9)
/// - MCP tools (list_boards, get_board)
pub fn create_app_adr002(
    token_validator: Arc<TokenValidator>,
    config: Arc<Config>,
    #[cfg(feature = "oauth-proxy")] oauth_provider: Arc<MiroOAuthProvider>,
    #[cfg(feature = "oauth-proxy")] cookie_manager: Arc<CookieManager>,
) -> Router {
    #[cfg(feature = "oauth-proxy")]
    let state = AppStateADR002 {
        token_validator,
        config,
        oauth_provider,
        cookie_manager,
        client_registry: ClientRegistry::new(),
    };

    #[cfg(not(feature = "oauth-proxy"))]
    let state = AppStateADR002 {
        token_validator,
        config,
    };

    // Public routes (no authentication required)
    #[cfg(feature = "oauth-proxy")]
    let oauth_routes = Router::new()
        // Standard paths with /oauth prefix
        .route("/oauth/authorize", get(authorize_handler))
        .route("/oauth/callback", get(callback_handler))
        .route("/oauth/token", post(token_handler))
        // Alias paths without /oauth prefix (for Claude.ai compatibility)
        .route("/authorize", get(authorize_handler))
        .route("/callback", get(callback_handler))
        .route("/token", post(token_handler))
        .with_state(state.clone());

    // DCR endpoint needs separate router with ClientRegistry state
    #[cfg(feature = "oauth-proxy")]
    let dcr_routes = Router::new()
        .route("/register", post(register_handler))
        .with_state(state.client_registry.clone());

    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/.well-known/oauth-protected-resource", get(oauth_metadata))
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth_authorization_server_metadata),
        );

    #[cfg(feature = "oauth-proxy")]
    let public_routes = public_routes.merge(oauth_routes).merge(dcr_routes);

    // Protected routes (Bearer token required)
    let protected_routes = Router::new()
        .route("/mcp", axum::routing::post(mcp_endpoint))
        .route(
            "/mcp/list_boards",
            axum::routing::post(crate::mcp::tools::list_boards),
        )
        .route(
            "/mcp/get_board/:board_id",
            axum::routing::post(crate::mcp::tools::get_board),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            bearer_auth_middleware_adr002,
        ));

    // CORS layer for Claude.ai compatibility
    // Allow Claude.ai domain to access OAuth metadata and endpoints
    let cors = CorsLayer::new()
        .allow_origin("https://claude.ai".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::COOKIE,
        ])
        .allow_credentials(true);

    // Merge routes and apply middlewares to ALL requests
    Router::new()
        .merge(public_routes.with_state(state.config.clone()))
        .merge(protected_routes)
        .layer(cors)
        .layer(middleware::from_fn(correlation_id_middleware))
}

/// Run HTTP server with ADR-002 Resource Server + ADR-004 Proxy OAuth
/// Includes Bearer token validation + OAuth proxy when stdio-mcp feature enabled
pub async fn run_server_adr002(
    port: u16,
    token_validator: Arc<TokenValidator>,
    config: Arc<Config>,
    #[cfg(feature = "oauth-proxy")] oauth_provider: Arc<MiroOAuthProvider>,
    #[cfg(feature = "oauth-proxy")] cookie_manager: Arc<CookieManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "oauth-proxy")]
    let app = create_app_adr002(token_validator, config, oauth_provider, cookie_manager);

    #[cfg(not(feature = "oauth-proxy"))]
    let app = create_app_adr002(token_validator, config);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!(
        "ADR-002 Resource Server + ADR-004 Proxy OAuth listening on {}",
        addr
    );
    info!(
        "OAuth metadata endpoint: http://{}/.well-known/oauth-protected-resource",
        addr
    );
    #[cfg(feature = "oauth-proxy")]
    info!("OAuth proxy endpoints: /oauth/authorize, /oauth/callback, /oauth/token");
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
    #[cfg(feature = "oauth-proxy")]
    fn test_create_app_adr002() {
        let token_validator = Arc::new(TokenValidator::new());
        let config = Arc::new(Config::from_env_or_file().unwrap());
        let oauth_provider = Arc::new(MiroOAuthProvider::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.redirect_uri.clone(),
        ));
        let cookie_manager = Arc::new(CookieManager::new(&config.encryption_key));
        let app = create_app_adr002(token_validator, config, oauth_provider, cookie_manager);
        assert!(std::mem::size_of_val(&app) > 0);
    }

    #[test]
    #[cfg(not(feature = "oauth-proxy"))]
    fn test_create_app_adr002() {
        let token_validator = Arc::new(TokenValidator::new());
        let config = Arc::new(Config::from_env_or_file().unwrap());
        let app = create_app_adr002(token_validator, config);
        assert!(std::mem::size_of_val(&app) > 0);
    }
}
