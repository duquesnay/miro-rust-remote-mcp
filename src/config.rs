use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid encryption key: {0}")]
    InvalidEncryptionKey(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
}

/// Configuration for Miro MCP Server
#[derive(Debug, Clone)]
pub struct Config {
    /// Miro OAuth2 client ID
    pub client_id: String,

    /// Miro OAuth2 client secret
    pub client_secret: String,

    /// OAuth2 redirect URI
    pub redirect_uri: String,

    /// Encryption key for token storage (32 bytes)
    pub encryption_key: [u8; 32],

    /// MCP server port
    pub port: u16,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        let client_id = env::var("MIRO_CLIENT_ID")
            .map_err(|_| ConfigError::MissingEnvVar("MIRO_CLIENT_ID".to_string()))?;

        let client_secret = env::var("MIRO_CLIENT_SECRET")
            .map_err(|_| ConfigError::MissingEnvVar("MIRO_CLIENT_SECRET".to_string()))?;

        let redirect_uri = env::var("MIRO_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:3000/oauth/callback".to_string());

        // Validate redirect URI
        let _ = url::Url::parse(&redirect_uri)?;

        let encryption_key_hex = env::var("TOKEN_ENCRYPTION_KEY")
            .map_err(|_| ConfigError::MissingEnvVar("TOKEN_ENCRYPTION_KEY".to_string()))?;

        let encryption_key = Self::parse_encryption_key(&encryption_key_hex)?;

        let port = env::var("MCP_SERVER_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .unwrap_or(3000);

        Ok(Config {
            client_id,
            client_secret,
            redirect_uri,
            encryption_key,
            port,
        })
    }

    /// Parse encryption key from hex string (must be 32 bytes)
    fn parse_encryption_key(hex_str: &str) -> Result<[u8; 32], ConfigError> {
        let bytes = hex::decode(hex_str.trim())
            .map_err(|e| ConfigError::InvalidEncryptionKey(format!("Invalid hex: {}", e)))?;

        if bytes.len() != 32 {
            return Err(ConfigError::InvalidEncryptionKey(format!(
                "Expected 32 bytes, got {}",
                bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_encryption_key_valid() {
        let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = Config::parse_encryption_key(hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_parse_encryption_key_invalid_length() {
        let hex = "0123456789abcdef";
        let result = Config::parse_encryption_key(hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_encryption_key_invalid_hex() {
        let hex = "not_valid_hex_string_here_xxxxxx";
        let result = Config::parse_encryption_key(hex);
        assert!(result.is_err());
    }
}
