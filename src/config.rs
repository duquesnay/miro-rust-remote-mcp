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

    /// Miro OAuth2 client secret (optional for ADR-005 Resource Server)
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<String>,

    /// OAuth2 redirect URI
    redirect_uri: String,

    /// Encryption key for token storage (32-byte hex string, optional for ADR-005)
    #[serde(skip_serializing_if = "Option::is_none")]
    encryption_key: Option<String>,

    /// MCP server port
    port: u16,

    /// Base URL for OAuth endpoints (e.g., https://your-server.com)
    #[serde(skip_serializing_if = "Option::is_none")]
    base_url: Option<String>,
}

/// Configuration for Miro MCP Server
#[derive(Debug, Clone)]
pub struct Config {
    /// Miro OAuth2 client ID
    pub client_id: String,

    /// Miro OAuth2 client secret (optional for ADR-005 Resource Server pattern)
    /// Required only for ADR-004 OAuth Proxy pattern
    pub client_secret: String,

    /// OAuth2 redirect URI
    pub redirect_uri: String,

    /// Encryption key for token storage (32 bytes, optional for ADR-005)
    /// Required only for ADR-004 OAuth Proxy pattern (token storage)
    /// ADR-005 doesn't store tokens (they're passed in Authorization headers)
    pub encryption_key: [u8; 32],

    /// MCP server port
    pub port: u16,

    /// Base URL for OAuth proxy endpoints (e.g., https://your-server.com)
    /// Used to construct authorization_endpoint and token_endpoint in metadata
    pub base_url: Option<String>,
}

impl Config {
    /// Load configuration from file at ~/.config/mcp/miro-rust/config.json
    pub fn from_file() -> Result<Self, ConfigError> {
        let config_path = Self::get_config_path()?;

        // Read and parse the configuration file
        let contents = fs::read_to_string(&config_path).map_err(|e| ConfigError::FileNotFound {
            path: config_path.display().to_string(),
            reason: format!(
                "{}. Create the config directory and file:\n\
                     mkdir -p ~/.config/mcp/miro-rust\n\
                     cp config.example.json ~/.config/mcp/miro-rust/config.json\n\
                     Then edit the file with your Miro OAuth2 credentials.",
                e
            ),
        })?;

        let config_file: ConfigFile = serde_json::from_str(&contents)?;

        // Validate redirect URI
        let _ = url::Url::parse(&config_file.redirect_uri)?;

        // Parse encryption key from hex (use dummy value if not provided for ADR-005)
        let encryption_key = match config_file.encryption_key {
            Some(key_hex) => Self::parse_encryption_key(&key_hex)?,
            None => [0u8; 32], // Dummy key for ADR-005 (not used)
        };

        Ok(Config {
            client_id: config_file.client_id,
            client_secret: config_file.client_secret.unwrap_or_default(),
            redirect_uri: config_file.redirect_uri,
            encryption_key,
            port: config_file.port,
            base_url: config_file.base_url,
        })
    }

    /// Get the configuration file path: ~/.config/mcp/miro-rust/config.json
    fn get_config_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::home_dir()
            .map(|home| home.join(".config/mcp/miro-rust"))
            .ok_or_else(|| ConfigError::FileNotFound {
                path: "~/.config/mcp/miro-rust/config.json".to_string(),
                reason: "Could not determine home directory".to_string(),
            })?;

        Ok(config_dir.join("config.json"))
    }

    /// Ensure configuration directory exists (creates if needed)
    pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::home_dir()
            .map(|home| home.join(".config/mcp/miro-rust"))
            .ok_or_else(|| ConfigError::FileNotFound {
                path: "~/.config/mcp/miro-rust".to_string(),
                reason: "Could not determine home directory".to_string(),
            })?;

        fs::create_dir_all(&config_dir)?;
        Ok(config_dir)
    }

    /// Load configuration from environment variables
    /// Reads: MIRO_CLIENT_ID, MIRO_REDIRECT_URI, MCP_SERVER_PORT, BASE_URL
    /// Optional (for ADR-004 OAuth Proxy): MIRO_CLIENT_SECRET, MIRO_ENCRYPTION_KEY
    pub fn from_env_vars() -> Result<Self, ConfigError> {
        let client_id = std::env::var("MIRO_CLIENT_ID").map_err(|_| ConfigError::FileNotFound {
            path: "environment".to_string(),
            reason: "MIRO_CLIENT_ID environment variable not set".to_string(),
        })?;

        // Optional for ADR-005 Resource Server (OAuth handled by Claude.ai)
        let client_secret = std::env::var("MIRO_CLIENT_SECRET").unwrap_or_else(|_| "".to_string());

        let redirect_uri =
            std::env::var("MIRO_REDIRECT_URI").map_err(|_| ConfigError::FileNotFound {
                path: "environment".to_string(),
                reason: "MIRO_REDIRECT_URI environment variable not set".to_string(),
            })?;

        // Validate redirect URI
        let _ = url::Url::parse(&redirect_uri)?;

        // Optional for ADR-005 Resource Server (no token storage)
        let encryption_key = match std::env::var("MIRO_ENCRYPTION_KEY") {
            Ok(key_hex) => Self::parse_encryption_key(&key_hex)?,
            Err(_) => [0u8; 32], // Dummy key for ADR-005 (not used)
        };

        let port = std::env::var("MCP_SERVER_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(3000);

        let base_url = std::env::var("BASE_URL").ok();

        Ok(Config {
            client_id,
            client_secret,
            redirect_uri,
            encryption_key,
            port,
            base_url,
        })
    }

    /// Load configuration from environment variables first, fallback to config file
    /// Priority: Environment variables > Config file
    pub fn from_env_or_file() -> Result<Self, ConfigError> {
        // Try environment variables first (for container deployment)
        match Self::from_env_vars() {
            Ok(config) => {
                eprintln!("✓ Configuration loaded from environment variables");
                Ok(config)
            }
            Err(env_err) => {
                eprintln!("⚠ Failed to load config from environment: {}", env_err);
                eprintln!("  Falling back to config file...");
                // Fall back to config file (for local development)
                match Self::from_file() {
                    Ok(config) => Ok(config),
                    Err(file_err) => {
                        // Return both errors for better diagnostics
                        Err(ConfigError::FileNotFound {
                            path: "environment or file".to_string(),
                            reason: format!(
                                "Environment variable error: {}\nConfig file error: {}",
                                env_err, file_err
                            ),
                        })
                    }
                }
            }
        }
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

    #[test]
    #[serial_test::serial]
    fn test_from_env_vars_success() {
        // Set required environment variables
        std::env::set_var("MIRO_CLIENT_ID", "test_client_id");
        std::env::set_var("MIRO_CLIENT_SECRET", "test_secret");
        std::env::set_var("MIRO_REDIRECT_URI", "http://localhost:3000/callback");
        std::env::set_var(
            "MIRO_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        std::env::set_var("MCP_SERVER_PORT", "8080");

        let result = Config::from_env_vars();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.client_id, "test_client_id");
        assert_eq!(config.client_secret, "test_secret");
        assert_eq!(config.redirect_uri, "http://localhost:3000/callback");
        assert_eq!(config.port, 8080);

        // Cleanup
        std::env::remove_var("MIRO_CLIENT_ID");
        std::env::remove_var("MIRO_CLIENT_SECRET");
        std::env::remove_var("MIRO_REDIRECT_URI");
        std::env::remove_var("MIRO_ENCRYPTION_KEY");
        std::env::remove_var("MCP_SERVER_PORT");
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_vars_missing_var() {
        // Ensure variables are not set
        std::env::remove_var("MIRO_CLIENT_ID");
        std::env::remove_var("MIRO_CLIENT_SECRET");

        let result = Config::from_env_vars();
        assert!(result.is_err());
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_vars_default_port() {
        // Set required environment variables (without port)
        std::env::set_var("MIRO_CLIENT_ID", "test_client_id");
        std::env::set_var("MIRO_CLIENT_SECRET", "test_secret");
        std::env::set_var("MIRO_REDIRECT_URI", "http://localhost:3000/callback");
        std::env::set_var(
            "MIRO_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        std::env::remove_var("MCP_SERVER_PORT");

        let result = Config::from_env_vars();
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.port, 3000); // Default port

        // Cleanup
        std::env::remove_var("MIRO_CLIENT_ID");
        std::env::remove_var("MIRO_CLIENT_SECRET");
        std::env::remove_var("MIRO_REDIRECT_URI");
        std::env::remove_var("MIRO_ENCRYPTION_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn test_from_env_vars_invalid_redirect_uri() {
        std::env::set_var("MIRO_CLIENT_ID", "test_client_id");
        std::env::set_var("MIRO_CLIENT_SECRET", "test_secret");
        std::env::set_var("MIRO_REDIRECT_URI", "not_a_valid_url");
        std::env::set_var(
            "MIRO_ENCRYPTION_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );

        let result = Config::from_env_vars();
        assert!(result.is_err());

        // Cleanup
        std::env::remove_var("MIRO_CLIENT_ID");
        std::env::remove_var("MIRO_CLIENT_SECRET");
        std::env::remove_var("MIRO_REDIRECT_URI");
        std::env::remove_var("MIRO_ENCRYPTION_KEY");
    }
}
