use cookie::{Cookie, CookieJar, Key};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Error types for cookie-based token storage
#[derive(Error, Debug)]
pub enum CookieTokenError {
    #[error("Failed to serialize token: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize token: {0}")]
    DeserializationError(String),

    #[error("Token cookie not found")]
    CookieNotFound,

    #[error("Access token expired")]
    TokenExpired,

    #[error("Cookie tampering detected")]
    TamperingDetected,
}

/// OAuth tokens stored in encrypted cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenCookie {
    /// Access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: String,
    /// Expiry timestamp (seconds since UNIX epoch)
    pub expires_at: u64,
}

impl OAuthTokenCookie {
    /// Create new token cookie from OAuth response
    pub fn new(access_token: String, refresh_token: String, expires_in_seconds: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            access_token,
            refresh_token,
            expires_at: now + expires_in_seconds,
        }
    }

    /// Check if access token has expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now >= self.expires_at
    }

    /// Get seconds until expiry
    pub fn seconds_until_expiry(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        (self.expires_at as i64) - (now as i64)
    }
}

/// Manages encrypted cookie-based OAuth token storage
#[derive(Clone)]
pub struct CookieTokenManager {
    /// Cookie encryption key
    key: Key,
    /// Cookie name
    cookie_name: String,
}

impl CookieTokenManager {
    /// Cookie name for OAuth tokens
    const DEFAULT_COOKIE_NAME: &'static str = "miro_access_token";

    /// Create new cookie token manager from encryption key
    pub fn from_config(encryption_key: [u8; 32]) -> Self {
        // Cookie crate requires 64-byte key
        let mut full_key = [0u8; 64];
        full_key[0..32].copy_from_slice(&encryption_key);
        full_key[32..64].copy_from_slice(&encryption_key);

        Self {
            key: Key::from(&full_key),
            cookie_name: Self::DEFAULT_COOKIE_NAME.to_string(),
        }
    }

    /// Create encrypted cookie containing OAuth tokens
    pub fn create_cookie(
        &self,
        tokens: OAuthTokenCookie,
    ) -> Result<Cookie<'static>, CookieTokenError> {
        // Serialize tokens to JSON
        let json = serde_json::to_string(&tokens)
            .map_err(|e| CookieTokenError::SerializationError(e.to_string()))?;

        // Create cookie with security attributes
        let mut cookie = Cookie::new(self.cookie_name.clone(), json);
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(cookie::SameSite::Strict); // Strict for tokens
        cookie.set_path("/");

        // Set cookie max-age to token expiry
        let max_age = tokens.seconds_until_expiry();
        if max_age > 0 {
            cookie.set_max_age(cookie::time::Duration::seconds(max_age));
        }

        // Encrypt cookie
        let mut jar = CookieJar::new();
        jar.private_mut(&self.key).add(cookie);

        // Retrieve encrypted cookie
        jar.get(&self.cookie_name)
            .ok_or(CookieTokenError::SerializationError(
                "Failed to retrieve cookie after encryption".to_string(),
            ))
            .map(|c| c.clone())
    }

    /// Retrieve OAuth tokens from encrypted cookie
    pub fn retrieve_tokens(
        &self,
        encrypted_cookie_value: &str,
    ) -> Result<OAuthTokenCookie, CookieTokenError> {
        // Reconstruct cookie
        let cookie = Cookie::new(self.cookie_name.clone(), encrypted_cookie_value.to_string());

        // Decrypt cookie
        let mut jar = CookieJar::new();
        jar.add(cookie);

        let private_jar = jar.private(&self.key);
        let decrypted = private_jar
            .get(&self.cookie_name)
            .ok_or(CookieTokenError::TamperingDetected)?;

        // Deserialize tokens
        let json_value = decrypted.value().to_string();
        let tokens: OAuthTokenCookie = serde_json::from_str(&json_value)
            .map_err(|e| CookieTokenError::DeserializationError(e.to_string()))?;

        Ok(tokens)
    }

    /// Create deletion cookie (expires immediately)
    pub fn deletion_cookie(&self) -> Cookie<'static> {
        let mut cookie = Cookie::new(self.cookie_name.clone(), "");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(cookie::SameSite::Strict);
        cookie.set_path("/");
        cookie.set_max_age(cookie::time::Duration::seconds(0));
        cookie
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_key() -> [u8; 32] {
        [0u8; 32]
    }

    fn get_test_tokens() -> OAuthTokenCookie {
        OAuthTokenCookie::new(
            "test_access_token".to_string(),
            "test_refresh_token".to_string(),
            3600, // 1 hour
        )
    }

    #[test]
    fn test_token_cookie_creation() {
        let tokens = OAuthTokenCookie::new(
            "access".to_string(),
            "refresh".to_string(),
            3600,
        );

        assert_eq!(tokens.access_token, "access");
        assert_eq!(tokens.refresh_token, "refresh");
        assert!(tokens.expires_at > 0);
    }

    #[test]
    fn test_token_not_expired_immediately() {
        let tokens = get_test_tokens();
        assert!(!tokens.is_expired());
    }

    #[test]
    fn test_token_expiry_detection() {
        let mut tokens = get_test_tokens();
        // Set expires_at to past
        tokens.expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 1;

        assert!(tokens.is_expired());
    }

    #[test]
    fn test_seconds_until_expiry() {
        let tokens = get_test_tokens();
        let seconds = tokens.seconds_until_expiry();
        assert!(seconds > 3500 && seconds <= 3600);
    }

    #[test]
    fn test_cookie_encryption_roundtrip() {
        let manager = CookieTokenManager::from_config(get_test_key());
        let tokens = get_test_tokens();

        // Create encrypted cookie
        let cookie = manager.create_cookie(tokens.clone()).unwrap();

        // Verify cookie attributes
        assert_eq!(cookie.name(), CookieTokenManager::DEFAULT_COOKIE_NAME);
        assert!(cookie.http_only().unwrap());
        assert!(cookie.secure().unwrap());
        assert_eq!(cookie.same_site(), Some(cookie::SameSite::Strict));
        assert_eq!(cookie.path(), Some("/"));

        // Retrieve tokens
        let retrieved = manager.retrieve_tokens(cookie.value()).unwrap();

        assert_eq!(retrieved.access_token, tokens.access_token);
        assert_eq!(retrieved.refresh_token, tokens.refresh_token);
        assert_eq!(retrieved.expires_at, tokens.expires_at);
    }

    #[test]
    fn test_tampering_detection() {
        let manager = CookieTokenManager::from_config(get_test_key());

        // Try to retrieve with tampered cookie value
        let result = manager.retrieve_tokens("tampered_data");

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CookieTokenError::TamperingDetected
        ));
    }

    #[test]
    fn test_deletion_cookie() {
        let manager = CookieTokenManager::from_config(get_test_key());
        let cookie = manager.deletion_cookie();

        assert_eq!(cookie.name(), CookieTokenManager::DEFAULT_COOKIE_NAME);
        assert_eq!(cookie.value(), "");
        assert!(cookie.http_only().unwrap());
        assert!(cookie.secure().unwrap());
        assert_eq!(
            cookie.max_age(),
            Some(cookie::time::Duration::seconds(0))
        );
    }

    #[test]
    fn test_different_keys_prevent_decryption() {
        let manager1 = CookieTokenManager::from_config([1u8; 32]);
        let manager2 = CookieTokenManager::from_config([2u8; 32]);

        let tokens = get_test_tokens();
        let cookie = manager1.create_cookie(tokens).unwrap();

        // Try to decrypt with different key
        let result = manager2.retrieve_tokens(cookie.value());

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CookieTokenError::TamperingDetected
        ));
    }
}
