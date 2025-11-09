use super::types::{AuthError, TokenSet};
use ring::aead::{
    Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM,
};
use ring::error::Unspecified;
use serde_json;
use std::fs;
use std::path::PathBuf;

/// Nonce sequence for AES-256-GCM
struct CounterNonceSequence {
    counter: u64,
}

impl CounterNonceSequence {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

impl NonceSequence for CounterNonceSequence {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..12].copy_from_slice(&self.counter.to_be_bytes());
        self.counter = self.counter.wrapping_add(1);
        Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

/// Token store with encrypted storage
pub struct TokenStore {
    encryption_key: [u8; 32],
    storage_path: PathBuf,
}

impl TokenStore {
    /// Create a new token store
    pub fn new(encryption_key: [u8; 32]) -> Result<Self, AuthError> {
        let storage_path = Self::get_storage_path()?;

        // Ensure directory exists
        if let Some(parent) = storage_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AuthError::TokenStorageError(format!("Failed to create storage directory: {}", e))
            })?;
        }

        Ok(Self {
            encryption_key,
            storage_path,
        })
    }

    /// Get the storage path for tokens
    fn get_storage_path() -> Result<PathBuf, AuthError> {
        let home = std::env::var("HOME").map_err(|_| {
            AuthError::TokenStorageError("HOME environment variable not set".to_string())
        })?;

        let mut path = PathBuf::from(home);
        path.push(".miro-mcp");
        path.push("tokens.enc");

        Ok(path)
    }

    /// Save encrypted tokens to disk
    pub fn save(&self, tokens: &TokenSet) -> Result<(), AuthError> {
        // Serialize tokens to JSON
        let json = serde_json::to_vec(tokens)?;

        // Encrypt the data
        let encrypted = self.encrypt(&json)?;

        // Write to disk
        fs::write(&self.storage_path, encrypted)
            .map_err(|e| AuthError::TokenStorageError(format!("Failed to write tokens: {}", e)))?;

        Ok(())
    }

    /// Load and decrypt tokens from disk
    pub fn load(&self) -> Result<TokenSet, AuthError> {
        // Read encrypted data from disk
        let encrypted = fs::read(&self.storage_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AuthError::NoToken
            } else {
                AuthError::TokenStorageError(format!("Failed to read tokens: {}", e))
            }
        })?;

        // Decrypt the data
        let decrypted = self.decrypt(&encrypted)?;

        // Deserialize from JSON
        let tokens: TokenSet = serde_json::from_slice(&decrypted)?;

        Ok(tokens)
    }

    /// Check if tokens exist and are not expired
    pub fn has_valid_token(&self) -> bool {
        match self.load() {
            Ok(tokens) => !tokens.is_expired(),
            Err(_) => false,
        }
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, AuthError> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key).map_err(|_| {
            AuthError::EncryptionError("Failed to create encryption key".to_string())
        })?;

        let nonce_sequence = CounterNonceSequence::new();
        let mut sealing_key = SealingKey::new(unbound_key, nonce_sequence);

        let mut encrypted_data = data.to_vec();
        sealing_key
            .seal_in_place_append_tag(Aad::empty(), &mut encrypted_data)
            .map_err(|_| AuthError::EncryptionError("Encryption failed".to_string()))?;

        Ok(encrypted_data)
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, AuthError> {
        let unbound_key = UnboundKey::new(&AES_256_GCM, &self.encryption_key).map_err(|_| {
            AuthError::EncryptionError("Failed to create decryption key".to_string())
        })?;

        let nonce_sequence = CounterNonceSequence::new();
        let mut opening_key = OpeningKey::new(unbound_key, nonce_sequence);

        let mut decrypted_data = data.to_vec();
        let decrypted = opening_key
            .open_in_place(Aad::empty(), &mut decrypted_data)
            .map_err(|_| AuthError::EncryptionError("Decryption failed".to_string()))?;

        Ok(decrypted.to_vec())
    }

    /// Delete stored tokens
    pub fn clear(&self) -> Result<(), AuthError> {
        if self.storage_path.exists() {
            fs::remove_file(&self.storage_path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_key() -> [u8; 32] {
        [0u8; 32] // Test key - all zeros
    }

    #[test]
    fn test_token_set_expiry() {
        let tokens = TokenSet::new(
            "access_token".to_string(),
            Some("refresh_token".to_string()),
            3600, // 1 hour
        );

        assert!(!tokens.is_expired());
        assert!(tokens.expires_in() > 0);
    }

    #[test]
    fn test_token_set_expired() {
        let tokens = TokenSet::new(
            "access_token".to_string(),
            Some("refresh_token".to_string()),
            0, // Already expired
        );

        assert!(tokens.is_expired());
        assert!(tokens.expires_in() <= 0);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let store = TokenStore {
            encryption_key: get_test_key(),
            storage_path: PathBuf::from("/tmp/test_tokens.enc"),
        };

        let data = b"test data";
        let encrypted = store.encrypt(data).expect("Encryption failed");
        let decrypted = store.decrypt(&encrypted).expect("Decryption failed");

        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_token_serialization() {
        let tokens = TokenSet::new(
            "access_123".to_string(),
            Some("refresh_456".to_string()),
            3600,
        );

        let json = serde_json::to_vec(&tokens).expect("Serialization failed");
        let deserialized: TokenSet = serde_json::from_slice(&json).expect("Deserialization failed");

        assert_eq!(tokens.access_token, deserialized.access_token);
        assert_eq!(tokens.refresh_token, deserialized.refresh_token);
        assert_eq!(tokens.expires_at, deserialized.expires_at);
    }
}
