//! MCP client implementation using rmcp SDK
//!
//! Provides MCP client functionality for connecting to both local (stdio)
//! and remote (HTTP/SSE) MCP servers.

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use rmcp::{
    model::{
        CallToolRequestParams, GetPromptRequestParams, PromptMessageContent,
        ReadResourceRequestParams, ResourceContents,
    },
    transport::{StreamableHttpClientTransport, TokioChildProcess},
    RoleClient, ServiceExt,
};
use tokio::process::Command;
use tracing::{debug, info, warn};

use crate::config::{McpConfig, McpLocalConfig, McpRemoteConfig};

use super::error::{McpError, Result};
use super::types::{
    McpContent, McpPrompt, McpPromptArgument, McpResource, McpResourceContent, McpStatus,
    McpTool, McpToolResult,
};

type McpService = rmcp::service::RunningService<RoleClient, ()>;

pub struct McpClient {
    name: String,
    status: McpStatus,
    service: Option<McpService>,
}

impl McpClient {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: McpStatus::Disabled,
            service: None,
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
        info!(
            "MCP client '{}' connecting via stdio: {} {:?}",
            self.name, config.command, config.args
        );

        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);

        if let Some(env) = &config.env {
            cmd.envs(env);
        }

        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
        }

        cmd.stderr(Stdio::inherit());

        let transport = TokioChildProcess::new(cmd)
            .map_err(|e| McpError::ProcessSpawnFailed(e.to_string()))?;

        match ().serve(transport).await {
            Ok(service) => {
                self.service = Some(service);
                self.status = McpStatus::Connected;
                info!("MCP client '{}' connected successfully via stdio", self.name);
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                warn!(
                    "MCP client '{}' stdio connection failed: {}",
                    self.name, error_msg
                );
                self.status = McpStatus::Failed {
                    error: error_msg.clone(),
                };
                Err(McpError::ConnectionFailed(error_msg))
            }
        }
    }

    pub async fn connect_http(&mut self, config: &McpRemoteConfig) -> Result<()> {
        info!(
            "MCP client '{}' connecting via HTTP: {}",
            self.name, config.url
        );

        let uri: Arc<str> = Arc::from(config.url.as_str());
        let transport = StreamableHttpClientTransport::from_uri(uri);

        match ().serve(transport).await {
            Ok(service) => {
                self.service = Some(service);
                self.status = McpStatus::Connected;
                info!(
                    "MCP client '{}' connected successfully via HTTP",
                    self.name
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("401") || error_msg.contains("Unauthorized") {
                    self.status = McpStatus::NeedsAuth;
                    return Err(McpError::AuthRequired);
                }
                warn!(
                    "MCP client '{}' HTTP connection failed: {}",
                    self.name, error_msg
                );
                self.status = McpStatus::Failed {
                    error: error_msg.clone(),
                };
                Err(McpError::ConnectionFailed(error_msg))
            }
        }
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(service) = self.service.take() {
            let _ = service.cancel().await;
            info!("MCP client '{}' disconnected", self.name);
        }
        self.status = McpStatus::Disabled;
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let tools = service
            .list_all_tools()
            .await
            .map_err(|e| McpError::ToolCallFailed(e.to_string()))?;

        let result: Vec<McpTool> = tools
            .into_iter()
            .map(|tool| McpTool {
                name: tool.name.to_string(),
                description: tool.description.map(|d| d.to_string()),
                input_schema: serde_json::to_value(&tool.input_schema)
                    .unwrap_or(serde_json::json!({})),
            })
            .collect();

        debug!("MCP client '{}' listed {} tools", self.name, result.len());
        Ok(result)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<McpToolResult> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let params = if let Some(args) = arguments {
            let map = args.as_object().cloned().unwrap_or_default();
            CallToolRequestParams::new(name.to_string()).with_arguments(map)
        } else {
            CallToolRequestParams::new(name.to_string())
        };

        let result = service
            .call_tool(params)
            .await
            .map_err(|e| McpError::ToolCallFailed(e.to_string()))?;

        let content: Vec<McpContent> = result
            .content
            .into_iter()
            .filter_map(|annotated| convert_content(annotated.raw))
            .collect();

        debug!(
            "MCP client '{}' called tool '{}', got {} content items",
            self.name,
            name,
            content.len()
        );

        Ok(McpToolResult {
            content,
            is_error: result.is_error,
        })
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let resources = service
            .list_all_resources()
            .await
            .map_err(|e| McpError::ToolCallFailed(e.to_string()))?;

        let result: Vec<McpResource> = resources
            .into_iter()
            .map(|annotated| {
                let res = annotated.raw;
                McpResource {
                    name: res.name,
                    uri: res.uri,
                    description: res.description,
                    mime_type: res.mime_type,
                    client: self.name.clone(),
                }
            })
            .collect();

        debug!(
            "MCP client '{}' listed {} resources",
            self.name,
            result.len()
        );
        Ok(result)
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let prompts = service
            .list_all_prompts()
            .await
            .map_err(|e| McpError::ToolCallFailed(e.to_string()))?;

        let result: Vec<McpPrompt> = prompts
            .into_iter()
            .map(|prompt| McpPrompt {
                name: prompt.name,
                description: prompt.description,
                arguments: prompt.arguments.map(|args| {
                    args.into_iter()
                        .map(|arg| McpPromptArgument {
                            name: arg.name,
                            description: arg.description,
                            required: arg.required,
                        })
                        .collect()
                }),
                client: self.name.clone(),
            })
            .collect();

        debug!("MCP client '{}' listed {} prompts", self.name, result.len());
        Ok(result)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<McpResourceContent> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let result = service
            .read_resource(ReadResourceRequestParams::new(uri.to_string()))
            .await
            .map_err(|e| McpError::ToolCallFailed(e.to_string()))?;

        if let Some(content) = result.contents.into_iter().next() {
            match content {
                ResourceContents::TextResourceContents {
                    uri, mime_type, text, ..
                } => Ok(McpResourceContent {
                    uri,
                    mime_type,
                    text: Some(text),
                    blob: None,
                }),
                ResourceContents::BlobResourceContents {
                    uri, mime_type, blob, ..
                } => Ok(McpResourceContent {
                    uri,
                    mime_type,
                    text: None,
                    blob: Some(blob),
                }),
            }
        } else {
            Err(McpError::ToolCallFailed("No content returned".to_string()))
        }
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<String> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let params = if let Some(args) = arguments {
            let json_map: serde_json::Map<String, serde_json::Value> = args
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            GetPromptRequestParams::new(name.to_string()).with_arguments(json_map)
        } else {
            GetPromptRequestParams::new(name.to_string())
        };

        let result = service
            .get_prompt(params)
            .await
            .map_err(|e| McpError::ToolCallFailed(e.to_string()))?;

        let prompt_text: String = result
            .messages
            .into_iter()
            .filter_map(|msg| match msg.content {
                PromptMessageContent::Text { text } => {
                    Some(format!("{:?}: {}", msg.role, text))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(prompt_text)
    }

    pub async fn get_server_info(&self) -> Result<Option<super::types::McpServerInfo>> {
        let service = self
            .service
            .as_ref()
            .ok_or_else(|| McpError::ConnectionFailed("Not connected".to_string()))?;

        let info = service.peer_info();
        if let Some(info) = info {
            Ok(Some(super::types::McpServerInfo {
                name: info.server_info.name.clone(),
                version: info.server_info.version.clone(),
                protocol_version: info.protocol_version.to_string(),
                capabilities: super::types::McpServerCapabilities {
                    tools: info.capabilities.tools.clone().map(|t| {
                        super::types::McpToolsCapability {
                            list_changed: t.list_changed,
                        }
                    }),
                    resources: info.capabilities.resources.clone().map(|r| {
                        super::types::McpResourcesCapability {
                            subscribe: r.subscribe,
                            list_changed: r.list_changed,
                        }
                    }),
                    prompts: info.capabilities.prompts.clone().map(|p| {
                        super::types::McpPromptsCapability {
                            list_changed: p.list_changed,
                        }
                    }),
                },
            }))
        } else {
            Ok(None)
        }
    }
}

fn convert_content(content: rmcp::model::RawContent) -> Option<McpContent> {
    match content {
        rmcp::model::RawContent::Text(text) => Some(McpContent::Text { text: text.text }),
        rmcp::model::RawContent::Image(img) => Some(McpContent::Image {
            data: img.data,
            mime_type: img.mime_type,
        }),
        rmcp::model::RawContent::Resource(res) => {
            let resource_content = match res.resource {
                ResourceContents::TextResourceContents {
                    uri, mime_type, text, ..
                } => McpResourceContent {
                    uri,
                    mime_type,
                    text: Some(text),
                    blob: None,
                },
                ResourceContents::BlobResourceContents {
                    uri, mime_type, blob, ..
                } => McpResourceContent {
                    uri,
                    mime_type,
                    text: None,
                    blob: Some(blob),
                },
            };
            Some(McpContent::Resource {
                resource: resource_content,
            })
        }
        rmcp::model::RawContent::Audio(_) => {
            debug!("Audio content not yet supported");
            None
        }
        rmcp::model::RawContent::ResourceLink(_) => {
            debug!("ResourceLink content not yet supported");
            None
        }
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if self.service.is_some() {
            debug!("MCP client '{}' being dropped", self.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = McpClient::new("test-client");
        assert_eq!(client.name, "test-client");
        assert!(matches!(client.status(), McpStatus::Disabled));
    }

    #[test]
    fn test_status_disabled() {
        let client = McpClient::new("test");
        assert!(matches!(client.status(), McpStatus::Disabled));
    }

    #[tokio::test]
    async fn test_list_tools_not_connected() {
        let client = McpClient::new("test");
        let result = client.list_tools().await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            McpError::ConnectionFailed(_)
        ));
    }

    #[tokio::test]
    async fn test_call_tool_not_connected() {
        let client = McpClient::new("test");
        let result = client.call_tool("test", None).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            McpError::ConnectionFailed(_)
        ));
    }

    #[tokio::test]
    async fn test_list_resources_not_connected() {
        let client = McpClient::new("test");
        let result = client.list_resources().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_prompts_not_connected() {
        let client = McpClient::new("test");
        let result = client.list_prompts().await;
        assert!(result.is_err());
    }
}