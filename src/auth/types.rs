use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Token set containing access token, refresh token, and expiry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp
}

impl TokenSet {
    /// Create a new token set
    pub fn new(access_token: String, refresh_token: Option<String>, expires_in: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            access_token,
            refresh_token,
            expires_at: now + expires_in,
        }
    }

    /// Check if the access token is expired (with 60s buffer)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        // Consider token expired if it expires within 60 seconds
        self.expires_at <= now + 60
    }

    /// Get time until expiry in seconds
    pub fn expires_in(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        (self.expires_at as i64) - (now as i64)
    }
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("OAuth2 error: {0}")]
    OAuth2Error(String),

    #[error("Token storage error: {0}")]
    TokenStorageError(String),

    #[error("Token encryption error: {0}")]
    EncryptionError(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("No token available")]
    NoToken,

    #[error("Invalid token format")]
    InvalidTokenFormat,

    #[error("CSRF validation failed")]
    CsrfValidationFailed,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl<RE, T> From<oauth2::RequestTokenError<RE, T>> for AuthError
where
    RE: std::error::Error + 'static,
    T: oauth2::ErrorResponse + 'static,
{
    fn from(err: oauth2::RequestTokenError<RE, T>) -> Self {
        AuthError::OAuth2Error(err.to_string())
    }
}
