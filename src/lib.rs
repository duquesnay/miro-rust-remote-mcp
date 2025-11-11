pub mod auth;
pub mod config;
pub mod http_server;
pub mod mcp;
pub mod miro;

#[cfg(feature = "oauth-proxy")]
pub mod oauth;

#[cfg(feature = "stdio-mcp")]
pub mod oauth_dcr;

pub use auth::{AuthError, TokenSet, TokenValidator, UserInfo};
pub use config::Config;
pub use http_server::{create_app_adr002, run_server_adr002};

#[cfg(feature = "stdio-mcp")]
pub use auth::TokenStore;
#[cfg(feature = "stdio-mcp")]
pub use mcp::{AuthHandler, MiroMcpServer};
#[cfg(feature = "stdio-mcp")]
pub use miro::{MiroClient, MiroError};
