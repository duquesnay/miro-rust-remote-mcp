use super::types::{ClientRegistrationRequest, ClientRegistrationResponse, RegisteredClient};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

/// In-memory client registry
/// For production, this should be persistent (database)
#[derive(Clone)]
pub struct ClientRegistry {
    clients: Arc<RwLock<HashMap<String, RegisteredClient>>>,
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl ClientRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new OAuth client
    pub fn register(&self, client: RegisteredClient) -> Result<(), String> {
        let mut clients = self.clients.write().map_err(|e| e.to_string())?;
        clients.insert(client.client_id.clone(), client);
        Ok(())
    }

    /// Get a registered client by ID
    pub fn get(&self, client_id: &str) -> Option<RegisteredClient> {
        let clients = self.clients.read().ok()?;
        clients.get(client_id).cloned()
    }

    /// Validate client credentials
    pub fn validate(&self, client_id: &str, client_secret: &str) -> bool {
        if let Some(client) = self.get(client_id) {
            client.client_secret == client_secret
        } else {
            false
        }
    }
}

/// Handle Dynamic Client Registration (RFC 7591)
/// POST /register
pub async fn register_handler(
    State(registry): State<ClientRegistry>,
    Json(req): Json<ClientRegistrationRequest>,
) -> Result<Json<ClientRegistrationResponse>, Response> {
    info!(
        client_name = %req.client_name,
        redirect_uris = ?req.redirect_uris,
        "Received client registration request"
    );

    // Validate request
    if req.client_name.is_empty() {
        warn!("Registration rejected: empty client_name");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_client_metadata",
                "error_description": "client_name is required"
            })),
        )
            .into_response());
    }

    if req.redirect_uris.is_empty() {
        warn!("Registration rejected: no redirect_uris");
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_redirect_uri",
                "error_description": "At least one redirect_uri is required"
            })),
        )
            .into_response());
    }

    // Validate redirect URIs are HTTPS (except localhost for development)
    for uri in &req.redirect_uris {
        if !uri.starts_with("https://") && !uri.starts_with("http://localhost") {
            warn!(uri = %uri, "Registration rejected: non-HTTPS redirect_uri");
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_redirect_uri",
                    "error_description": "redirect_uri must use HTTPS (or http://localhost for development)"
                })),
            )
                .into_response());
        }
    }

    // Generate client credentials
    let client_id = uuid::Uuid::new_v4().to_string();
    let client_secret = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();

    // Default grant types if not specified
    let grant_types = if req.grant_types.is_empty() {
        vec!["authorization_code".to_string()]
    } else {
        req.grant_types
    };

    // Default response types if not specified
    let response_types = if req.response_types.is_empty() {
        vec!["code".to_string()]
    } else {
        req.response_types
    };

    // Default token endpoint auth method
    let token_endpoint_auth_method = req
        .token_endpoint_auth_method
        .unwrap_or_else(|| "client_secret_basic".to_string());

    // Create registered client
    let client = RegisteredClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        client_name: req.client_name.clone(),
        redirect_uris: req.redirect_uris.clone(),
        grant_types: grant_types.clone(),
        created_at: now,
    };

    // Store client
    if let Err(e) = registry.register(client) {
        warn!(error = %e, "Failed to register client");
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "server_error",
                "error_description": "Failed to register client"
            })),
        )
            .into_response());
    }

    info!(
        client_id = %client_id,
        client_name = %req.client_name,
        "Client registered successfully"
    );

    // Return registration response
    Ok(Json(ClientRegistrationResponse {
        client_id,
        client_secret,
        registration_access_token: None,
        registration_client_uri: None,
        client_name: req.client_name,
        redirect_uris: req.redirect_uris,
        grant_types,
        response_types,
        token_endpoint_auth_method,
        client_id_issued_at: now.timestamp(),
        client_secret_expires_at: None, // Never expires
    }))
}
