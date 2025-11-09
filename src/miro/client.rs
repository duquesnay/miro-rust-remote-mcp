use crate::auth::{AuthError, MiroOAuthClient, TokenStore};
use reqwest::StatusCode;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error types for Miro API operations
#[derive(Debug, thiserror::Error)]
pub enum MiroError {
    #[error("Authentication error: {0}")]
    AuthError(#[from] AuthError),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Unauthorized - token may be expired")]
    Unauthorized,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

/// Miro API client with automatic token refresh
pub struct MiroClient {
    http_client: reqwest::Client,
    token_store: Arc<RwLock<TokenStore>>,
    oauth_client: Arc<MiroOAuthClient>,
}

impl MiroClient {
    /// Create a new Miro API client
    pub fn new(token_store: TokenStore, oauth_client: MiroOAuthClient) -> Result<Self, MiroError> {
        let http_client = reqwest::Client::builder()
            .user_agent("miro-mcp-server/0.1.0")
            .build()?;

        Ok(Self {
            http_client,
            token_store: Arc::new(RwLock::new(token_store)),
            oauth_client: Arc::new(oauth_client),
        })
    }

    /// Get a valid access token, refreshing if necessary
    async fn get_valid_token(&self) -> Result<String, MiroError> {
        let token_store = self.token_store.read().await;
        let tokens = token_store.load()?;

        // Check if token is expired
        if tokens.is_expired() {
            drop(token_store); // Release read lock

            // Refresh the token
            let refresh_token = tokens.refresh_token.ok_or(AuthError::NoToken)?;

            let new_tokens = self
                .oauth_client
                .refresh_access_token(refresh_token)
                .await?;

            // Save the new tokens
            let token_store = self.token_store.write().await;
            token_store.save(&new_tokens)?;

            Ok(new_tokens.access_token)
        } else {
            Ok(tokens.access_token)
        }
    }

    /// Make an authenticated GET request to Miro API
    pub async fn get(&self, path: &str) -> Result<Value, MiroError> {
        self.request("GET", path, None).await
    }

    /// Make an authenticated POST request to Miro API
    pub async fn post(&self, path: &str, body: Option<Value>) -> Result<Value, MiroError> {
        self.request("POST", path, body).await
    }

    /// Make an authenticated PATCH request to Miro API
    pub async fn patch(&self, path: &str, body: Option<Value>) -> Result<Value, MiroError> {
        self.request("PATCH", path, body).await
    }

    /// Make an authenticated DELETE request to Miro API
    pub async fn delete(&self, path: &str) -> Result<Value, MiroError> {
        self.request("DELETE", path, None).await
    }

    /// Make an authenticated request with automatic retry on 401
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, MiroError> {
        let url = format!("https://api.miro.com/v2{}", path);

        // First attempt
        match self.execute_request(method, &url, body.clone()).await {
            Ok(response) => Ok(response),
            Err(MiroError::Unauthorized) => {
                // Token might be expired, force refresh and retry once
                let token_store = self.token_store.read().await;
                let tokens = token_store.load()?;
                drop(token_store);

                if let Some(refresh_token) = tokens.refresh_token {
                    let new_tokens = self
                        .oauth_client
                        .refresh_access_token(refresh_token)
                        .await?;

                    let token_store = self.token_store.write().await;
                    token_store.save(&new_tokens)?;
                    drop(token_store);

                    // Retry the request with new token
                    self.execute_request(method, &url, body).await
                } else {
                    Err(MiroError::Unauthorized)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Execute a single HTTP request
    async fn execute_request(
        &self,
        method: &str,
        url: &str,
        body: Option<Value>,
    ) -> Result<Value, MiroError> {
        let token = self.get_valid_token().await?;

        let mut request = match method {
            "GET" => self.http_client.get(url),
            "POST" => self.http_client.post(url),
            "PATCH" => self.http_client.patch(url),
            "DELETE" => self.http_client.delete(url),
            _ => {
                return Err(MiroError::ApiError {
                    status: 400,
                    message: format!("Unsupported HTTP method: {}", method),
                })
            }
        };

        request = request.bearer_auth(&token);

        if let Some(body_value) = body {
            request = request.json(&body_value);
        }

        let response = request.send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let json = response.json().await?;
                Ok(json)
            }
            StatusCode::NO_CONTENT => Ok(Value::Null),
            StatusCode::UNAUTHORIZED => Err(MiroError::Unauthorized),
            StatusCode::TOO_MANY_REQUESTS => Err(MiroError::RateLimitExceeded),
            status => {
                let message = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                Err(MiroError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn get_test_config() -> Config {
        Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
            encryption_key: [0u8; 32],
            port: 3000,
        }
    }

    #[test]
    fn test_client_creation() {
        let config = get_test_config();
        let token_store = TokenStore::new(config.encryption_key).unwrap();
        let oauth_client = MiroOAuthClient::new(&config).unwrap();

        let result = MiroClient::new(token_store, oauth_client);
        assert!(result.is_ok());
    }
}
