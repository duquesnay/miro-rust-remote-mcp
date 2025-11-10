/// OAuth Session Management with Encrypted Cookies and Authorization Code Store
///
/// Implements:
/// - Encrypted cookie storage for OAuth state (PKCE + client info)
/// - Temporary authorization code store with TTL
/// - PKCE validation

use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

const NONCE_LEN: usize = 12;
const AUTH_CODE_TTL_SECS: u64 = 600; // 10 minutes

/// OAuth authorization state stored in encrypted cookie
#[derive(Serialize, Deserialize, Clone)]
pub struct OAuthState {
    pub client_id: String,
    pub redirect_uri: String,
    pub state: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: Option<String>,
    pub timestamp: u64,
}

/// Authorization code with PKCE challenge for validation
#[derive(Clone)]
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scope: Option<String>,
    pub created_at: SystemTime,
}

/// Simple nonce sequence that increments
struct CounterNonceSequence(u32);

impl CounterNonceSequence {
    fn new() -> Self {
        Self(0)
    }
}

impl NonceSequence for CounterNonceSequence {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        let mut nonce_bytes = [0u8; NONCE_LEN];
        nonce_bytes[..4].copy_from_slice(&self.0.to_le_bytes());
        self.0 = self.0.wrapping_add(1);
        Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

/// Cookie encryption/decryption using AES-256-GCM
pub struct CookieCipher {
    key: [u8; 32],
    rng: SystemRandom,
}

impl CookieCipher {
    /// Create new cipher from encryption key
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            rng: SystemRandom::new(),
        }
    }

    /// Encrypt OAuth state into base64 string
    pub fn encrypt(&self, state: &OAuthState) -> Result<String, String> {
        // Serialize to JSON
        let plaintext = serde_json::to_vec(state)
            .map_err(|e| format!("Failed to serialize state: {}", e))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_LEN];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|_| "Failed to generate nonce".to_string())?;

        // Create sealing key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.key)
            .map_err(|_| "Failed to create encryption key".to_string())?;
        let mut sealing_key = SealingKey::new(unbound_key, CounterNonceSequence::new());

        // Encrypt
        let mut in_out = plaintext.clone();
        let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| "Invalid nonce".to_string())?;

        sealing_key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| "Encryption failed".to_string())?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&in_out);

        // Encode to base64
        Ok(base64::encode(&result))
    }

    /// Decrypt OAuth state from base64 string
    pub fn decrypt(&self, encrypted: &str) -> Result<OAuthState, String> {
        // Decode from base64
        let data = base64::decode(encrypted)
            .map_err(|e| format!("Invalid base64: {}", e))?;

        if data.len() < NONCE_LEN {
            return Err("Invalid encrypted data length".to_string());
        }

        // Extract nonce and ciphertext
        let (nonce_bytes, mut ciphertext) = data.split_at(NONCE_LEN);
        let mut ciphertext = ciphertext.to_vec();

        // Create opening key
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.key)
            .map_err(|_| "Failed to create decryption key".to_string())?;
        let mut opening_key = OpeningKey::new(unbound_key, CounterNonceSequence::new());

        // Decrypt
        let nonce = Nonce::try_assume_unique_for_key(nonce_bytes)
            .map_err(|_| "Invalid nonce".to_string())?;

        let plaintext = opening_key.open_in_place(nonce, Aad::empty(), &mut ciphertext)
            .map_err(|_| "Decryption failed".to_string())?;

        // Deserialize JSON
        serde_json::from_slice(plaintext)
            .map_err(|e| format!("Failed to deserialize state: {}", e))
    }
}

/// Store for authorization codes with automatic expiration
pub struct AuthCodeStore {
    codes: Arc<RwLock<HashMap<String, AuthorizationCode>>>,
}

impl AuthCodeStore {
    pub fn new() -> Self {
        let store = Self {
            codes: Arc::new(RwLock::new(HashMap::new())),
        };

        // Start background cleanup task
        let codes = Arc::clone(&store.codes);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let mut codes = codes.write().await;
                let now = SystemTime::now();
                codes.retain(|_, auth_code| {
                    now.duration_since(auth_code.created_at)
                        .map(|d| d.as_secs() < AUTH_CODE_TTL_SECS)
                        .unwrap_or(false)
                });
            }
        });

        store
    }

    /// Store an authorization code
    pub async fn store(&self, code: AuthorizationCode) -> Result<(), String> {
        let mut codes = self.codes.write().await;
        codes.insert(code.code.clone(), code);
        Ok(())
    }

    /// Consume an authorization code (one-time use)
    pub async fn consume(&self, code: &str) -> Result<AuthorizationCode, String> {
        let mut codes = self.codes.write().await;
        codes.remove(code)
            .ok_or_else(|| "Invalid or expired authorization code".to_string())
    }

    /// Validate PKCE challenge
    pub fn validate_pkce(
        code_verifier: &str,
        code_challenge: &str,
        method: &str,
    ) -> Result<(), String> {
        use sha2::{Sha256, Digest};

        match method {
            "S256" => {
                let mut hasher = Sha256::new();
                hasher.update(code_verifier.as_bytes());
                let hash = hasher.finalize();
                let computed_challenge = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD);

                if computed_challenge == code_challenge {
                    Ok(())
                } else {
                    Err("PKCE validation failed".to_string())
                }
            }
            "plain" => {
                if code_verifier == code_challenge {
                    Ok(())
                } else {
                    Err("PKCE validation failed".to_string())
                }
            }
            _ => Err(format!("Unsupported code challenge method: {}", method)),
        }
    }
}

impl Default for AuthCodeStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_cipher_roundtrip() {
        let key = [42u8; 32];
        let cipher = CookieCipher::new(key);

        let state = OAuthState {
            client_id: "test_client".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
            state: "random_state".to_string(),
            code_challenge: "challenge123".to_string(),
            code_challenge_method: "S256".to_string(),
            scope: Some("read write".to_string()),
            timestamp: 1234567890,
        };

        let encrypted = cipher.encrypt(&state).unwrap();
        let decrypted = cipher.decrypt(&encrypted).unwrap();

        assert_eq!(state.client_id, decrypted.client_id);
        assert_eq!(state.redirect_uri, decrypted.redirect_uri);
        assert_eq!(state.code_challenge, decrypted.code_challenge);
    }

    #[test]
    fn test_pkce_validation_s256() {
        let verifier = "test_verifier_with_high_entropy_12345678";

        // Compute challenge
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = base64::encode_config(&hash, base64::URL_SAFE_NO_PAD);

        // Validate
        assert!(AuthCodeStore::validate_pkce(verifier, &challenge, "S256").is_ok());
        assert!(AuthCodeStore::validate_pkce("wrong_verifier", &challenge, "S256").is_err());
    }

    #[tokio::test]
    async fn test_auth_code_store() {
        let store = AuthCodeStore::new();

        let auth_code = AuthorizationCode {
            code: "test_code_123".to_string(),
            client_id: "client_id".to_string(),
            redirect_uri: "http://localhost/callback".to_string(),
            code_challenge: "challenge".to_string(),
            code_challenge_method: "S256".to_string(),
            scope: None,
            created_at: SystemTime::now(),
        };

        // Store code
        store.store(auth_code.clone()).await.unwrap();

        // Consume code (should work once)
        let consumed = store.consume("test_code_123").await.unwrap();
        assert_eq!(consumed.client_id, "client_id");

        // Second consumption should fail
        assert!(store.consume("test_code_123").await.is_err());
    }
}
