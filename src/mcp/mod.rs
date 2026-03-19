//! MCP (Model Context Protocol) module
//!
//! Provides MCP client functionality using the official rmcp SDK.
//! Supports both local (stdio) and remote (HTTP/SSE) MCP server connections.

pub mod client;
pub mod error;
pub mod manager;
pub mod types;

pub use manager::McpManager;
pub use types::{
    AuthCallback, AuthStartResponse, McpAddRequest, McpConfigRequest, McpStatus, RemoveAuthResponse,
};
