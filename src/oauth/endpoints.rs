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

use super::{
    pkce::generate_pkce_pair,
    types::OAuthState,
};

/// Cookie name for OAuth state during authorization flow
const STATE_COOKIE_NAME: &str = "miro_oauth_state";

/// Cookie name for access token after successful authorization
const TOKEN_COOKIE_NAME: &str = "miro_auth";

/// OAuth state cookie max age (5 minutes for authorization flow)
const STATE_COOKIE_MAX_AGE: i64 = 300; // 5 minutes in seconds

/// Token cookie max age (60 days to match Miro refresh token validity)
const TOKEN_COOKIE_MAX_AGE: i64 = 60 * 24 * 60 * 60; // 60 days in seconds

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
/// Generates PKCE pair, creates state nonce, stores in encrypted cookie,
/// then redirects to Miro authorization endpoint.
///
/// # Flow
/// 1. Generate PKCE code verifier and challenge
/// 2. Generate random state nonce (CSRF protection)
/// 3. Store state and PKCE verifier in encrypted cookie
/// 4. Redirect user to Miro authorization URL with PKCE challenge
pub async fn authorize_handler(
    State(state): State<crate::http_server::AppStateADR002>,
) -> Result<Response, OAuthEndpointError> {
    let provider = &state.oauth_provider;
    let cookie_manager = &state.cookie_manager;
    info!("Starting OAuth authorization flow");

    // Generate PKCE pair
    let pkce = generate_pkce_pair();
    info!("Generated PKCE pair");

    // Generate random state nonce (32 bytes = 43 chars base64url)
    let mut rng = rand::thread_rng();
    let state_bytes: [u8; 32] = rng.gen();
    let state = URL_SAFE_NO_PAD.encode(state_bytes);

    // Create OAuth state for cookie storage
    let oauth_state = OAuthState {
        state: state.clone(),
        code_verifier: pkce.verifier,
        redirect_uri: String::from("https://claude.ai/oauth/callback"), // Claude.ai callback
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
        [(header::SET_COOKIE, cookie_header), (header::LOCATION, auth_url.to_string())],
    )
        .into_response())
}

/// Handle GET /oauth/callback - OAuth callback from Miro
///
/// Validates state, exchanges authorization code for access token,
/// stores token in encrypted cookie, and redirects back to Claude.ai.
///
/// # Flow
/// 1. Extract and validate state from cookie
/// 2. Verify state parameter matches cookie
/// 3. Exchange authorization code for access token (with PKCE verifier)
/// 4. Store access token in encrypted cookie
/// 5. Redirect to Claude.ai success URL
pub async fn callback_handler(
    State(state): State<crate::http_server::AppStateADR002>,
    Query(params): Query<CallbackParams>,
    headers: HeaderMap,
) -> Result<Response, OAuthEndpointError> {
    let provider = &state.oauth_provider;
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
    let code = params
        .code
        .as_ref()
        .ok_or_else(|| OAuthEndpointError::InvalidRequest("Authorization code missing".to_string()))?;

    // Exchange code for access token
    info!("Exchanging authorization code for access token");
    let cookie_data = provider
        .exchange_code_for_token(code, &oauth_state.code_verifier)
        .await
        .map_err(|e| OAuthEndpointError::OAuthError(format!("Token exchange failed: {}", e)))?;

    info!(
        user_id = %cookie_data.user_info.user_id,
        expires_at = %cookie_data.expires_at,
        "Successfully obtained access token"
    );

    // Encrypt and store token in cookie
    let encrypted_token = cookie_manager
        .encrypt(&cookie_data)
        .map_err(|e| OAuthEndpointError::CookieError(format!("Failed to encrypt token: {}", e)))?;

    // Build response with token cookie and redirect to Claude.ai
    let token_cookie = format!(
        "{}={}; HttpOnly; Secure; SameSite=Lax; Max-Age={}; Path=/",
        TOKEN_COOKIE_NAME, encrypted_token, TOKEN_COOKIE_MAX_AGE
    );

    // Clear state cookie (no longer needed)
    let clear_state_cookie = format!(
        "{}=; HttpOnly; Secure; SameSite=Lax; Max-Age=0; Path=/",
        STATE_COOKIE_NAME
    );

    Ok((
        StatusCode::FOUND,
        [
            (header::SET_COOKIE, token_cookie),
            (header::SET_COOKIE, clear_state_cookie),
            (header::LOCATION, oauth_state.redirect_uri),
        ],
    )
        .into_response())
}

/// Handle POST /oauth/token - Return access token to Claude.ai
///
/// Extracts token from encrypted cookie and returns in RFC 6749 format.
/// Claude.ai calls this endpoint to retrieve the access token after OAuth completion.
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
) -> Result<Json<TokenResponseRfc6749>, OAuthEndpointError> {
    let cookie_manager = &state.cookie_manager;
    info!("Handling token request");

    // Extract token from cookie
    let token_cookie = extract_cookie(&headers, TOKEN_COOKIE_NAME).ok_or_else(|| {
        OAuthEndpointError::Unauthorized("Token cookie not found - user must authorize first".to_string())
    })?;

    let cookie_data: super::types::CookieData = cookie_manager.decrypt(&token_cookie).map_err(|e| {
        OAuthEndpointError::Unauthorized(format!("Failed to decrypt token: {}", e))
    })?;

    // Calculate remaining time until expiration
    let now = Utc::now();
    let expires_in = (cookie_data.expires_at - now).num_seconds().max(0);

    info!(
        user_id = %cookie_data.user_info.user_id,
        expires_in = %expires_in,
        "Returning access token to Claude.ai"
    );

    // Return token in RFC 6749 format
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
            let mut parts = cookie.trim().splitn(2, '=');
            let name = parts.next()?;
            let value = parts.next()?;
            if name == cookie_name {
                Some(value.to_string())
            } else {
                None
            }
        })
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
            OAuthEndpointError::InvalidState(msg) => (StatusCode::BAD_REQUEST, format!("Invalid state: {}", msg)),
            OAuthEndpointError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, format!("Invalid request: {}", msg)),
            OAuthEndpointError::OAuthError(msg) => (StatusCode::BAD_REQUEST, format!("OAuth error: {}", msg)),
            OAuthEndpointError::CookieError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Cookie error: {}", msg)),
            OAuthEndpointError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, format!("Unauthorized: {}", msg)),
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
            "session=abc123; miro_auth=token456; other=xyz".parse().unwrap(),
        );

        assert_eq!(extract_cookie(&headers, "miro_auth"), Some("token456".to_string()));
        assert_eq!(extract_cookie(&headers, "session"), Some("abc123".to_string()));
        assert_eq!(extract_cookie(&headers, "nonexistent"), None);
    }

    #[test]
    fn test_extract_cookie_with_spaces() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            "miro_auth=token456 ; other=xyz".parse().unwrap(),
        );

        assert_eq!(extract_cookie(&headers, "miro_auth"), Some("token456".to_string()));
    }
}
