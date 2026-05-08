//! MCP tool proxy — wraps MCP server tools as native Tool implementations.
//!
//! When MCP servers connect, their tools are registered into the main
//! ToolRegistry via these proxy wrappers, making them visible to the LLM.

use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

use crate::mcp::McpManager;
use crate::tool::error::Result;
use crate::tool::traits::{Tool, ToolResult};

/// A proxy that delegates Tool trait calls to an MCP server's tool.
pub struct McpToolProxy {
    /// MCP server name (e.g. "websearch")
    server_name: String,
    /// Tool name as reported by the MCP server
    tool_name: String,
    /// Human-readable description
    description: String,
    /// JSON Schema for parameters
    parameters: Value,
    /// Reference to the MCP manager for call_tool()
    manager: Arc<McpManager>,
}

impl McpToolProxy {
    pub fn new(
        server_name: String,
        tool_name: String,
        description: String,
        parameters: Value,
        manager: Arc<McpManager>,
    ) -> Self {
        Self { server_name, tool_name, description, parameters, manager }
    }
}

#[async_trait]
impl Tool for McpToolProxy {
    fn name(&self) -> &str { &self.tool_name }

    fn description(&self) -> &str { &self.description }

    fn parameters(&self) -> Value { self.parameters.clone() }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        match self.manager.call_tool(&self.server_name, &self.tool_name, Some(args)).await {
            Ok(result) => {
                let text: String = result.content.iter()
                    .filter_map(|c| match c {
                        crate::mcp::types::McpContent::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::new(text))
            }
            Err(e) => Ok(ToolResult::new(format!("MCP error: {e}"))),
        }
    }
}

/// Register all MCP server tools into the given ToolRegistry.
/// Called during server startup and when new MCP servers connect.
pub async fn register_mcp_tools(
    manager: &Arc<McpManager>,
    registry: &crate::tool::registry::ToolRegistry,
) {
    let all_tools = manager.list_tools().await;
    for (server_name, tools) in all_tools {
        let manager_ref = Arc::clone(manager);
        for tool in tools {
            let proxy = McpToolProxy::new(
                server_name.clone(),
                format!("mcp_{}_{}", server_name, tool.name),
                tool.description.unwrap_or_else(|| format!("MCP tool: {}", tool.name)),
                tool.input_schema,
                Arc::clone(&manager_ref),
            );
            registry.register(Box::new(proxy));
        }
    }
}
