use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use miro_mcp_server::mcp::oauth_metadata;
use serde_json::Value;
use tower::ServiceExt;

/// Test that metadata endpoint returns RFC 9728 Protected Resource Metadata
#[tokio::test]
async fn test_metadata_endpoint_returns_rfc9728_format() {
    // Create router with metadata endpoint
    let app = Router::new().route("/.well-known/oauth-protected-resource",
        axum::routing::get(oauth_metadata));

    // Make request to metadata endpoint
    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-protected-resource")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 OK
    assert_eq!(response.status(), StatusCode::OK);

    // Parse response body
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metadata: Value = serde_json::from_slice(&body).unwrap();

    // Verify RFC 9728 required fields
    assert!(
        metadata.get("resource").is_some(),
        "Missing 'resource' field (RFC 9728 required)"
    );
    assert!(
        metadata.get("authorization_servers").is_some(),
        "Missing 'authorization_servers' field (RFC 9728 required)"
    );

    // Verify values are correct
    assert_eq!(
        metadata["resource"].as_str().unwrap(),
        "https://api.miro.com",
        "Resource should be Miro API"
    );

    let auth_servers = metadata["authorization_servers"]
        .as_array()
        .expect("authorization_servers should be array");
    assert_eq!(
        auth_servers.len(),
        1,
        "Should have exactly one authorization server"
    );
    assert_eq!(
        auth_servers[0].as_str().unwrap(),
        "https://miro.com/oauth",
        "Authorization server should be Miro OAuth"
    );

    // Verify optional fields are present and correct
    assert!(
        metadata.get("scopes_supported").is_some(),
        "Should include scopes_supported"
    );
    let scopes = metadata["scopes_supported"]
        .as_array()
        .expect("scopes_supported should be array");
    assert!(
        scopes.contains(&Value::String("boards:read".to_string())),
        "Should support boards:read scope"
    );
    assert!(
        scopes.contains(&Value::String("boards:write".to_string())),
        "Should support boards:write scope"
    );

    // Verify bearer methods
    assert!(
        metadata.get("bearer_methods_supported").is_some(),
        "Should include bearer_methods_supported"
    );
    let methods = metadata["bearer_methods_supported"]
        .as_array()
        .expect("bearer_methods_supported should be array");
    assert!(
        methods.contains(&Value::String("header".to_string())),
        "Should support header bearer method"
    );
}

/// Test that metadata does NOT include RFC 8414 Authorization Server fields
#[tokio::test]
async fn test_metadata_does_not_include_rfc8414_fields() {
    let app = Router::new().route("/.well-known/oauth-protected-resource",
        axum::routing::get(oauth_metadata));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-protected-resource")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metadata: Value = serde_json::from_slice(&body).unwrap();

    // These are RFC 8414 fields that should NOT be in RFC 9728 metadata
    assert!(
        metadata.get("issuer").is_none(),
        "Should NOT include 'issuer' (that's RFC 8414 Authorization Server metadata)"
    );
    assert!(
        metadata.get("authorization_endpoint").is_none(),
        "Should NOT include 'authorization_endpoint' (that's RFC 8414)"
    );
    assert!(
        metadata.get("token_endpoint").is_none(),
        "Should NOT include 'token_endpoint' (that's RFC 8414)"
    );
}

/// Test WWW-Authenticate header includes resource_metadata parameter
#[tokio::test]
async fn test_www_authenticate_includes_resource_metadata() {
    use axum::http::header::WWW_AUTHENTICATE;
    use miro_mcp_server::{http_server::create_http_server, TokenValidator};
    use std::sync::Arc;

    // Create mock token validator (won't be called in this test)
    let token_validator = Arc::new(TokenValidator::new());

    // Create app with bearer middleware
    let app = create_http_server(token_validator);

    // Make request without auth token
    let response = app
        .oneshot(
            Request::builder()
                .uri("/mcp")
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 401
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Check WWW-Authenticate header
    let www_auth = response
        .headers()
        .get(WWW_AUTHENTICATE)
        .expect("Should have WWW-Authenticate header")
        .to_str()
        .unwrap();

    // Should include resource_metadata parameter per RFC 9728
    assert!(
        www_auth.contains("resource_metadata="),
        "WWW-Authenticate should include resource_metadata parameter, got: {}",
        www_auth
    );
    assert!(
        www_auth.contains("/.well-known/oauth-protected-resource"),
        "resource_metadata should point to metadata endpoint, got: {}",
        www_auth
    );
}
