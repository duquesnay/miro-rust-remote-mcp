use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found at {path}: {reason}")]
    FileNotFound { path: String, reason: String },

    #[error("Failed to parse configuration file: {0}")]
    ParseError(String),

    #[error("Invalid encryption key: {0}")]
    InvalidEncryptionKey(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Failed to read configuration file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::error::Error),
}

/// Configuration file format (for deserialization)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigFile {
    /// Miro OAuth2 client ID
    client_id: String,

    /// Miro OAuth2 client secret
    client_secret: String,

    /// OAuth2 redirect URI
    redirect_uri: String,

    /// Encryption key for token storage (32-byte hex string)
    encryption_key: String,

    /// MCP server port
    port: u16,
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
    /// Load configuration from file at ~/.config/mcp/miro-mcp-server/config.json
    pub fn from_file() -> Result<Self, ConfigError> {
        let config_path = Self::get_config_path()?;

        // Read and parse the configuration file
        let contents = fs::read_to_string(&config_path).map_err(|e| ConfigError::FileNotFound {
            path: config_path.display().to_string(),
            reason: format!(
                "{}. Create the config directory and file:\n\
                     mkdir -p ~/.config/mcp/miro-mcp-server\n\
                     cp config.example.json ~/.config/mcp/miro-mcp-server/config.json\n\
                     Then edit the file with your Miro OAuth2 credentials.",
                e
            ),
        })?;

        let config_file: ConfigFile = serde_json::from_str(&contents)?;

        // Validate redirect URI
        let _ = url::Url::parse(&config_file.redirect_uri)?;

        // Parse encryption key from hex
        let encryption_key = Self::parse_encryption_key(&config_file.encryption_key)?;

        Ok(Config {
            client_id: config_file.client_id,
            client_secret: config_file.client_secret,
            redirect_uri: config_file.redirect_uri,
            encryption_key,
            port: config_file.port,
        })
    }

    /// Get the configuration file path: ~/.config/mcp/miro-mcp-server/config.json
    fn get_config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::home_dir()
            .map(|home| home.join(".config/mcp/miro-mcp-server"))
            .ok_or_else(|| ConfigError::FileNotFound {
                path: "~/.config/mcp/miro-mcp-server/config.json".to_string(),
                reason: "Could not determine home directory".to_string(),
            })?;

        Ok(config_dir.join("config.json"))
    }

    /// Ensure configuration directory exists (creates if needed)
    pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::home_dir()
            .map(|home| home.join(".config/mcp/miro-mcp-server"))
            .ok_or_else(|| ConfigError::FileNotFound {
                path: "~/.config/mcp/miro-mcp-server".to_string(),
                reason: "Could not determine home directory".to_string(),
            })?;

        fs::create_dir_all(&config_dir)?;
        Ok(config_dir)
    }

    /// Load configuration from environment variables (legacy method, deprecated)
    #[deprecated(since = "0.2.0", note = "Use Config::from_file() instead")]
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_file()
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
