use axum::{response::IntoResponse, Json};
use serde::Serialize;

/// OAuth protected resource metadata per RFC 9728
#[derive(Serialize, Debug)]
pub struct OAuthProtectedResource {
    /// List of resources protected by OAuth2
    pub protected_resources: Vec<ProtectedResourceInfo>,
}

/// Information about a single protected resource
#[derive(Serialize, Debug)]
pub struct ProtectedResourceInfo {
    /// The resource URI (e.g., Miro API endpoint)
    pub resource: String,
    /// Authorization servers that protect this resource
    pub authorization_servers: Vec<String>,
}

/// Handle OAuth protected resource metadata endpoint
/// Returns metadata about which OAuth servers protect which resources
/// This tells clients (like Claude) where to find OAuth authorization
pub async fn oauth_metadata() -> impl IntoResponse {
    Json(OAuthProtectedResource {
        protected_resources: vec![ProtectedResourceInfo {
            resource: "https://api.miro.com".to_string(),
            authorization_servers: vec!["https://miro.com/oauth".to_string()],
        }],
    })
}
