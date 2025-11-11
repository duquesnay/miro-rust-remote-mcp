use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Serialize;
use thiserror::Error;
use url::Url;

use super::types::{CookieData, TokenResponse, UserInfo};

/// Miro OAuth endpoints
const MIRO_AUTH_ENDPOINT: &str = "https://miro.com/oauth/authorize";
const MIRO_TOKEN_ENDPOINT: &str = "https://api.miro.com/v1/oauth/token";

/// Miro OAuth scopes
const MIRO_SCOPES: &[&str] = &["boards:read", "boards:write"];

/// Errors from Miro OAuth operations
#[derive(Error, Debug)]
pub enum MiroOAuthError {
    #[error("Failed to build authorization URL: {0}")]
    UrlBuildError(#[from] url::ParseError),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Miro OAuth error: {error} - {error_description}")]
    OAuthError {
        error: String,
        error_description: String,
    },

    #[error("Missing required field in token response: {0}")]
    MissingField(String),

    #[error("Invalid token response: {0}")]
    InvalidResponse(String),
}

/// OAuth proxy provider for Miro
///
/// Handles OAuth 2.0 authorization code flow with PKCE between Claude.ai and Miro.
/// The server acts as a proxy, obtaining tokens from Miro on behalf of Claude.ai.
#[derive(Clone)]
pub struct MiroOAuthProvider {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    http_client: Client,
}

impl MiroOAuthProvider {
    /// Create a new Miro OAuth provider
    ///
    /// # Arguments
    /// * `client_id` - Miro OAuth application client ID
    /// * `client_secret` - Miro OAuth application client secret
    /// * `redirect_uri` - OAuth callback URL (must match Miro app registration)
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client_id,
            client_secret,
            redirect_uri,
            http_client: Client::new(),
        }
    }

    /// Build Miro authorization URL with PKCE challenge
    ///
    /// # Arguments
    /// * `state` - CSRF protection nonce
    /// * `pkce_challenge` - PKCE code challenge (SHA-256 hash of verifier)
    ///
    /// # Returns
    /// URL to redirect user to for Miro authorization
    ///
    /// # Example
    /// ```ignore
    /// let url = provider.build_authorization_url("random_state", "pkce_challenge")?;
    /// // Redirect user to: https://miro.com/oauth/authorize?client_id=...&response_type=code&...
    /// ```
    pub fn build_authorization_url(
        &self,
        state: &str,
        pkce_challenge: &str,
    ) -> Result<Url, MiroOAuthError> {
        let mut url = Url::parse(MIRO_AUTH_ENDPOINT)?;

        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", &MIRO_SCOPES.join(" "))
            .append_pair("state", state)
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256");

        Ok(url)
    }

    /// Exchange authorization code for access token
    ///
    /// # Arguments
    /// * `code` - Authorization code from Miro callback
    /// * `pkce_verifier` - PKCE code verifier (proves we generated the challenge)
    ///
    /// # Returns
    /// `CookieData` containing access token, refresh token, and user info
    ///
    /// # Errors
    /// Returns error if:
    /// - HTTP request fails
    /// - Miro returns OAuth error
    /// - Token response is invalid
    pub async fn exchange_code_for_token(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<CookieData, MiroOAuthError> {
        #[derive(Serialize)]
        struct TokenRequest<'a> {
            grant_type: &'a str,
            code: &'a str,
            redirect_uri: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
            code_verifier: &'a str,
        }

        let request_body = TokenRequest {
            grant_type: "authorization_code",
            code,
            redirect_uri: &self.redirect_uri,
            client_id: &self.client_id,
            client_secret: &self.client_secret,
            code_verifier: pkce_verifier,
        };

        let response = self
            .http_client
            .post(MIRO_TOKEN_ENDPOINT)
            .form(&request_body)
            .send()
            .await?;

        self.parse_token_response(response).await
    }

    /// Refresh access token using refresh token
    ///
    /// # Arguments
    /// * `refresh_token` - Refresh token from previous authorization
    ///
    /// # Returns
    /// New `CookieData` with refreshed access token
    ///
    /// # Errors
    /// Returns error if:
    /// - HTTP request fails
    /// - Miro returns OAuth error
    /// - Refresh token is invalid or expired
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<CookieData, MiroOAuthError> {
        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            grant_type: &'a str,
            refresh_token: &'a str,
            client_id: &'a str,
            client_secret: &'a str,
        }

        let request_body = RefreshRequest {
            grant_type: "refresh_token",
            refresh_token,
            client_id: &self.client_id,
            client_secret: &self.client_secret,
        };

        let response = self
            .http_client
            .post(MIRO_TOKEN_ENDPOINT)
            .form(&request_body)
            .send()
            .await?;

        self.parse_token_response(response).await
    }

    /// Parse token response from Miro and convert to CookieData
    ///
    /// # Arguments
    /// * `response` - HTTP response from Miro token endpoint
    ///
    /// # Returns
    /// `CookieData` with tokens and expiration
    ///
    /// # Errors
    /// Returns error if response indicates OAuth error or is malformed
    async fn parse_token_response(
        &self,
        response: reqwest::Response,
    ) -> Result<CookieData, MiroOAuthError> {
        let status = response.status();

        if !status.is_success() {
            // Try to parse OAuth error response
            #[derive(serde::Deserialize)]
            struct ErrorResponse {
                error: String,
                error_description: Option<String>,
            }

            let error_response: ErrorResponse = response.json().await?;
            return Err(MiroOAuthError::OAuthError {
                error: error_response.error,
                error_description: error_response
                    .error_description
                    .unwrap_or_else(|| "No description provided".to_string()),
            });
        }

        let token_response: TokenResponse = response.json().await?;

        // Calculate expiration time
        let expires_at = Utc::now() + Duration::seconds(token_response.expires_in as i64);

        // Extract user info (Miro includes this in token response)
        let user_info = if let Some(user) = token_response.user {
            UserInfo::from(user)
        } else {
            // If user info not in token response, use placeholder
            // In production, you might fetch user info from Miro API
            UserInfo {
                user_id: "unknown".to_string(),
                email: None,
                name: None,
            }
        };

        // Refresh token should be present in initial authorization, might be missing in refresh
        let refresh_token = token_response
            .refresh_token
            .ok_or_else(|| MiroOAuthError::MissingField("refresh_token".to_string()))?;

        Ok(CookieData {
            access_token: token_response.access_token,
            refresh_token,
            expires_at,
            user_info,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_provider() -> MiroOAuthProvider {
        MiroOAuthProvider::new(
            "test_client_id".to_string(),
            "test_client_secret".to_string(),
            "http://localhost:3000/oauth/callback".to_string(),
        )
    }

    #[test]
    fn test_build_authorization_url() {
        let provider = get_test_provider();
        let url = provider
            .build_authorization_url("test_state", "test_challenge")
            .unwrap();

        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("miro.com"));
        assert_eq!(url.path(), "/oauth/authorize");

        let params: std::collections::HashMap<_, _> =
            url.query_pairs().into_owned().collect();

        assert_eq!(params.get("client_id"), Some(&"test_client_id".to_string()));
        assert_eq!(params.get("response_type"), Some(&"code".to_string()));
        assert_eq!(params.get("state"), Some(&"test_state".to_string()));
        assert_eq!(params.get("code_challenge"), Some(&"test_challenge".to_string()));
        assert_eq!(params.get("code_challenge_method"), Some(&"S256".to_string()));
        assert_eq!(
            params.get("redirect_uri"),
            Some(&"http://localhost:3000/oauth/callback".to_string())
        );

        // Verify scopes
        let scopes = params.get("scope").unwrap();
        assert!(scopes.contains("boards:read"));
        assert!(scopes.contains("boards:write"));
    }
}
