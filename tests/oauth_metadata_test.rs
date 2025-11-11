use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use miro_mcp_server::mcp::{oauth_authorization_server_metadata, oauth_metadata};
use serde_json::Value;
use tower::ServiceExt;

/// Test RFC 8414 Authorization Server Metadata endpoint
/// This endpoint is used for Dynamic Client Registration and standard OAuth flows
#[tokio::test]
async fn test_authorization_server_metadata_endpoint() {
    use miro_mcp_server::Config;
    use std::sync::Arc;

    let config = Arc::new(Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:3010/oauth/callback".to_string(),
        encryption_key: [0u8; 32],
        port: 3010,
        base_url: Some("http://localhost:3010".to_string()),
    });

    let app = Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            axum::routing::get(oauth_authorization_server_metadata),
        )
        .with_state(config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-authorization-server")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metadata: Value = serde_json::from_slice(&body).unwrap();

    // RFC 8414 required fields for Authorization Server
    assert!(
        metadata.get("issuer").is_some(),
        "Missing 'issuer' field (RFC 8414 required)"
    );
    assert!(
        metadata.get("authorization_endpoint").is_some(),
        "Missing 'authorization_endpoint' field (RFC 8414 required)"
    );
    assert!(
        metadata.get("token_endpoint").is_some(),
        "Missing 'token_endpoint' field (RFC 8414 required)"
    );

    // Verify endpoint values point to our server
    let issuer = metadata["issuer"].as_str().unwrap();
    assert_eq!(
        issuer, "http://localhost:3010",
        "Issuer should be our base URL"
    );

    let auth_endpoint = metadata["authorization_endpoint"].as_str().unwrap();
    assert!(
        auth_endpoint.contains("/oauth/authorize"),
        "Authorization endpoint should be at /oauth/authorize"
    );

    let token_endpoint = metadata["token_endpoint"].as_str().unwrap();
    assert!(
        token_endpoint.contains("/oauth/token"),
        "Token endpoint should be at /oauth/token"
    );

    // Dynamic Client Registration support
    assert!(
        metadata.get("registration_endpoint").is_some(),
        "Missing 'registration_endpoint' (RFC 7591 support)"
    );
    let reg_endpoint = metadata["registration_endpoint"].as_str().unwrap();
    assert!(
        reg_endpoint.contains("/register"),
        "Registration endpoint should be at /register"
    );

    // Grant and response types
    let grant_types = metadata["grant_types_supported"]
        .as_array()
        .expect("grant_types_supported should be array");
    assert!(
        grant_types.contains(&Value::String("authorization_code".to_string())),
        "Should support authorization_code grant type"
    );

    let response_types = metadata["response_types_supported"]
        .as_array()
        .expect("response_types_supported should be array");
    assert!(
        response_types.contains(&Value::String("code".to_string())),
        "Should support code response type"
    );

    // Token endpoint auth methods
    let auth_methods = metadata["token_endpoint_auth_methods_supported"]
        .as_array()
        .expect("token_endpoint_auth_methods_supported should be array");
    assert!(
        auth_methods.contains(&Value::String("client_secret_basic".to_string())),
        "Should support client_secret_basic auth method"
    );
    assert!(
        auth_methods.contains(&Value::String("client_secret_post".to_string())),
        "Should support client_secret_post auth method"
    );
}

/// Test Protected Resource Metadata endpoint for ADR-004 OAuth Proxy pattern
/// This tells Claude.ai to use our server as the OAuth proxy to Miro
#[tokio::test]
async fn test_protected_resource_metadata_endpoint() {
    use miro_mcp_server::Config;
    use std::sync::Arc;

    let config = Arc::new(Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:3010/oauth/callback".to_string(),
        encryption_key: [0u8; 32],
        port: 3010,
        base_url: Some("http://localhost:3010".to_string()),
    });

    let app = Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            axum::routing::get(oauth_metadata),
        )
        .with_state(config);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-protected-resource")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let metadata: Value = serde_json::from_slice(&body).unwrap();

    // ADR-004: Protected resource metadata includes authorization/token endpoints
    // that point to OUR server, not Miro directly
    assert!(metadata.get("issuer").is_some(), "Missing 'issuer' field");
    assert!(
        metadata.get("authorization_endpoint").is_some(),
        "Missing 'authorization_endpoint' field"
    );
    assert!(
        metadata.get("token_endpoint").is_some(),
        "Missing 'token_endpoint' field"
    );

    // Issuer should be actual provider (Miro)
    let issuer = metadata["issuer"].as_str().unwrap();
    assert_eq!(issuer, "https://miro.com", "Issuer should be Miro");

    // But endpoints should point to OUR proxy
    let auth_endpoint = metadata["authorization_endpoint"].as_str().unwrap();
    assert!(
        auth_endpoint.contains("localhost:3010/oauth/authorize"),
        "Authorization endpoint should point to our proxy, got: {}",
        auth_endpoint
    );

    let token_endpoint = metadata["token_endpoint"].as_str().unwrap();
    assert!(
        token_endpoint.contains("localhost:3010/oauth/token"),
        "Token endpoint should point to our proxy, got: {}",
        token_endpoint
    );

    // Grant and response types
    let grant_types = metadata["grant_types_supported"]
        .as_array()
        .expect("grant_types_supported should be array");
    assert!(
        grant_types.contains(&Value::String("authorization_code".to_string())),
        "Should support authorization_code grant type"
    );

    let response_types = metadata["response_types_supported"]
        .as_array()
        .expect("response_types_supported should be array");
    assert!(
        response_types.contains(&Value::String("code".to_string())),
        "Should support code response type"
    );
}

/// Test Bearer token authentication with WWW-Authenticate header
/// Verifies that unauthenticated requests get 401 with proper WWW-Authenticate header
#[tokio::test]
async fn test_bearer_auth_returns_401_with_www_authenticate() {
    use axum::http::header::WWW_AUTHENTICATE;
    use miro_mcp_server::{http_server::create_app_adr002, Config, TokenValidator};
    use std::sync::Arc;

    let token_validator = Arc::new(TokenValidator::new());
    let config = Arc::new(Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:3010/oauth/callback".to_string(),
        encryption_key: [0u8; 32],
        port: 3010,
        base_url: Some("http://localhost:3010".to_string()),
    });

    // Create app with bearer middleware
    #[cfg(feature = "oauth-proxy")]
    {
        use miro_mcp_server::oauth::cookie_manager::CookieManager;
        use miro_mcp_server::oauth::proxy_provider::MiroOAuthProvider;

        let oauth_provider = Arc::new(MiroOAuthProvider::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.redirect_uri.clone(),
        ));
        let cookie_manager = Arc::new(CookieManager::new(&config.encryption_key));

        let app = create_app_adr002(token_validator, config, oauth_provider, cookie_manager);

        // Make request without auth token
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/mcp")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 401 when missing authorization
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Check WWW-Authenticate header per RFC 6750
        let www_auth = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .expect("Should have WWW-Authenticate header")
            .to_str()
            .unwrap();

        // Should include Bearer scheme and realm
        assert!(
            www_auth.contains("Bearer"),
            "WWW-Authenticate should use Bearer scheme, got: {}",
            www_auth
        );
        assert!(
            www_auth.contains("realm"),
            "WWW-Authenticate should include realm, got: {}",
            www_auth
        );
    }

    #[cfg(not(feature = "oauth-proxy"))]
    {
        let app = create_app_adr002(token_validator, config);

        // Make request without auth token
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/mcp")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return 401 when missing authorization
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Check WWW-Authenticate header per RFC 6750
        let www_auth = response
            .headers()
            .get(WWW_AUTHENTICATE)
            .expect("Should have WWW-Authenticate header")
            .to_str()
            .unwrap();

        // Should include Bearer scheme and realm
        assert!(
            www_auth.contains("Bearer"),
            "WWW-Authenticate should use Bearer scheme, got: {}",
            www_auth
        );
        assert!(
            www_auth.contains("realm"),
            "WWW-Authenticate should include realm, got: {}",
            www_auth
        );
    }
}
