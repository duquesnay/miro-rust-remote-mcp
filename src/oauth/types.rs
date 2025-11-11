use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Data stored in encrypted cookies for OAuth state management
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CookieData {
    /// Miro access token
    pub access_token: String,

    /// Miro refresh token
    pub refresh_token: String,

    /// Token expiration timestamp
    pub expires_at: DateTime<Utc>,

    /// User information from Miro
    pub user_info: UserInfo,
}

/// User information from Miro
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserInfo {
    /// Miro user ID
    pub user_id: String,

    /// User's email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// User's display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// OAuth state stored temporarily during authorization flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    /// CSRF protection nonce
    pub state: String,

    /// PKCE code verifier (stored to validate challenge)
    pub code_verifier: String,

    /// Redirect URI after OAuth completion
    pub redirect_uri: String,
}

/// PKCE code verifier and challenge pair
#[derive(Debug, Clone)]
pub struct PkcePair {
    /// Code verifier (random secret)
    pub verifier: String,

    /// Code challenge (SHA256 hash of verifier)
    pub challenge: String,
}

/// Token response from Miro OAuth endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,

    /// User information from Miro (included in token response)
    #[serde(flatten)]
    pub user: Option<MiroUser>,
}

/// Miro user information from token endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct MiroUser {
    pub user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl From<MiroUser> for UserInfo {
    fn from(miro_user: MiroUser) -> Self {
        UserInfo {
            user_id: miro_user.user_id,
            email: miro_user.email,
            name: miro_user.name,
        }
    }
}
