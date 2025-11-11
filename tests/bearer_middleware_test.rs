use axum::http::{Request, StatusCode};
use axum::Router;
use miro_mcp_server::{http_server::create_app_adr002, Config, TokenValidator};
use std::sync::Arc;
use tower::ServiceExt;

#[cfg(feature = "oauth-proxy")]
use miro_mcp_server::oauth::cookie_manager::CookieManager;
#[cfg(feature = "oauth-proxy")]
use miro_mcp_server::oauth::proxy_provider::MiroOAuthProvider;

fn get_test_config() -> Config {
    Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:3010/oauth/callback".to_string(),
        encryption_key: [0u8; 32],
        port: 3010,
        base_url: Some("http://localhost:3010".to_string()),
    }
}

fn create_test_app() -> Router {
    let config = Arc::new(get_test_config());
    let token_validator = Arc::new(TokenValidator::new());

    #[cfg(feature = "oauth-proxy")]
    {
        let oauth_provider = Arc::new(MiroOAuthProvider::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.redirect_uri.clone(),
        ));
        let cookie_manager = Arc::new(CookieManager::new(&config.encryption_key));

        create_app_adr002(token_validator, config, oauth_provider, cookie_manager)
    }

    #[cfg(not(feature = "oauth-proxy"))]
    {
        create_app_adr002(token_validator, config)
    }
}

#[tokio::test]
async fn test_health_endpoint_no_auth_required() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_oauth_metadata_no_auth_required() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-protected-resource")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_oauth_authorize_no_auth_required() {
    let app = create_test_app();

    // OAuth authorization request with required query parameters
    let response = app
        .oneshot(
            Request::builder()
                .uri("/oauth/authorize?response_type=code&client_id=test_client&redirect_uri=https://claude.ai/api/mcp/auth_callback")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should redirect to Miro (302 or 303)
    assert!(
        response.status().is_redirection(),
        "Expected redirect, got: {}",
        response.status()
    );
}

// Note: Protected routes tests will be added when MCP endpoints are implemented
// For now, we verify that:
// 1. Public routes work without auth
// 2. Protected routes middleware is in place (will reject missing tokens when routes exist)
