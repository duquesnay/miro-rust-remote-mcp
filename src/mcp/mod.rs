#[cfg(feature = "stdio-mcp")]
pub mod auth_handler;
pub mod handlers;
pub mod metadata;
pub mod protocol;
#[cfg(feature = "stdio-mcp")]
pub mod server;
pub mod tools;

#[cfg(feature = "stdio-mcp")]
pub use auth_handler::AuthHandler;
pub use handlers::{handle_initialize, handle_tools_call, handle_tools_list};
pub use metadata::{oauth_authorization_server_metadata, oauth_metadata};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
#[cfg(feature = "stdio-mcp")]
pub use server::MiroMcpServer;
pub use tools::{get_board, list_boards};
