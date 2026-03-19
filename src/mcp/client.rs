//! MCP client implementation using rmcp SDK
//!
//! Note: This is a stub implementation. Full MCP client support will be added
//! once the rmcp API stabilizes.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::config::{McpConfig, McpLocalConfig, McpRemoteConfig};

use super::error::{McpError, Result};
use super::types::{McpContent, McpResourceContent, McpStatus, McpTool, McpToolResult};

/// MCP client stub
pub struct McpClient {
    name: String,
    status: McpStatus,
}

impl McpClient {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: McpStatus::Disabled,
        }
    }

    pub fn status(&self) -> &McpStatus {
        &self.status
    }

    pub async fn connect(&mut self, config: &McpConfig) -> Result<()> {
        match config {
            McpConfig::Local(local) => self.connect_stdio(local).await,
            McpConfig::Remote(remote) => self.connect_http(remote).await,
        }
    }

    pub async fn connect_stdio(&mut self, config: &McpLocalConfig) -> Result<()> {
        tracing::info!(
            "MCP client '{}' would connect via stdio: {:?}",
            self.name,
            config.command
        );
        self.status = McpStatus::Failed {
            error: "MCP client not yet implemented".to_string(),
        };
        Err(McpError::ConnectionFailed(
            "MCP client stub - not implemented".to_string(),
        ))
    }

    pub async fn connect_http(&mut self, config: &McpRemoteConfig) -> Result<()> {
        tracing::info!(
            "MCP client '{}' would connect via HTTP: {}",
            self.name,
            config.url
        );
        self.status = McpStatus::Failed {
            error: "MCP client not yet implemented".to_string(),
        };
        Err(McpError::ConnectionFailed(
            "MCP client stub - not implemented".to_string(),
        ))
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        self.status = McpStatus::Disabled;
        tracing::info!("MCP client '{}' disconnected", self.name);
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        Err(McpError::ConnectionFailed("Not connected".to_string()))
    }

    pub async fn call_tool(
        &self,
        _name: &str,
        _arguments: Option<serde_json::Value>,
    ) -> Result<McpToolResult> {
        Err(McpError::ConnectionFailed("Not connected".to_string()))
    }

    pub async fn list_resources(&self) -> Result<Vec<super::types::McpResource>> {
        Err(McpError::ConnectionFailed("Not connected".to_string()))
    }

    pub async fn list_prompts(&self) -> Result<Vec<super::types::McpPrompt>> {
        Err(McpError::ConnectionFailed("Not connected".to_string()))
    }
}
