use crate::auth::types::AuthError;
use lru::LruCache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// User information returned from Miro token validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Miro user ID
    pub user_id: String,
    /// Miro team ID
    pub team_id: String,
    /// Scopes granted to the token
    pub scopes: Vec<String>,
    /// Timestamp when this cache entry was created
    #[serde(skip)]
    cached_at: u64,
}

impl UserInfo {
    /// Create new UserInfo with current timestamp
    pub fn new(user_id: String, team_id: String, scopes: Vec<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        Self {
            user_id,
            team_id,
            scopes,
            cached_at: now,
        }
    }

    /// Check if this cache entry is expired (5 minute TTL)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        const TTL_SECONDS: u64 = 5 * 60; // 5 minutes
        now - self.cached_at > TTL_SECONDS
    }
}

/// Response from Miro's token introspection endpoint
#[derive(Debug, Deserialize)]
struct MiroTokenResponse {
    #[serde(rename = "user_id")]
    user: String,
    #[serde(rename = "team_id")]
    team: String,
    #[serde(rename = "scopes")]
    scopes: String, // Space-separated string
}

/// Token validator with LRU caching
pub struct TokenValidator {
    /// LRU cache for validated tokens (capacity: 100)
    cache: Mutex<LruCache<String, UserInfo>>,
    /// HTTP client for Miro API calls
    http_client: Client,
    /// Miro OAuth token endpoint
    token_endpoint: String,
}

impl TokenValidator {
    /// Create a new token validator
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            http_client: Client::new(),
            token_endpoint: "https://api.miro.com/v1/oauth-token".to_string(),
        }
    }

    /// Create a token validator with custom endpoint (for testing)
    pub fn new_with_endpoint(endpoint: String) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(100).unwrap())),
            http_client: Client::new(),
            token_endpoint: endpoint,
        }
    }

    /// Validate a token and return user info
    ///
    /// First checks the cache, then validates with Miro API if cache miss or expired.
    /// Returns 401 for invalid or expired tokens.
    pub async fn validate_token(&self, token: &str) -> Result<UserInfo, AuthError> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(user_info) = cache.get(token) {
                if !user_info.is_expired() {
                    debug!(
                        user_id = %user_info.user_id,
                        "Token validation cache hit"
                    );
                    return Ok(user_info.clone());
                } else {
                    debug!("Cached token expired, revalidating");
                    // Remove expired entry
                    cache.pop(token);
                }
            }
        }

        // Cache miss or expired - validate with Miro API
        debug!("Token validation cache miss, calling Miro API");
        let user_info = self.validate_with_miro(token).await?;

        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(token.to_string(), user_info.clone());
        }

        info!(
            user_id = %user_info.user_id,
            team_id = %user_info.team_id,
            "Token validated successfully"
        );

        Ok(user_info)
    }

    /// Validate token with Miro API
    async fn validate_with_miro(&self, token: &str) -> Result<UserInfo, AuthError> {
        debug!(
            endpoint = %self.token_endpoint,
            "Calling Miro token validation endpoint"
        );

        let response = self
            .http_client
            .get(&self.token_endpoint)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| {
                warn!(
                    error = %e,
                    endpoint = %self.token_endpoint,
                    error_type = "http_request_failed",
                    "Failed to call Miro token endpoint"
                );
                AuthError::TokenValidationFailed(format!("HTTP request failed: {}", e))
            })?;

        let status = response.status();

        if status == reqwest::StatusCode::UNAUTHORIZED {
            warn!(
                status = %status,
                error_type = "invalid_token",
                "Token validation failed: 401 Unauthorized from Miro API"
            );
            return Err(AuthError::TokenInvalid);
        }

        if !status.is_success() {
            warn!(
                status = %status,
                error_type = "api_error",
                "Token validation failed with non-2xx status from Miro API"
            );
            return Err(AuthError::TokenValidationFailed(format!(
                "Miro API returned status {}",
                status
            )));
        }

        let miro_response: MiroTokenResponse = response.json().await.map_err(|e| {
            warn!(
                error = %e,
                error_type = "json_parse_failed",
                "Failed to parse Miro token response"
            );
            AuthError::TokenValidationFailed(format!("Failed to parse response: {}", e))
        })?;

        debug!(
            user_id = %miro_response.user,
            team_id = %miro_response.team,
            scopes = %miro_response.scopes,
            "Miro API returned valid token information"
        );

        // Parse space-separated scopes
        let scopes: Vec<String> = miro_response
            .scopes
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        Ok(UserInfo::new(
            miro_response.user,
            miro_response.team,
            scopes,
        ))
    }

    /// Get cache statistics (for testing and monitoring)
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        (cache.len(), cache.cap().get())
    }

    /// Clear the cache (for testing)
    pub fn clear_cache(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
}

impl Default for TokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_info_expiry() {
        let user_info = UserInfo::new(
            "user123".to_string(),
            "team456".to_string(),
            vec!["boards:read".to_string()],
        );

        // Should not be expired immediately
        assert!(!user_info.is_expired());
    }

    #[test]
    fn test_user_info_expired_after_ttl() {
        let mut user_info = UserInfo::new(
            "user123".to_string(),
            "team456".to_string(),
            vec!["boards:read".to_string()],
        );

        // Manually set cached_at to 6 minutes ago
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        user_info.cached_at = now - (6 * 60); // 6 minutes ago

        // Should be expired
        assert!(user_info.is_expired());
    }

    #[test]
    fn test_token_validator_creation() {
        let validator = TokenValidator::new();
        let stats = validator.cache_stats();
        assert_eq!(stats.0, 0); // Empty cache
        assert_eq!(stats.1, 100); // Capacity 100
    }
}
