#![cfg_attr(not(feature = "stdio-mcp"), allow(dead_code))]

#[cfg(feature = "stdio-mcp")]
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
#[cfg(feature = "stdio-mcp")]
use base64::{engine::general_purpose::STANDARD, Engine as _};
#[cfg(feature = "stdio-mcp")]
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Cookie encryption/decryption errors
#[derive(Error, Debug)]
pub enum CookieError {
    #[error("Failed to serialize cookie data: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Failed to encrypt cookie: {0}")]
    EncryptionError(String),

    #[error("Failed to decrypt cookie: {0}")]
    DecryptionError(String),

    #[error("Invalid cookie format: {0}")]
    InvalidFormat(String),

    #[error("Failed to decode base64: {0}")]
    Base64Error(#[from] base64::DecodeError),
}

/// Manager for encrypting and decrypting cookie data using AES-256-GCM
///
/// Cookie format: [12-byte nonce][ciphertext][16-byte auth tag]
/// All base64 encoded for safe transmission in HTTP headers
#[cfg(feature = "stdio-mcp")]
pub struct CookieManager {
    cipher: Aes256Gcm,
}

#[cfg(feature = "stdio-mcp")]
impl CookieManager {
    /// Create a new CookieManager with the provided encryption key
    ///
    /// # Arguments
    /// * `key` - 32-byte encryption key for AES-256-GCM
    ///
    /// # Security
    /// The key should be cryptographically random and kept secret
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
        Self { cipher }
    }

    /// Encrypt cookie data into a base64-encoded string
    ///
    /// # Arguments
    /// * `data` - Data to encrypt (must be serializable to JSON)
    ///
    /// # Returns
    /// Base64-encoded string: [nonce][ciphertext][auth_tag]
    ///
    /// # Security
    /// - Uses AES-256-GCM authenticated encryption
    /// - Random 12-byte nonce generated per encryption
    /// - 16-byte authentication tag prevents tampering
    pub fn encrypt<T: Serialize>(&self, data: &T) -> Result<String, CookieError> {
        // Serialize data to JSON
        let plaintext = serde_json::to_vec(data)?;

        // Generate random 12-byte nonce
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from(nonce_bytes);

        // Encrypt with AES-256-GCM (produces ciphertext + 16-byte auth tag)
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|e| CookieError::EncryptionError(e.to_string()))?;

        // Combine: nonce || ciphertext (which includes auth tag)
        let mut encrypted = Vec::with_capacity(12 + ciphertext.len());
        encrypted.extend_from_slice(&nonce_bytes);
        encrypted.extend_from_slice(&ciphertext);

        // Base64 encode for cookie transmission
        Ok(STANDARD.encode(encrypted))
    }

    /// Decrypt cookie data from a base64-encoded string
    ///
    /// # Arguments
    /// * `encrypted_b64` - Base64-encoded encrypted cookie data
    ///
    /// # Returns
    /// Decrypted and deserialized data
    ///
    /// # Errors
    /// Returns error if:
    /// - Base64 decoding fails
    /// - Cookie format is invalid (too short)
    /// - Decryption fails (wrong key or tampered data)
    /// - Deserialization fails (invalid JSON)
    pub fn decrypt<T: DeserializeOwned>(&self, encrypted_b64: &str) -> Result<T, CookieError> {
        // Base64 decode
        let encrypted = STANDARD.decode(encrypted_b64)?;

        // Validate minimum length: 12-byte nonce + 16-byte tag = 28 bytes minimum
        if encrypted.len() < 28 {
            return Err(CookieError::InvalidFormat(format!(
                "Cookie too short: {} bytes (minimum 28)",
                encrypted.len()
            )));
        }

        // Extract nonce (first 12 bytes)
        let nonce_bytes: [u8; 12] = encrypted[0..12].try_into()
            .map_err(|_| CookieError::InvalidFormat("Failed to extract nonce".to_string()))?;
        let nonce = Nonce::from(nonce_bytes);

        // Extract ciphertext + auth tag (remaining bytes)
        let ciphertext = &encrypted[12..];

        // Decrypt and verify auth tag
        let plaintext = self
            .cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| CookieError::DecryptionError(e.to_string()))?;

        // Deserialize JSON
        let data = serde_json::from_slice(&plaintext)?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        message: String,
        count: u32,
    }

    fn get_test_key() -> [u8; 32] {
        // Deterministic key for testing
        [42u8; 32]
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let manager = CookieManager::new(&get_test_key());
        let original = TestData {
            message: "Hello, World!".to_string(),
            count: 42,
        };

        let encrypted = manager.encrypt(&original).unwrap();
        assert!(!encrypted.is_empty());

        let decrypted: TestData = manager.decrypt(&encrypted).unwrap();
        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        // Same data should produce different ciphertexts due to random nonce
        let manager = CookieManager::new(&get_test_key());
        let data = TestData {
            message: "Test".to_string(),
            count: 1,
        };

        let encrypted1 = manager.encrypt(&data).unwrap();
        let encrypted2 = manager.encrypt(&data).unwrap();

        assert_ne!(encrypted1, encrypted2, "Nonce should make ciphertexts unique");
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let manager1 = CookieManager::new(&get_test_key());
        let data = TestData {
            message: "Secret".to_string(),
            count: 99,
        };

        let encrypted = manager1.encrypt(&data).unwrap();

        // Try to decrypt with different key
        let wrong_key = [0u8; 32];
        let manager2 = CookieManager::new(&wrong_key);
        let result: Result<TestData, _> = manager2.decrypt(&encrypted);

        assert!(result.is_err(), "Decryption should fail with wrong key");
    }

    #[test]
    fn test_decrypt_tampered_data_fails() {
        let manager = CookieManager::new(&get_test_key());
        let data = TestData {
            message: "Original".to_string(),
            count: 1,
        };

        let mut encrypted = manager.encrypt(&data).unwrap();

        // Tamper with the ciphertext (flip a bit in the middle)
        let bytes = STANDARD.decode(&encrypted).unwrap();
        let mut tampered = bytes.clone();
        tampered[20] ^= 0xFF; // Flip bits
        encrypted = STANDARD.encode(tampered);

        let result: Result<TestData, _> = manager.decrypt(&encrypted);
        assert!(result.is_err(), "Decryption should fail with tampered data");
    }

    #[test]
    fn test_decrypt_too_short_fails() {
        let manager = CookieManager::new(&get_test_key());
        let short_data = STANDARD.encode(b"tooshort");

        let result: Result<TestData, _> = manager.decrypt(&short_data);
        assert!(matches!(result, Err(CookieError::InvalidFormat(_))));
    }

    #[test]
    fn test_decrypt_invalid_base64_fails() {
        let manager = CookieManager::new(&get_test_key());
        let invalid_b64 = "not!valid@base64#";

        let result: Result<TestData, _> = manager.decrypt(invalid_b64);
        assert!(matches!(result, Err(CookieError::Base64Error(_))));
    }
}
