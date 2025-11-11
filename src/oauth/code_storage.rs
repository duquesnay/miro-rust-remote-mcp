/// In-memory storage for pending authorization codes
///
/// Stores code exchanges between callback and token endpoints.
/// Uses Arc<RwLock<HashMap>> for thread-safe access in async context.
///
/// # Why not cookies?
/// Cookies don't work for OAuth Authorization Server pattern because:
/// - Callback sets cookie in user's browser
/// - Token endpoint is called by Claude.ai's backend (server-to-server)
/// - Backend doesn't have access to browser cookies
///
/// # Cleanup Strategy
/// - Codes expire after 5 minutes (PENDING_CODE_MAX_AGE)
/// - Cleanup runs periodically via background task
/// - Codes are also removed immediately after successful exchange
use super::types::PendingCodeExchange;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Storage for pending authorization codes awaiting token exchange
/// Thread-safe wrapper around HashMap for async access
#[derive(Clone)]
pub struct CodeStorage {
    /// Map: authorization code -> pending exchange data
    store: Arc<RwLock<HashMap<String, PendingCodeExchange>>>,
}

impl CodeStorage {
    /// Create new empty code storage
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a pending code exchange (called from callback handler)
    pub async fn store(&self, code: &str, pending: PendingCodeExchange) {
        let mut store = self.store.write().await;
        info!(
            code_length = code.len(),
            expires_at = %pending.expires_at,
            "Storing pending code exchange"
        );
        store.insert(code.to_string(), pending);
    }

    /// Retrieve and remove a pending code exchange (called from token handler)
    /// Returns None if code not found or expired
    pub async fn take(&self, code: &str) -> Option<PendingCodeExchange> {
        let mut store = self.store.write().await;

        // Remove from map
        let pending = store.remove(code)?;

        // Check if expired
        let now = Utc::now();
        if now > pending.expires_at {
            warn!(
                code_length = code.len(),
                expired_at = %pending.expires_at,
                "Authorization code has expired"
            );
            return None;
        }

        info!(
            code_length = code.len(),
            "Retrieved pending code exchange"
        );
        Some(pending)
    }

    /// Remove expired codes (cleanup task)
    pub async fn cleanup_expired(&self) {
        let mut store = self.store.write().await;
        let now = Utc::now();
        let initial_count = store.len();

        // Remove expired entries
        store.retain(|code, pending| {
            if now > pending.expires_at {
                warn!(
                    code_length = code.len(),
                    expired_at = %pending.expires_at,
                    "Removing expired authorization code"
                );
                false
            } else {
                true
            }
        });

        let removed = initial_count - store.len();
        if removed > 0 {
            info!(
                removed = removed,
                remaining = store.len(),
                "Cleaned up expired authorization codes"
            );
        }
    }

    /// Get current storage statistics (for monitoring/debugging)
    pub async fn stats(&self) -> (usize, usize) {
        let store = self.store.read().await;
        let total = store.len();
        let now = Utc::now();
        let expired = store.values().filter(|p| now > p.expires_at).count();
        (total, expired)
    }
}

impl Default for CodeStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Start background cleanup task for expired codes
/// Runs every 60 seconds to remove expired entries
pub fn start_cleanup_task(storage: CodeStorage) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            storage.cleanup_expired().await;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[tokio::test]
    async fn test_store_and_take() {
        let storage = CodeStorage::new();
        let code = "test_code_123";
        let pending = PendingCodeExchange {
            code: code.to_string(),
            code_verifier: "verifier_abc".to_string(),
            expires_at: Utc::now() + Duration::seconds(300),
        };

        // Store
        storage.store(code, pending.clone()).await;

        // Retrieve
        let retrieved = storage.take(code).await.unwrap();
        assert_eq!(retrieved.code, pending.code);
        assert_eq!(retrieved.code_verifier, pending.code_verifier);

        // Second take should return None (already removed)
        assert!(storage.take(code).await.is_none());
    }

    #[tokio::test]
    async fn test_expired_code() {
        let storage = CodeStorage::new();
        let code = "expired_code";
        let pending = PendingCodeExchange {
            code: code.to_string(),
            code_verifier: "verifier".to_string(),
            expires_at: Utc::now() - Duration::seconds(60), // Expired 1 minute ago
        };

        storage.store(code, pending).await;

        // Should return None for expired code
        assert!(storage.take(code).await.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let storage = CodeStorage::new();

        // Add expired code
        let expired_code = "expired";
        storage
            .store(
                expired_code,
                PendingCodeExchange {
                    code: expired_code.to_string(),
                    code_verifier: "v1".to_string(),
                    expires_at: Utc::now() - Duration::seconds(60),
                },
            )
            .await;

        // Add valid code
        let valid_code = "valid";
        storage
            .store(
                valid_code,
                PendingCodeExchange {
                    code: valid_code.to_string(),
                    code_verifier: "v2".to_string(),
                    expires_at: Utc::now() + Duration::seconds(300),
                },
            )
            .await;

        // Cleanup
        storage.cleanup_expired().await;

        // Check stats
        let (total, expired) = storage.stats().await;
        assert_eq!(total, 1); // Only valid code remains
        assert_eq!(expired, 0); // No expired codes left
    }
}
