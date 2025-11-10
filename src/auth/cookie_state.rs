use cookie::{Cookie, CookieJar, Key};
use oauth2::{CsrfToken, PkceCodeVerifier};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Error types for cookie-based OAuth state management
#[derive(Error, Debug)]
pub enum CookieStateError {
    #[error("Failed to serialize state: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize state: {0}")]
    DeserializationError(String),

    #[error("State cookie not found")]
    CookieNotFound,

    #[error("State cookie expired (max age: 10 minutes)")]
    CookieExpired,

    #[error("CSRF token validation failed: expected {expected}, got {actual}")]
    CsrfValidationFailed { expected: String, actual: String },

    #[error("Cookie tampering detected")]
    TamperingDetected,
}

/// OAuth state stored in encrypted cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthCookieState {
    /// CSRF token for state validation
    pub csrf_token: String,
    /// PKCE code verifier
    pub pkce_verifier: String,
    /// Creation timestamp (seconds since UNIX epoch)
    pub created_at: u64,
}

impl OAuthCookieState {
    /// Create new OAuth state
    pub fn new(csrf_token: CsrfToken, pkce_verifier: PkceCodeVerifier) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            csrf_token: csrf_token.secret().to_string(),
            pkce_verifier: pkce_verifier.secret().to_string(),
            created_at,
        }
    }

    /// Check if state has expired (10 minutes)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age = now.saturating_sub(self.created_at);
        age > 600 // 10 minutes in seconds
    }

    /// Validate CSRF token
    pub fn validate_csrf(&self, expected: &str) -> Result<(), CookieStateError> {
        if self.csrf_token != expected {
            return Err(CookieStateError::CsrfValidationFailed {
                expected: expected.to_string(),
                actual: self.csrf_token.clone(),
            });
        }
        Ok(())
    }
}

/// Manages encrypted cookie-based OAuth state
#[derive(Clone)]
pub struct CookieStateManager {
    /// Cookie encryption key (derived from config)
    key: Key,
    /// Cookie name
    cookie_name: String,
}

impl CookieStateManager {
    /// Cookie name for OAuth state
    const DEFAULT_COOKIE_NAME: &'static str = "miro_oauth_state";

    /// Create new cookie state manager from encryption key
    /// Derives a 64-byte key from the 32-byte encryption key using key derivation
    pub fn from_config(encryption_key: [u8; 32]) -> Self {
        // Cookie crate requires 64-byte key, so we derive it from our 32-byte key
        // Using HKDF-like expansion: repeat the key twice (simple but effective for this use case)
        let mut full_key = [0u8; 64];
        full_key[0..32].copy_from_slice(&encryption_key);
        full_key[32..64].copy_from_slice(&encryption_key);

        Self {
            key: Key::from(&full_key),
            cookie_name: Self::DEFAULT_COOKIE_NAME.to_string(),
        }
    }

    /// Create encrypted cookie containing OAuth state
    pub fn create_cookie(&self, state: OAuthCookieState) -> Result<Cookie<'static>, CookieStateError> {
        // Serialize state to JSON
        let json = serde_json::to_string(&state)
            .map_err(|e| CookieStateError::SerializationError(e.to_string()))?;

        // Create cookie with security attributes
        let mut cookie = Cookie::new(self.cookie_name.clone(), json);
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(cookie::SameSite::Lax);
        cookie.set_path("/");
        cookie.set_max_age(cookie::time::Duration::seconds(600)); // 10 minutes

        // Encrypt cookie using private jar
        let mut jar = CookieJar::new();
        jar.private_mut(&self.key).add(cookie);

        // Retrieve encrypted cookie
        jar.get(&self.cookie_name)
            .ok_or(CookieStateError::SerializationError(
                "Failed to retrieve cookie after encryption".to_string(),
            ))
            .map(|c| c.clone())
    }

    /// Retrieve and validate OAuth state from encrypted cookie
    pub fn retrieve_and_validate(
        &self,
        encrypted_cookie_value: &str,
        expected_csrf: &str,
    ) -> Result<OAuthCookieState, CookieStateError> {
        // Reconstruct cookie from value (make it 'static owned)
        let cookie = Cookie::new(self.cookie_name.clone(), encrypted_cookie_value.to_string());

        // Decrypt cookie
        let mut jar = CookieJar::new();
        jar.add(cookie);

        let private_jar = jar.private(&self.key);
        let decrypted = private_jar
            .get(&self.cookie_name)
            .ok_or(CookieStateError::TamperingDetected)?;

        // Deserialize state (clone the value to avoid lifetime issues)
        let json_value = decrypted.value().to_string();
        let state: OAuthCookieState = serde_json::from_str(&json_value)
            .map_err(|e| CookieStateError::DeserializationError(e.to_string()))?;

        // Validate expiry
        if state.is_expired() {
            return Err(CookieStateError::CookieExpired);
        }

        // Validate CSRF token
        state.validate_csrf(expected_csrf)?;

        Ok(state)
    }

    /// Create deletion cookie (expires immediately)
    pub fn deletion_cookie(&self) -> Cookie<'static> {
        let mut cookie = Cookie::new(self.cookie_name.clone(), "");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(cookie::SameSite::Lax);
        cookie.set_path("/");
        cookie.set_max_age(cookie::time::Duration::seconds(0)); // Immediate expiry
        cookie
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_key() -> [u8; 32] {
        [0u8; 32]
    }

    fn get_test_state() -> OAuthCookieState {
        OAuthCookieState::new(
            CsrfToken::new("test_csrf_token".to_string()),
            PkceCodeVerifier::new("test_pkce_verifier".to_string()),
        )
    }

    #[test]
    fn test_oauth_cookie_state_creation() {
        let csrf = CsrfToken::new("test_token".to_string());
        let verifier = PkceCodeVerifier::new("test_verifier".to_string());

        let state = OAuthCookieState::new(csrf, verifier);

        assert_eq!(state.csrf_token, "test_token");
        assert_eq!(state.pkce_verifier, "test_verifier");
        assert!(state.created_at > 0);
    }

    #[test]
    fn test_state_not_expired_immediately() {
        let state = get_test_state();
        assert!(!state.is_expired());
    }

    #[test]
    fn test_state_expiry_detection() {
        let mut state = get_test_state();
        // Set created_at to 11 minutes ago
        state.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 660;

        assert!(state.is_expired());
    }

    #[test]
    fn test_csrf_validation_success() {
        let state = get_test_state();
        let result = state.validate_csrf(&state.csrf_token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_csrf_validation_failure() {
        let state = get_test_state();
        let result = state.validate_csrf("wrong_token");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CookieStateError::CsrfValidationFailed { .. }
        ));
    }

    #[test]
    fn test_cookie_encryption_roundtrip() {
        let manager = CookieStateManager::from_config(get_test_key());
        let state = get_test_state();
        let csrf_token = state.csrf_token.clone();

        // Create encrypted cookie
        let cookie = manager.create_cookie(state.clone()).unwrap();

        // Verify cookie attributes
        assert_eq!(cookie.name(), CookieStateManager::DEFAULT_COOKIE_NAME);
        assert!(cookie.http_only().unwrap());
        assert!(cookie.secure().unwrap());
        assert_eq!(cookie.same_site(), Some(cookie::SameSite::Lax));
        assert_eq!(cookie.path(), Some("/"));

        // Retrieve and validate
        let retrieved = manager
            .retrieve_and_validate(cookie.value(), &csrf_token)
            .unwrap();

        assert_eq!(retrieved.csrf_token, state.csrf_token);
        assert_eq!(retrieved.pkce_verifier, state.pkce_verifier);
        assert_eq!(retrieved.created_at, state.created_at);
    }

    #[test]
    fn test_tampering_detection() {
        let manager = CookieStateManager::from_config(get_test_key());
        let state = get_test_state();

        // Try to retrieve with tampered cookie value
        let result = manager.retrieve_and_validate("tampered_data", &state.csrf_token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CookieStateError::TamperingDetected));
    }

    #[test]
    fn test_expired_cookie_rejection() {
        let manager = CookieStateManager::from_config(get_test_key());
        let mut state = get_test_state();

        // Set created_at to 11 minutes ago
        state.created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 660;

        let csrf_token = state.csrf_token.clone();
        let cookie = manager.create_cookie(state).unwrap();

        let result = manager.retrieve_and_validate(cookie.value(), &csrf_token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CookieStateError::CookieExpired));
    }

    #[test]
    fn test_deletion_cookie() {
        let manager = CookieStateManager::from_config(get_test_key());
        let cookie = manager.deletion_cookie();

        assert_eq!(cookie.name(), CookieStateManager::DEFAULT_COOKIE_NAME);
        assert_eq!(cookie.value(), "");
        assert!(cookie.http_only().unwrap());
        assert!(cookie.secure().unwrap());
        assert_eq!(cookie.max_age(), Some(cookie::time::Duration::seconds(0)));
    }

    #[test]
    fn test_different_keys_prevent_decryption() {
        let manager1 = CookieStateManager::from_config([1u8; 32]);
        let manager2 = CookieStateManager::from_config([2u8; 32]);

        let state = get_test_state();
        let csrf_token = state.csrf_token.clone();
        let cookie = manager1.create_cookie(state).unwrap();

        // Try to decrypt with different key
        let result = manager2.retrieve_and_validate(cookie.value(), &csrf_token);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CookieStateError::TamperingDetected));
    }
}
