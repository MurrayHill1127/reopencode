//! Agent system - AI agents that orchestrate tasks

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

/// Tool definition for agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Tool call from agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Agent response
#[derive(Debug)]
pub struct AgentResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
}

/// Agent error
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("Model error: {0}")]
    Model(String),
    
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// Agent trait - all agents must implement this
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent name
    fn name(&self) -> &str;
    
    /// Get agent configuration
    fn config(&self) -> &AgentConfig;
    
    /// Execute the agent with messages and tools
    async fn execute(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
    ) -> Result<AgentResponse, AgentError>;
}

/// Sisyphus - the main orchestrator agent
pub struct Sisyphus {
    config: AgentConfig,
}

impl Sisyphus {
    pub fn new() -> Self {
        Self {
            config: AgentConfig {
                name: "sisyphus".to_string(),
                model: "anthropic/claude-opus-4-6".to_string(),
                temperature: 0.7,
                max_tokens: Some(4096),
            },
        }
    }
}

#[async_trait]
impl Agent for Sisyphus {
    fn name(&self) -> &str {
        "sisyphus"
    }
    
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    async fn execute(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<AgentResponse, AgentError> {
        // TODO: Implement actual execution
        Ok(AgentResponse {
            content: "Sisyphus agent ready".to_string(),
            tool_calls: vec![],
            usage: Usage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        })
    }
}

impl Default for Sisyphus {
    fn default() -> Self {
        Self::new()
    }
}
