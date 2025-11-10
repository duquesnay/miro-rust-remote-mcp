use crate::auth::{MiroOAuthClient, TokenStore};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
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
}

/// Handle OAuth callback from Miro
async fn oauth_callback(
    Query(params): Query<OAuthCallback>,
    State(state): State<AppState>,
) -> Response {
    info!("Received OAuth callback with code");

    match handle_oauth_exchange(params, state).await {
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
    state: AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    // Exchange code for tokens
    let tokens = state
        .oauth_client
        .exchange_code(params.code, params.state)
        .await?;

    // Save tokens to encrypted storage
    let token_store = state.token_store.write().await;
    token_store.save(&tokens)?;

    info!("OAuth tokens saved successfully");
    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Create and configure the HTTP server
pub fn create_app(oauth_client: Arc<MiroOAuthClient>, token_store: Arc<RwLock<TokenStore>>) -> Router {
    let state = AppState {
        oauth_client,
        token_store,
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/oauth/callback", get(oauth_callback))
        .with_state(state)
}

/// Run the HTTP server
pub async fn run_server(
    port: u16,
    oauth_client: Arc<MiroOAuthClient>,
    token_store: Arc<RwLock<TokenStore>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app(oauth_client, token_store);
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

        let app = create_app(oauth_client, token_store);
        assert!(std::mem::size_of_val(&app) > 0);
    }
}
