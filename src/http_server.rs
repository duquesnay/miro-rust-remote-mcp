use crate::auth::{CookieStateManager, MiroOAuthClient, OAuthCookieState, TokenStore};
use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use oauth2::PkceCodeVerifier;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// OAuth callback query parameters
#[derive(Debug, Deserialize)]
pub struct OAuthCallback {
    code: String,
    state: String,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    oauth_client: Arc<MiroOAuthClient>,
    token_store: Arc<RwLock<TokenStore>>,
    cookie_manager: CookieStateManager,
}

/// Handle OAuth callback from Miro
async fn oauth_callback(
    Query(params): Query<OAuthCallback>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Response {
    info!("Received OAuth callback with code");

    match handle_oauth_exchange(params, state, headers).await {
        Ok(()) => Html(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Authorization Successful</title>
                <style>
                    body {
                        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                        margin: 0;
                        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    }
                    .container {
                        background: white;
                        padding: 3rem;
                        border-radius: 12px;
                        box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                        text-align: center;
                        max-width: 500px;
                    }
                    h1 { color: #2d3748; margin-bottom: 1rem; }
                    p { color: #4a5568; line-height: 1.6; }
                    .success { color: #48bb78; font-size: 3rem; margin-bottom: 1rem; }
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="success">✓</div>
                    <h1>Authorization Successful!</h1>
                    <p>Your Miro account has been connected.</p>
                    <p>You can now close this window and return to Claude.</p>
                </div>
            </body>
            </html>
            "#
        )
        .into_response(),
        Err(e) => {
            error!("OAuth exchange failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <title>Authorization Failed</title>
                        <style>
                            body {{
                                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
                                display: flex;
                                justify-content: center;
                                align-items: center;
                                height: 100vh;
                                margin: 0;
                                background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
                            }}
                            .container {{
                                background: white;
                                padding: 3rem;
                                border-radius: 12px;
                                box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
                                text-align: center;
                                max-width: 500px;
                            }}
                            h1 {{ color: #2d3748; margin-bottom: 1rem; }}
                            p {{ color: #4a5568; line-height: 1.6; }}
                            .error {{ color: #f56565; font-size: 3rem; margin-bottom: 1rem; }}
                            code {{ background: #f7fafc; padding: 0.2rem 0.4rem; border-radius: 3px; }}
                        </style>
                    </head>
                    <body>
                        <div class="container">
                            <div class="error">✗</div>
                            <h1>Authorization Failed</h1>
                            <p>Error: <code>{}</code></p>
                            <p>Please try again or contact support.</p>
                        </div>
                    </body>
                    </html>
                    "#,
                    e
                )),
            )
                .into_response()
        }
    }
}

/// Exchange authorization code for access token
async fn handle_oauth_exchange(
    params: OAuthCallback,
    app_state: AppState,
    headers: axum::http::HeaderMap,
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract cookie from request headers
    let cookie_value = headers
        .get(header::COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            // Parse cookies and find miro_oauth_state
            cookies.split(';')
                .map(|c| c.trim())
                .find(|c| c.starts_with("miro_oauth_state="))
                .map(|c| c.strip_prefix("miro_oauth_state=").unwrap().to_string())
        })
        .ok_or("OAuth state cookie not found")?;

    // Retrieve and validate OAuth state from cookie
    let oauth_state = app_state
        .cookie_manager
        .retrieve_and_validate(&cookie_value, &params.state)
        .map_err(|e| format!("Cookie validation failed: {}", e))?;

    // Extract PKCE verifier
    let pkce_verifier = PkceCodeVerifier::new(oauth_state.pkce_verifier);

    // Exchange code for tokens
    let tokens = app_state
        .oauth_client
        .exchange_code(params.code, pkce_verifier)
        .await?;

    // Save tokens to encrypted storage
    let token_store = app_state.token_store.write().await;
    token_store.save(&tokens)?;

    info!("OAuth tokens saved successfully");
    Ok(())
}

/// Initiate OAuth flow - creates cookie and redirects to Miro
async fn oauth_authorize(State(state): State<AppState>) -> Response {
    match handle_oauth_authorize(state).await {
        Ok(response) => response,
        Err(e) => {
            error!("OAuth authorization failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Authorization failed: {}", e),
            )
                .into_response()
        }
    }
}

/// Generate authorization URL, create cookie, and redirect
async fn handle_oauth_authorize(
    app_state: AppState,
) -> Result<Response, Box<dyn std::error::Error>> {
    // Generate authorization URL with CSRF and PKCE
    let (auth_url, csrf_token, pkce_verifier) = app_state
        .oauth_client
        .get_authorization_url()
        .map_err(|e| format!("Failed to generate auth URL: {}", e))?;

    // Create OAuth state for cookie
    let oauth_state = OAuthCookieState::new(csrf_token, pkce_verifier);

    // Create encrypted cookie
    let cookie = app_state
        .cookie_manager
        .create_cookie(oauth_state)
        .map_err(|e| format!("Failed to create cookie: {}", e))?;

    // Build redirect response with cookie
    let response = axum::response::Redirect::to(&auth_url);
    let mut response = response.into_response();

    // Set cookie header
    let cookie_header = format!("{}={}", cookie.name(), cookie.value());
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie_header.parse().unwrap(),
    );

    info!("Redirecting to Miro authorization URL with encrypted state cookie");
    Ok(response)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Create and configure the HTTP server
pub fn create_app(
    oauth_client: Arc<MiroOAuthClient>,
    token_store: Arc<RwLock<TokenStore>>,
    cookie_manager: CookieStateManager,
) -> Router {
    let state = AppState {
        oauth_client,
        token_store,
        cookie_manager,
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/callback", get(oauth_callback))
        .with_state(state)
}

/// Run the HTTP server
pub async fn run_server(
    port: u16,
    oauth_client: Arc<MiroOAuthClient>,
    token_store: Arc<RwLock<TokenStore>>,
    cookie_manager: CookieStateManager,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app(oauth_client, token_store, cookie_manager);
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("OAuth HTTP server listening on {}", addr);
    info!("OAuth callback URL: http://127.0.0.1:{}/oauth/callback", port);

    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn get_test_config() -> Config {
        Config {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3010/oauth/callback".to_string(),
            encryption_key: [0u8; 32],
            port: 3010,
        }
    }

    #[test]
    fn test_create_app() {
        let config = get_test_config();
        let oauth_client = Arc::new(MiroOAuthClient::new(&config).unwrap());
        let token_store = Arc::new(RwLock::new(TokenStore::new(config.encryption_key).unwrap()));
        let cookie_manager = CookieStateManager::from_config(config.encryption_key);

        let app = create_app(oauth_client, token_store, cookie_manager);
        assert!(std::mem::size_of_val(&app) > 0);
    }
}
