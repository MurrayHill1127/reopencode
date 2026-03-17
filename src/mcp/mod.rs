//! MCP (Model Context Protocol) module
//!
//! Provides MCP client functionality using the official rmcp SDK.
//! Supports both local (stdio) and remote (HTTP/SSE) MCP server connections.

pub mod client;
pub mod error;
pub mod manager;
pub mod types;

pub use client::McpClient;
pub use error::{McpError, Result};
pub use manager::McpManager;
pub use types::{
    McpStatus, McpTool, McpToolResult, McpResource, McpPrompt,
    McpContent, McpServerInfo, McpServerCapabilities,
    McpAddRequest, McpConfigRequest, AuthStartResponse, AuthCallback, RemoveAuthResponse,
};