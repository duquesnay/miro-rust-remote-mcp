use super::types::{AuthError, TokenSet};
use crate::config::Config;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope,
    TokenResponse, TokenUrl,
};

// Miro OAuth2 endpoints
const MIRO_AUTH_URL: &str = "https://miro.com/oauth/authorize";
const MIRO_TOKEN_URL: &str = "https://api.miro.com/v1/oauth/token";

/// OAuth2 client for Miro authentication
#[derive(Clone)]
pub struct MiroOAuthClient {
    client: BasicClient,
}

impl MiroOAuthClient {
    /// Create a new Miro OAuth2 client
    pub fn new(config: &Config) -> Result<Self, AuthError> {
        let client_id = ClientId::new(config.client_id.clone());
        let client_secret = ClientSecret::new(config.client_secret.clone());

        let auth_url = AuthUrl::new(MIRO_AUTH_URL.to_string())
            .map_err(|e| AuthError::OAuth2Error(format!("Invalid auth URL: {}", e)))?;

        let token_url = TokenUrl::new(MIRO_TOKEN_URL.to_string())
            .map_err(|e| AuthError::OAuth2Error(format!("Invalid token URL: {}", e)))?;

        let redirect_url = RedirectUrl::new(config.redirect_uri.clone())
            .map_err(|e| AuthError::OAuth2Error(format!("Invalid redirect URI: {}", e)))?;

        let client = BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url);

        Ok(Self { client })
    }

    /// Generate authorization URL with PKCE and CSRF protection
    /// Returns (auth_url, csrf_token, pkce_verifier) - caller must store verifier in cookie
    pub fn get_authorization_url(&self) -> Result<(String, CsrfToken, PkceCodeVerifier), AuthError> {
        // Generate PKCE challenge
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        // Generate authorization URL with state
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("boards:read".to_string()))
            .add_scope(Scope::new("boards:write".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        Ok((auth_url.to_string(), csrf_token, pkce_verifier))
    }

    /// Exchange authorization code for access token
    /// Takes pkce_verifier as parameter (retrieved from cookie by caller)
    pub async fn exchange_code(
        &self,
        code: String,
        pkce_verifier: PkceCodeVerifier,
    ) -> Result<TokenSet, AuthError> {

        // Exchange code for token
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code))
            .set_pkce_verifier(pkce_verifier)
            .request_async(async_http_client)
            .await?;

        // Extract token details
        let access_token = token_response.access_token().secret().to_string();
        let refresh_token = token_response
            .refresh_token()
            .map(|t| t.secret().to_string());
        let expires_in = token_response
            .expires_in()
            .map(|d| d.as_secs())
            .unwrap_or(3600); // Default to 1 hour if not specified

        Ok(TokenSet::new(access_token, refresh_token, expires_in))
    }

    /// Refresh access token using refresh token
    pub async fn refresh_access_token(
        &self,
        refresh_token_str: String,
    ) -> Result<TokenSet, AuthError> {
        let refresh_token = RefreshToken::new(refresh_token_str);

        let token_response = self
            .client
            .exchange_refresh_token(&refresh_token)
            .request_async(async_http_client)
            .await?;

        // Extract token details
        let access_token = token_response.access_token().secret().to_string();
        let new_refresh_token = token_response
            .refresh_token()
            .map(|t| t.secret().to_string());
        let expires_in = token_response
            .expires_in()
            .map(|d| d.as_secs())
            .unwrap_or(3600);

        Ok(TokenSet::new(
            access_token,
            new_refresh_token.or(Some(refresh_token.secret().to_string())),
            expires_in,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_oauth_client_creation() {
        let config = get_test_config();
        let result = MiroOAuthClient::new(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_authorization_url_generation() {
        let config = get_test_config();
        let client = MiroOAuthClient::new(&config).unwrap();

        let result = client.get_authorization_url();
        assert!(result.is_ok());

        let (url, _csrf_token, _pkce_verifier) = result.unwrap();
        assert!(url.contains("https://miro.com/oauth/authorize"));
        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("code_challenge"));
        assert!(url.contains("state"));
    }
}
