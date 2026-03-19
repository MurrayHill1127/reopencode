//! MCP client manager for handling multiple MCP servers

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::RwLock;

use crate::config::{Config, McpConfig};

use super::client::McpClient;
use super::error::{McpError, Result};
use super::types::McpStatus;

pub type ClientMap = DashMap<String, Arc<RwLock<McpClient>>>;

pub struct McpManager {
    clients: ClientMap,
    statuses: RwLock<HashMap<String, McpStatus>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            clients: DashMap::new(),
            statuses: RwLock::new(HashMap::new()),
        }
    }

    pub async fn initialize_from_config(&self, config: &Config) {
        for (name, mcp_config) in &config.mcp {
            if let Err(e) = self.add(name, mcp_config.clone()).await {
                tracing::error!("Failed to initialize MCP server '{}': {}", name, e);
            }
        }
    }

    pub async fn add(&self, name: &str, config: McpConfig) -> Result<()> {
        if self.clients.contains_key(name) {
            self.remove(name).await?;
        }

        let mut client = McpClient::new(name);

        match client.connect(&config).await {
            Ok(()) => {
                tracing::info!("MCP server '{}' added and connected", name);
            }
            Err(McpError::AuthRequired) => {
                tracing::info!("MCP server '{}' requires authentication", name);
            }
            Err(e) => {
                tracing::warn!("MCP server '{}' connection failed: {}", name, e);
            }
        }

        let status = client.status().clone();
        self.statuses.write().await.insert(name.to_string(), status);
        self.clients
            .insert(name.to_string(), Arc::new(RwLock::new(client)));

        Ok(())
    }

    pub async fn remove(&self, name: &str) -> Result<()> {
        if let Some((_, client)) = self.clients.remove(name) {
            let mut client = client.write().await;
            client.disconnect().await?;
        }
        self.statuses.write().await.remove(name);
        tracing::info!("MCP server '{}' removed", name);
        Ok(())
    }

    pub async fn connect(&self, name: &str, config: McpConfig) -> Result<()> {
        self.add(name, config).await
    }

    pub async fn disconnect(&self, name: &str) -> Result<()> {
        if let Some(client) = self.clients.get(name) {
            let mut client = client.write().await;
            client.disconnect().await?;
            self.statuses
                .write()
                .await
                .insert(name.to_string(), McpStatus::Disabled);
        }
        Ok(())
    }

    pub async fn status(&self) -> HashMap<String, McpStatus> {
        self.statuses.read().await.clone()
    }

    pub async fn get_client(&self, name: &str) -> Option<Arc<RwLock<McpClient>>> {
        self.clients.get(name).map(|c| c.clone())
    }

    pub async fn list_tools(&self) -> HashMap<String, Vec<super::types::McpTool>> {
        let mut result = HashMap::new();

        for entry in self.clients.iter() {
            let name = entry.key().clone();
            let client = entry.value().read().await;

            if matches!(client.status(), McpStatus::Connected) {
                if let Ok(tools) = client.list_tools().await {
                    result.insert(name, tools);
                }
            }
        }

        result
    }

    pub async fn call_tool(
        &self,
        client_name: &str,
        tool_name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<super::types::McpToolResult> {
        let client = self
            .clients
            .get(client_name)
            .ok_or_else(|| McpError::ToolNotFound(client_name.to_string()))?;

        let client = client.read().await;
        client.call_tool(tool_name, arguments).await
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
