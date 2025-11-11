/// OAuth 2.1 Dynamic Client Registration (DCR) for Remote MCP
///
/// Implements OAuth 2.1 endpoints required for Claude Desktop remote MCP integration:
/// - /.well-known/oauth-authorization-server (metadata discovery)
/// - /oauth/register (dynamic client registration)
/// - /oauth/authorize (authorization with user consent)
/// - /oauth/token (token exchange - already exists, needs DCR support)
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OAuthConfig {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
}

/// OAuth 2.1 Authorization Server Metadata (RFC 8414)
#[derive(Serialize)]
pub struct AuthorizationServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: String,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

/// Dynamic Client Registration Request (RFC 7591)
#[derive(Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: Option<String>,
    pub grant_types: Option<Vec<String>>,
    pub response_types: Option<Vec<String>>,
    pub scope: Option<String>,
}

/// Dynamic Client Registration Response (RFC 7591)
#[derive(Serialize, Clone)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: String,
    pub client_id_issued_at: i64,
    pub client_secret_expires_at: i64,
    pub redirect_uris: Vec<String>,
    pub token_endpoint_auth_method: String,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
}

/// Authorization Request Parameters
#[derive(Deserialize)]
pub struct AuthorizeParams {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Registered client store (in-memory for MVP)
type ClientStore = Arc<RwLock<std::collections::HashMap<String, ClientRegistrationResponse>>>;

/// OAuth 2.1 Authorization Server Metadata endpoint
/// GET /.well-known/oauth-authorization-server
async fn metadata(State(config): State<OAuthConfig>) -> Json<AuthorizationServerMetadata> {
    Json(AuthorizationServerMetadata {
        issuer: config.base_url.clone(),
        authorization_endpoint: format!("{}/oauth/authorize", config.base_url),
        token_endpoint: format!("{}/oauth/token", config.base_url),
        registration_endpoint: format!("{}/oauth/register", config.base_url),
        scopes_supported: vec!["boards:read".to_string(), "boards:write".to_string()],
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic".to_string(),
            "client_secret_post".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string()],
    })
}

/// Dynamic Client Registration endpoint
/// POST /oauth/register
async fn register(
    axum::Extension(store): axum::Extension<ClientStore>,
    Json(request): Json<ClientRegistrationRequest>,
) -> Result<Json<ClientRegistrationResponse>, StatusCode> {
    // Generate client credentials
    let client_id = format!("client_{}", uuid::Uuid::new_v4());
    let client_secret = format!("secret_{}", uuid::Uuid::new_v4());

    let response = ClientRegistrationResponse {
        client_id: client_id.clone(),
        client_secret,
        client_id_issued_at: chrono::Utc::now().timestamp(),
        client_secret_expires_at: 0, // Never expires
        redirect_uris: request.redirect_uris.clone(),
        token_endpoint_auth_method: request
            .token_endpoint_auth_method
            .unwrap_or_else(|| "client_secret_basic".to_string()),
        grant_types: request
            .grant_types
            .unwrap_or_else(|| vec!["authorization_code".to_string()]),
        response_types: request
            .response_types
            .unwrap_or_else(|| vec!["code".to_string()]),
    };

    // Store registered client
    store.write().await.insert(client_id, response.clone());

    Ok(Json(response))
}

/// Authorization endpoint with user consent page
/// GET /oauth/authorize
async fn authorize(
    Query(params): Query<AuthorizeParams>,
    axum::Extension(_store): axum::Extension<ClientStore>,
) -> Response {
    // TODO: Validate client_id exists in store
    // TODO: Add actual user authentication
    // TODO: Store authorization request in session

    // For now, show a simple consent page
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Authorize Miro MCP Server</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 600px;
            margin: 100px auto;
            padding: 20px;
            background: #f5f5f5;
        }}
        .card {{
            background: white;
            border-radius: 8px;
            padding: 40px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        h1 {{
            color: #333;
            margin-bottom: 20px;
        }}
        .info {{
            background: #f9f9f9;
            border-left: 4px solid #4CAF50;
            padding: 15px;
            margin: 20px 0;
        }}
        .scopes {{
            list-style: none;
            padding: 0;
        }}
        .scopes li {{
            padding: 8px 0;
            border-bottom: 1px solid #eee;
        }}
        .buttons {{
            margin-top: 30px;
            display: flex;
            gap: 10px;
        }}
        button {{
            flex: 1;
            padding: 12px 24px;
            font-size: 16px;
            border-radius: 4px;
            border: none;
            cursor: pointer;
            font-weight: 500;
        }}
        .approve {{
            background: #4CAF50;
            color: white;
        }}
        .deny {{
            background: #f44336;
            color: white;
        }}
        button:hover {{
            opacity: 0.9;
        }}
    </style>
</head>
<body>
    <div class="card">
        <h1>🔐 Authorize Application</h1>

        <div class="info">
            <p><strong>Client:</strong> {}</p>
            <p><strong>Redirect URI:</strong> {}</p>
        </div>

        <h2>Requested Permissions:</h2>
        <ul class="scopes">
            <li>✓ Read your Miro boards</li>
            <li>✓ Create and modify items on boards</li>
            <li>✓ List board contents</li>
        </ul>

        <div class="buttons">
            <form method="post" action="/oauth/approve" style="flex: 1;">
                <input type="hidden" name="client_id" value="{}">
                <input type="hidden" name="redirect_uri" value="{}">
                <input type="hidden" name="state" value="{}">
                <input type="hidden" name="code_challenge" value="{}">
                <input type="hidden" name="code_challenge_method" value="{}">
                <button type="submit" class="approve">✓ Approve</button>
            </form>
            <form method="post" action="/oauth/deny" style="flex: 1;">
                <input type="hidden" name="redirect_uri" value="{}">
                <input type="hidden" name="state" value="{}">
                <button type="submit" class="deny">✗ Deny</button>
            </form>
        </div>
    </div>
</body>
</html>"#,
        params.client_id,
        params.redirect_uri,
        params.client_id,
        params.redirect_uri,
        params.state.as_deref().unwrap_or(""),
        params.code_challenge.as_deref().unwrap_or(""),
        params.code_challenge_method.as_deref().unwrap_or(""),
        params.redirect_uri,
        params.state.as_deref().unwrap_or(""),
    );

    Html(html).into_response()
}

/// Approval endpoint (user clicked "Approve")
/// POST /oauth/approve
async fn approve(/* TODO: extract form data */) -> Response {
    // TODO: Generate authorization code
    // TODO: Store code with PKCE challenge
    // TODO: Redirect to redirect_uri with code and state

    Html("<h1>TODO: Generate auth code and redirect</h1>").into_response()
}

/// Denial endpoint (user clicked "Deny")
/// POST /oauth/deny
async fn deny(/* TODO: extract form data */) -> Response {
    // TODO: Redirect to redirect_uri with error=access_denied

    Html("<h1>TODO: Redirect with error</h1>").into_response()
}

/// Create OAuth DCR router
pub fn create_oauth_router(config: OAuthConfig) -> Router {
    let client_store: ClientStore = Arc::new(RwLock::new(std::collections::HashMap::new()));

    Router::new()
        .route("/.well-known/oauth-authorization-server", get(metadata))
        .route("/oauth/register", post(register))
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/approve", post(approve))
        .route("/oauth/deny", post(deny))
        .with_state(config.clone())
        .layer(axum::Extension(client_store))
}
