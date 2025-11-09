use crate::auth::{MiroOAuthClient, TokenStore};
use std::sync::Arc;
use tokio::sync::RwLock;

/// OAuth callback handler
pub struct AuthHandler {
    oauth_client: Arc<MiroOAuthClient>,
    token_store: Arc<RwLock<TokenStore>>,
}

impl AuthHandler {
    pub fn new(oauth_client: Arc<MiroOAuthClient>, token_store: Arc<RwLock<TokenStore>>) -> Self {
        Self {
            oauth_client,
            token_store,
        }
    }

    /// Handle OAuth callback with code and state
    pub async fn handle_callback(
        &self,
        code: String,
        state: String,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Exchange code for tokens
        let tokens = self.oauth_client.exchange_code(code, state).await?;

        // Save tokens to encrypted storage
        let token_store = self.token_store.write().await;
        token_store.save(&tokens)?;

        Ok("Authentication successful! Tokens have been saved.".to_string())
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
    fn test_auth_handler_creation() {
        let config = get_test_config();
        let oauth_client = Arc::new(MiroOAuthClient::new(&config).unwrap());
        let token_store = Arc::new(RwLock::new(TokenStore::new(config.encryption_key).unwrap()));

        let handler = AuthHandler::new(oauth_client, token_store);
        assert!(std::mem::size_of_val(&handler) > 0);
    }
}
