//! MCP types and data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum McpStatus {
    Connected,
    Disabled,
    Failed { error: String },
    NeedsAuth,
    NeedsClientRegistration { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub name: String,
    pub uri: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub client: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<McpPromptArgument>>,
    pub client: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResourceContent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
    pub capabilities: McpServerCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    pub tools: Option<McpToolsCapability>,
    pub resources: Option<McpResourcesCapability>,
    pub prompts: Option<McpPromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct McpAddRequest {
    pub name: String,
    pub config: McpConfigRequest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpConfigRequest {
    Local {
        command: Vec<String>,
        #[serde(default)]
        environment: Option<HashMap<String, String>>,
        #[serde(default)]
        timeout: Option<u64>,
    },
    Remote {
        url: String,
        #[serde(default)]
        headers: Option<HashMap<String, String>>,
        #[serde(default)]
        timeout: Option<u64>,
    },
}

#[derive(Debug, Serialize)]
pub struct AuthStartResponse {
    pub authorization_url: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthCallback {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RemoveAuthResponse {
    pub success: bool,
}