//! MCP error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool call failed: {0}")]
    ToolCallFailed(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Client registration required: {0}")]
    ClientRegistrationRequired(String),

    #[error("Timeout")]
    Timeout,

    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, McpError>;
