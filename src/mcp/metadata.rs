use axum::{extract::State, response::IntoResponse, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::config::Config;

/// OAuth 2.0 Authorization Server Metadata per RFC 8414
/// This is what Claude.ai expects for OAuth auto-discovery in Proxy OAuth pattern (ADR-004)
#[derive(Serialize, Debug)]
pub struct OAuthAuthorizationServerMetadata {
    /// OAuth 2.0 issuer identifier (actual provider domain)
    pub issuer: String,
    /// OAuth 2.0 authorization endpoint URL (points to our proxy)
    pub authorization_endpoint: String,
    /// OAuth 2.0 token endpoint URL (points to our proxy)
    pub token_endpoint: String,
    /// Grant types supported (authorization_code for standard OAuth flow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_types_supported: Option<Vec<String>>,
    /// Response types supported (code for authorization code flow)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_types_supported: Option<Vec<String>>,
}

/// Handle OAuth protected resource metadata endpoint (ADR-004 Proxy OAuth pattern)
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

    Json(OAuthAuthorizationServerMetadata {
        issuer: "https://miro.com".to_string(),
        authorization_endpoint: format!("{}/oauth/authorize", base_url),
        token_endpoint: format!("{}/oauth/token", base_url),
        grant_types_supported: Some(vec!["authorization_code".to_string()]),
        response_types_supported: Some(vec!["code".to_string()]),
    })
}
