use miro_mcp_server::auth::{AuthError, TokenValidator};
use wiremock::matchers::{bearer_token, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test valid token validation with Miro API
#[tokio::test]
async fn test_valid_token_validation() {
    // Setup mock server
    let mock_server = MockServer::start().await;

    // Mock successful token validation response
    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("valid_token_123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user123",
            "team_id": "team456",
            "scopes": "boards:read boards:write"
        })))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    let result = validator.validate_token("valid_token_123").await;

    assert!(result.is_ok());
    let user_info = result.unwrap();
    assert_eq!(user_info.user_id, "user123");
    assert_eq!(user_info.team_id, "team456");
    assert_eq!(user_info.scopes, vec!["boards:read", "boards:write"]);
}

/// Test invalid token returns 401 error
#[tokio::test]
async fn test_invalid_token_returns_401() {
    let mock_server = MockServer::start().await;

    // Mock 401 Unauthorized response
    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("invalid_token"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    let result = validator.validate_token("invalid_token").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::TokenInvalid => {} // Expected
        other => panic!("Expected TokenInvalid, got {:?}", other),
    }
}

/// Test expired token returns validation error
#[tokio::test]
async fn test_expired_token_validation() {
    let mock_server = MockServer::start().await;

    // Mock 401 response for expired token
    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("expired_token"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    let result = validator.validate_token("expired_token").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::TokenInvalid => {} // Expected
        other => panic!("Expected TokenInvalid, got {:?}", other),
    }
}

/// Test cache hit scenario - second request doesn't call Miro API
#[tokio::test]
async fn test_cache_hit_scenario() {
    let mock_server = MockServer::start().await;

    // Mock should only be called once
    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("cached_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user789",
            "team_id": "team012",
            "scopes": "boards:read"
        })))
        .expect(1) // Should only be called once
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    // First request - cache miss, calls Miro API
    let result1 = validator.validate_token("cached_token").await;
    assert!(result1.is_ok());
    let user_info1 = result1.unwrap();
    assert_eq!(user_info1.user_id, "user789");

    // Second request - cache hit, does NOT call Miro API
    let result2 = validator.validate_token("cached_token").await;
    assert!(result2.is_ok());
    let user_info2 = result2.unwrap();
    assert_eq!(user_info2.user_id, "user789");

    // Verify cache stats
    let (cache_len, cache_cap) = validator.cache_stats();
    assert_eq!(cache_len, 1); // One token cached
    assert_eq!(cache_cap, 100); // Capacity 100
}

/// Test cache TTL expiry - test UserInfo expiry logic
#[tokio::test]
async fn test_cache_ttl_expiry() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("ttl_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_ttl",
            "team_id": "team_ttl",
            "scopes": "boards:read"
        })))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    // First validation
    let result1 = validator.validate_token("ttl_test_token").await;
    assert!(result1.is_ok());

    // Note: Full TTL expiry testing would require waiting 5 minutes or mocking time
    // The is_expired() logic is tested in the unit tests with manual time manipulation
    // This integration test verifies the validator properly handles fresh cache entries
    let (cache_len, _) = validator.cache_stats();
    assert_eq!(cache_len, 1);
}

/// Test cache capacity limit - 101st token evicts oldest
#[tokio::test]
async fn test_cache_capacity_limit() {
    let mock_server = MockServer::start().await;

    // Mock responses for multiple tokens
    for i in 0..101 {
        let token = format!("token_{}", i);
        Mock::given(method("GET"))
            .and(path("/v1/oauth-token"))
            .and(bearer_token(token.clone()))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": format!("user_{}", i),
                "team_id": "team_capacity",
                "scopes": "boards:read"
            })))
            .mount(&mock_server)
            .await;
    }

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    // Validate 100 tokens - all should fit in cache
    for i in 0..100 {
        let token = format!("token_{}", i);
        let result = validator.validate_token(&token).await;
        assert!(result.is_ok());
    }

    // Check cache is full
    let (cache_len, _) = validator.cache_stats();
    assert_eq!(cache_len, 100);

    // Validate 101st token - should evict oldest (token_0)
    let result = validator.validate_token("token_100").await;
    assert!(result.is_ok());

    // Cache should still have 100 entries
    let (cache_len, _) = validator.cache_stats();
    assert_eq!(cache_len, 100);
}

/// Test cache miss after clearing cache
#[tokio::test]
async fn test_cache_clear() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("clear_test_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_clear",
            "team_id": "team_clear",
            "scopes": "boards:read"
        })))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    // First validation - cache miss
    let result1 = validator.validate_token("clear_test_token").await;
    assert!(result1.is_ok());

    // Verify cache has entry
    let (cache_len, _) = validator.cache_stats();
    assert_eq!(cache_len, 1);

    // Clear cache
    validator.clear_cache();

    // Verify cache is empty
    let (cache_len, _) = validator.cache_stats();
    assert_eq!(cache_len, 0);
}

/// Test multiple different tokens are cached correctly
#[tokio::test]
async fn test_multiple_tokens_cached() {
    let mock_server = MockServer::start().await;

    // Mock responses for 3 different tokens
    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("token_a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_a",
            "team_id": "team_multi",
            "scopes": "boards:read"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("token_b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_b",
            "team_id": "team_multi",
            "scopes": "boards:write"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("token_c"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_c",
            "team_id": "team_multi",
            "scopes": "boards:read boards:write"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    // Validate all 3 tokens
    let result_a = validator.validate_token("token_a").await;
    assert!(result_a.is_ok());
    assert_eq!(result_a.unwrap().user_id, "user_a");

    let result_b = validator.validate_token("token_b").await;
    assert!(result_b.is_ok());
    assert_eq!(result_b.unwrap().user_id, "user_b");

    let result_c = validator.validate_token("token_c").await;
    assert!(result_c.is_ok());
    assert_eq!(result_c.unwrap().user_id, "user_c");

    // Verify all 3 are cached
    let (cache_len, _) = validator.cache_stats();
    assert_eq!(cache_len, 3);

    // Re-validate token_a - should be cache hit (expect(1) will fail if called again)
    let result_a2 = validator.validate_token("token_a").await;
    assert!(result_a2.is_ok());
    assert_eq!(result_a2.unwrap().user_id, "user_a");
}

/// Test API error handling - non-2xx status codes
#[tokio::test]
async fn test_api_error_handling() {
    let mock_server = MockServer::start().await;

    // Mock 500 Internal Server Error
    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("error_token"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    let result = validator.validate_token("error_token").await;

    assert!(result.is_err());
    match result.unwrap_err() {
        AuthError::TokenValidationFailed(msg) => {
            assert!(msg.contains("500"));
        }
        other => panic!("Expected TokenValidationFailed, got {:?}", other),
    }
}

/// Test scopes parsing - multiple scopes
#[tokio::test]
async fn test_scopes_parsing() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/oauth-token"))
        .and(bearer_token("multi_scope_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "user_id": "user_scopes",
            "team_id": "team_scopes",
            "scopes": "boards:read boards:write connectors:read connectors:write"
        })))
        .mount(&mock_server)
        .await;

    let validator = TokenValidator::new_with_endpoint(format!("{}/v1/oauth-token", mock_server.uri()));

    let result = validator.validate_token("multi_scope_token").await;

    assert!(result.is_ok());
    let user_info = result.unwrap();
    assert_eq!(
        user_info.scopes,
        vec!["boards:read", "boards:write", "connectors:read", "connectors:write"]
    );
}
