use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::config::Config;

/// OAuth 2.0 Authorization Server Metadata (RFC 8414)
/// For Dynamic Client Registration support
#[derive(Serialize, Debug)]
pub struct OAuthAuthorizationServerMetadata {
    /// Authorization server identifier
    pub issuer: String,
    /// Authorization endpoint URL
    pub authorization_endpoint: String,
    /// Token endpoint URL
    pub token_endpoint: String,
    /// Registration endpoint URL (RFC 7591)
    pub registration_endpoint: String,
    /// Grant types supported
    pub grant_types_supported: Vec<String>,
    /// Response types supported
    pub response_types_supported: Vec<String>,
    /// Token endpoint auth methods
    pub token_endpoint_auth_methods_supported: Vec<String>,
}

/// OAuth 2.0 Protected Resource Metadata
/// This is what Claude.ai expects for OAuth auto-discovery in Proxy OAuth pattern (ADR-004)
#[derive(Serialize, Debug)]
pub struct OAuthProtectedResourceMetadata {
    /// OAuth 2.0 issuer identifier (actual provider domain)
    pub issuer: String,
    /// OAuth 2.0 authorization endpoint URL (points to our proxy)
    pub authorization_endpoint: String,
    /// OAuth 2.0 token endpoint URL (points to our proxy)
    pub token_endpoint: String,
    /// Grant types supported (authorization_code for standard OAuth flow)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub grant_types_supported: Vec<String>,
    /// Response types supported (code for authorization code flow)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub response_types_supported: Vec<String>,
}

/// Handle OAuth authorization server metadata endpoint (RFC 8414)
/// GET /.well-known/oauth-authorization-server
/// Returns full authorization server metadata including DCR registration endpoint
pub async fn oauth_authorization_server_metadata(State(config): State<Arc<Config>>) -> impl IntoResponse {
    let base_url = config
        .base_url
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("http://localhost:3000");

    Json(OAuthAuthorizationServerMetadata {
        issuer: base_url.to_string(),
        authorization_endpoint: format!("{}/oauth/authorize", base_url),
        token_endpoint: format!("{}/oauth/token", base_url),
        registration_endpoint: format!("{}/register", base_url),
        grant_types_supported: vec!["authorization_code".to_string()],
        response_types_supported: vec!["code".to_string()],
        token_endpoint_auth_methods_supported: vec!["client_secret_basic".to_string(), "client_secret_post".to_string()],
    })
}

/// Handle OAuth protected resource metadata endpoint (ADR-004 Proxy OAuth pattern)
/// GET /.well-known/oauth-protected-resource
/// Returns OAuth Authorization Server metadata pointing to OUR proxy endpoints
/// This tells Claude.ai to use our server as the OAuth proxy between Claude and Miro
///
/// # Arguments
/// * `config` - Server configuration containing BASE_URL
///
/// # ADR-004 Changes
/// - authorization_endpoint: Points to {BASE_URL}/oauth/authorize (was Miro direct)
/// - token_endpoint: Points to {BASE_URL}/oauth/token (was Miro direct)
/// - issuer: Still "https://miro.com" (the actual OAuth provider)
pub async fn oauth_metadata(State(config): State<Arc<Config>>) -> impl IntoResponse {
    let base_url = config
        .base_url
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("http://localhost:3000");

    Json(OAuthProtectedResourceMetadata {
        issuer: "https://miro.com".to_string(),
        authorization_endpoint: format!("{}/oauth/authorize", base_url),
        token_endpoint: format!("{}/oauth/token", base_url),
        grant_types_supported: vec!["authorization_code".to_string()],
        response_types_supported: vec!["code".to_string()],
    })
}
