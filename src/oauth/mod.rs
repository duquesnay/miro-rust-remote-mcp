//! OAuth2 state management and PKCE utilities for Miro authentication

#[cfg(feature = "stdio-mcp")]
pub mod cookie_manager;
pub mod endpoints;
pub mod pkce;
pub mod proxy_provider;
pub mod types;

#[cfg(feature = "stdio-mcp")]
pub use cookie_manager::{CookieError, CookieManager};
pub use endpoints::*;
pub use pkce::*;
pub use proxy_provider::*;
pub use types::*;
