use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::{pkce::generate_pkce_pair, types::OAuthState};

/// Cookie name for OAuth state during authorization flow
const STATE_COOKIE_NAME: &str = "miro_oauth_state";

/// Cookie name for pending authorization code (temporary storage between callback and token endpoint)
const PENDING_CODE_COOKIE_NAME: &str = "miro_pending_code";

/// OAuth state cookie max age (5 minutes for authorization flow)
const STATE_COOKIE_MAX_AGE: i64 = 300; // 5 minutes in seconds

/// Pending code cookie max age (5 minutes - code must be exchanged quickly)
const PENDING_CODE_MAX_AGE: i64 = 300; // 5 minutes in seconds

/// Query parameters for OAuth authorization request (RFC 6749)
#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    /// OAuth response type (should be "code")
    response_type: String,

    /// Client ID from Claude.ai
    client_id: String,

    /// Redirect URI where Claude.ai wants the authorization code sent
    redirect_uri: String,

    /// State parameter from Claude.ai for CSRF protection
    #[serde(default)]
    state: Option<String>,

    /// Requested OAuth scopes
    #[serde(default)]
    scope: Option<String>,
}

/// Query parameters for OAuth callback
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    /// Authorization code from Miro
    code: Option<String>,

    /// State parameter for CSRF protection
    state: Option<String>,

    /// OAuth error (if authorization failed)
    error: Option<String>,

    /// OAuth error description
    error_description: Option<String>,
}

/// Token response format (RFC 6749)
#[derive(Debug, Serialize)]
pub struct TokenResponseRfc6749 {
    access_token: String,
    token_type: String,
    expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
}

/// Handle GET /oauth/authorize - Initiate OAuth flow with Miro
///
/// Receives authorization request from Claude.ai, generates PKCE pair,
/// stores state in encrypted cookie, then redirects to Miro.
///
/// # Flow
/// 1. Extract and validate authorization request parameters from Claude.ai
/// 2. Generate PKCE code verifier and challenge
/// 3. Generate random state nonce (CSRF protection)
/// 4. Store state, PKCE verifier, and Claude's redirect_uri in encrypted cookie
/// 5. Redirect user to Miro authorization URL with PKCE challenge
pub async fn authorize_handler(
    State(state): State<crate::http_server::AppStateADR002>,
    Query(params): Query<AuthorizeParams>,
) -> Result<Response, OAuthEndpointError> {
    let provider = &state.oauth_provider;
    let cookie_manager = &state.cookie_manager;

    info!(
        client_id = %params.client_id,
        redirect_uri = %params.redirect_uri,
        response_type = %params.response_type,
        state = ?params.state,
        scope = ?params.scope,
        "Starting OAuth authorization flow from Claude.ai"
    );

    // Validate response_type (must be "code" for authorization code flow)
    if params.response_type != "code" {
        warn!(response_type = %params.response_type, "Invalid response_type");
        return Err(OAuthEndpointError::InvalidRequest(format!(
            "Unsupported response_type: {}. Only 'code' is supported.",
            params.response_type
        )));
    }

    // Generate PKCE pair
    let pkce = generate_pkce_pair();
    info!("Generated PKCE pair");

    // Generate random state nonce (32 bytes = 43 chars base64url)
    let mut rng = rand::thread_rng();
    let state_bytes: [u8; 32] = rng.gen();
    let state = URL_SAFE_NO_PAD.encode(state_bytes);

    // Create OAuth state for cookie storage (use redirect_uri from Claude.ai's request)
    let oauth_state = OAuthState {
        state: state.clone(),
        code_verifier: pkce.verifier,
        redirect_uri: params.redirect_uri.clone(), // From Claude.ai's authorization request
    };

    // Encrypt and store state in cookie
    let encrypted_state = cookie_manager
        .encrypt(&oauth_state)
        .map_err(|e| OAuthEndpointError::CookieError(format!("Failed to encrypt state: {}", e)))?;

    // Build Miro authorization URL
    let auth_url = provider
        .build_authorization_url(&state, &pkce.challenge)
        .map_err(|e| OAuthEndpointError::OAuthError(format!("Failed to build auth URL: {}", e)))?;

    info!(
        auth_url = %auth_url,
        "Redirecting to Miro authorization endpoint"
    );

    // Build response with state cookie and redirect
    let cookie_header = format!(
        "{}={}; HttpOnly; Secure; SameSite=Lax; Max-Age={}; Path=/",
        STATE_COOKIE_NAME, encrypted_state, STATE_COOKIE_MAX_AGE
    );

    Ok((
        StatusCode::FOUND,
        [
            (header::SET_COOKIE, cookie_header),
            (header::LOCATION, auth_url.to_string()),
        ],
    )
        .into_response())
}

/// Handle GET /oauth/callback - OAuth callback from Miro
///
/// Receives authorization code from Miro and redirects it to Claude.ai.
/// This implements the standard OAuth2 Authorization Server flow where:
/// - Miro sends the code to our server
/// - We store the code + PKCE verifier temporarily
/// - We redirect to Claude.ai WITH the code in the URL
/// - Claude.ai then calls /oauth/token to exchange the code
///
/// # Flow
/// 1. Extract and validate state from cookie (CSRF protection)
/// 2. Verify state parameter matches cookie
/// 3. Store authorization code + PKCE verifier in encrypted cookie (temporary)
/// 4. Redirect to Claude.ai WITH code in URL: redirect_uri?code=XXX&state=YYY
pub async fn callback_handler(
    State(state): State<crate::http_server::AppStateADR002>,
    Query(params): Query<CallbackParams>,
    headers: HeaderMap,
) -> Result<Response, OAuthEndpointError> {
    let cookie_manager = &state.cookie_manager;
    info!("Handling OAuth callback from Miro");

    // Check for OAuth error from Miro
    if let Some(error) = params.error {
        let description = params.error_description.unwrap_or_default();
        error!(error = %error, description = %description, "OAuth error from Miro");
        return Err(OAuthEndpointError::OAuthError(format!(
            "Miro OAuth error: {} - {}",
            error, description
        )));
    }

    // Extract state from cookie
    let state_cookie = extract_cookie(&headers, STATE_COOKIE_NAME)
        .ok_or_else(|| OAuthEndpointError::InvalidState("State cookie not found".to_string()))?;

    let oauth_state: OAuthState = cookie_manager
        .decrypt(&state_cookie)
        .map_err(|e| OAuthEndpointError::InvalidState(format!("Failed to decrypt state: {}", e)))?;

    // Validate state parameter matches cookie
    let state_param = params
        .state
        .as_ref()
        .ok_or_else(|| OAuthEndpointError::InvalidState("State parameter missing".to_string()))?;

    if state_param != &oauth_state.state {
        warn!("State parameter mismatch - possible CSRF attack");
        return Err(OAuthEndpointError::InvalidState(
            "State parameter mismatch".to_string(),
        ));
    }

    info!("State validated successfully");

    // Extract authorization code
    let code = params.code.as_ref().ok_or_else(|| {
        OAuthEndpointError::InvalidRequest("Authorization code missing".to_string())
    })?;

    info!(
        code_length = code.len(),
        "Received authorization code from Miro"
    );

    // Store code + verifier temporarily for token endpoint to use
    let pending_exchange = super::types::PendingCodeExchange {
        code: code.clone(),
        code_verifier: oauth_state.code_verifier.clone(),
        expires_at: Utc::now() + chrono::Duration::seconds(PENDING_CODE_MAX_AGE),
    };

    let encrypted_pending = cookie_manager.encrypt(&pending_exchange).map_err(|e| {
        OAuthEndpointError::CookieError(format!("Failed to encrypt pending code: {}", e))
    })?;

    // Build response: store code in cookie and redirect to Claude.ai WITH the code
    let pending_code_cookie = format!(
        "{}={}; HttpOnly; Secure; SameSite=Lax; Max-Age={}; Path=/",
        PENDING_CODE_COOKIE_NAME, encrypted_pending, PENDING_CODE_MAX_AGE
    );

    // Clear state cookie (no longer needed)
    let clear_state_cookie = format!(
        "{}=; HttpOnly; Secure; SameSite=Lax; Max-Age=0; Path=/",
        STATE_COOKIE_NAME
    );

    // Redirect to Claude.ai WITH the authorization code in URL (standard OAuth2 flow)
    let redirect_url = format!(
        "{}?code={}&state={}",
        oauth_state.redirect_uri, code, state_param
    );

    info!(
        redirect_url = %redirect_url,
        "Redirecting to Claude.ai with authorization code"
    );

    Ok((
        StatusCode::FOUND,
        [
            (header::SET_COOKIE, pending_code_cookie),
            (header::SET_COOKIE, clear_state_cookie),
            (header::LOCATION, redirect_url),
        ],
    )
        .into_response())
}

/// Handle POST /oauth/token - Exchange authorization code for access token
///
/// Standard OAuth2 token endpoint that Claude.ai calls to exchange the authorization code
/// for an access token. This endpoint:
/// 1. Extracts authorization code from request (from Claude.ai)
/// 2. Retrieves PKCE verifier from encrypted cookie (stored during callback)
/// 3. Exchanges code with Miro API for access token
/// 4. Returns token in RFC 6749 format
///
/// # Request Format (application/x-www-form-urlencoded or JSON)
/// ```text
/// grant_type=authorization_code
/// code=<authorization_code>
/// redirect_uri=<redirect_uri>
/// client_id=<client_id>
/// code_verifier=<pkce_verifier> (optional, we have it in cookie)
/// ```
///
/// # Response Format (RFC 6749)
/// ```json
/// {
///   "access_token": "...",
///   "token_type": "Bearer",
///   "expires_in": 3600,
///   "refresh_token": "...",
///   "scope": "boards:read boards:write"
/// }
/// ```
pub async fn token_handler(
    State(state): State<crate::http_server::AppStateADR002>,
    headers: HeaderMap,
    axum::extract::Form(token_request): axum::extract::Form<super::types::TokenRequest>,
) -> Result<Json<TokenResponseRfc6749>, OAuthEndpointError> {
    let provider = &state.oauth_provider;
    let cookie_manager = &state.cookie_manager;
    let config = &state.config;
    let client_registry = &state.client_registry;

    info!(
        grant_type = %token_request.grant_type,
        client_id = %token_request.client_id,
        "Token endpoint called by Claude.ai"
    );

    // Validate grant_type
    if token_request.grant_type != "authorization_code" {
        return Err(OAuthEndpointError::InvalidRequest(format!(
            "Unsupported grant_type: {}",
            token_request.grant_type
        )));
    }

    // Extract client_secret from either Authorization header (client_secret_basic) or form body (client_secret_post)
    let client_secret = extract_client_secret(&headers, &token_request);

    // Validate client credentials
    // Priority: 1) DCR registered clients, 2) Manual config client_id (backwards compatibility)
    let is_valid_client = if let Some(ref secret) = client_secret {
        // Client provided secret - validate against registry (DCR)
        info!(client_id = %token_request.client_id, "Validating DCR registered client");
        client_registry.validate(&token_request.client_id, secret)
    } else {
        // No secret provided - check if it's our manual config client (backwards compatibility)
        info!(client_id = %token_request.client_id, "Checking manual config client");
        token_request.client_id == config.client_id
    };

    if !is_valid_client {
        warn!(
            client_id = %token_request.client_id,
            has_secret = client_secret.is_some(),
            "Client authentication failed"
        );
        return Err(OAuthEndpointError::Unauthorized(
            "Invalid client credentials".to_string(),
        ));
    }

    info!(client_id = %token_request.client_id, "Client authenticated successfully");

    // Extract pending code exchange from cookie
    let pending_cookie = extract_cookie(&headers, PENDING_CODE_COOKIE_NAME).ok_or_else(|| {
        OAuthEndpointError::Unauthorized(
            "Pending code cookie not found - authorization flow not completed".to_string(),
        )
    })?;

    let pending_exchange: super::types::PendingCodeExchange =
        cookie_manager.decrypt(&pending_cookie).map_err(|e| {
            OAuthEndpointError::Unauthorized(format!("Failed to decrypt pending code: {}", e))
        })?;

    // Check if code has expired
    let now = Utc::now();
    if now > pending_exchange.expires_at {
        return Err(OAuthEndpointError::Unauthorized(
            "Authorization code has expired".to_string(),
        ));
    }

    // Validate code matches what we received from callback
    if token_request.code != pending_exchange.code {
        warn!("Authorization code mismatch");
        return Err(OAuthEndpointError::Unauthorized(
            "Authorization code mismatch".to_string(),
        ));
    }

    info!("Exchanging authorization code with Miro API");

    // Exchange code for access token with Miro
    let cookie_data = provider
        .exchange_code_for_token(&pending_exchange.code, &pending_exchange.code_verifier)
        .await
        .map_err(|e| {
            OAuthEndpointError::OAuthError(format!("Token exchange with Miro failed: {}", e))
        })?;

    // Calculate token expiration
    let expires_in = (cookie_data.expires_at - now).num_seconds().max(0);

    info!(
        user_id = %cookie_data.user_info.user_id,
        expires_in = %expires_in,
        "Successfully exchanged code for access token"
    );

    // Return token in RFC 6749 format to Claude.ai
    Ok(Json(TokenResponseRfc6749 {
        access_token: cookie_data.access_token,
        token_type: "Bearer".to_string(),
        expires_in,
        refresh_token: Some(cookie_data.refresh_token),
        scope: Some("boards:read boards:write".to_string()),
    }))
}

/// Extract cookie value from request headers
fn extract_cookie(headers: &HeaderMap, cookie_name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;
            if name == cookie_name {
                Some(value.to_string())
            } else {
                None
            }
        })
}

/// Extract client_secret from request
/// Supports both client_secret_basic (Authorization header) and client_secret_post (form body)
fn extract_client_secret(
    headers: &HeaderMap,
    token_request: &super::types::TokenRequest,
) -> Option<String> {
    // Try client_secret_basic (Authorization: Basic base64(client_id:client_secret))
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(basic_token) = auth_str.strip_prefix("Basic ") {
                // Decode base64
                if let Ok(decoded_bytes) = URL_SAFE_NO_PAD.decode(basic_token.as_bytes()) {
                    if let Ok(decoded_str) = String::from_utf8(decoded_bytes) {
                        // Split by : to get client_id:client_secret
                        let (_client_id, client_secret) = decoded_str.split_once(':')?;
                        return Some(client_secret.to_string());
                    }
                }
            }
        }
    }

    // Try client_secret_post (included in form body)
    token_request.client_secret.clone()
}

/// Errors from OAuth endpoint handlers
#[derive(Debug)]
pub enum OAuthEndpointError {
    InvalidState(String),
    InvalidRequest(String),
    OAuthError(String),
    CookieError(String),
    Unauthorized(String),
}

impl IntoResponse for OAuthEndpointError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            OAuthEndpointError::InvalidState(msg) => {
                (StatusCode::BAD_REQUEST, format!("Invalid state: {}", msg))
            }
            OAuthEndpointError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, format!("Invalid request: {}", msg))
            }
            OAuthEndpointError::OAuthError(msg) => {
                (StatusCode::BAD_REQUEST, format!("OAuth error: {}", msg))
            }
            OAuthEndpointError::CookieError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Cookie error: {}", msg),
            ),
            OAuthEndpointError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, format!("Unauthorized: {}", msg))
            }
        };

        error!(status = %status, message = %message, "OAuth endpoint error");

        (status, message).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "session=abc123; miro_auth=token456; other=xyz"
                .parse()
                .unwrap(),
        );

        assert_eq!(
            extract_cookie(&headers, "miro_auth"),
            Some("token456".to_string())
        );
        assert_eq!(
            extract_cookie(&headers, "session"),
            Some("abc123".to_string())
        );
        assert_eq!(extract_cookie(&headers, "nonexistent"), None);
    }

    #[test]
    fn test_extract_cookie_with_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "miro_auth=token456 ; other=xyz".parse().unwrap(),
        );

        assert_eq!(
            extract_cookie(&headers, "miro_auth"),
            Some("token456".to_string())
        );
    }
}
