//! Agent system - AI agents that orchestrate tasks

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error};

use crate::provider::{Message as ProviderMessage, MessageRole as ProviderMessageRole, Provider};

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

impl From<Role> for ProviderMessageRole {
    fn from(role: Role) -> Self {
        match role {
            Role::System => ProviderMessageRole::System,
            Role::User => ProviderMessageRole::User,
            Role::Assistant => ProviderMessageRole::Assistant,
        }
    }
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
    provider: Arc<dyn Provider>,
}

impl Sisyphus {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self {
            config: AgentConfig {
                name: "sisyphus".to_string(),
                model: "anthropic/claude-opus-4-6".to_string(),
                temperature: 0.7,
                max_tokens: Some(4096),
            },
            provider,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = model.into();
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.config.temperature = temperature;
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.config.max_tokens = max_tokens;
        self
    }

    fn convert_messages(messages: Vec<Message>) -> Vec<ProviderMessage> {
        messages
            .into_iter()
            .map(|m| ProviderMessage::new(m.role.into(), m.content))
            .collect()
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
        messages: Vec<Message>,
        _tools: Vec<ToolDefinition>,
    ) -> Result<AgentResponse, AgentError> {
        debug!("Sisyphus executing with {} messages", messages.len());

        let provider_messages = Self::convert_messages(messages);
        
        let response = self
            .provider
            .chat(
                provider_messages,
                &self.config.model,
                self.config.temperature,
                self.config.max_tokens,
            )
            .await
            .map_err(|e| {
                error!("Provider error: {}", e);
                AgentError::Provider(e.to_string())
            })?;

        debug!("Provider response: {} tokens used", response.usage.total_tokens);

        Ok(AgentResponse {
            content: response.content,
            tool_calls: vec![],
            usage: Usage {
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                total_tokens: response.usage.total_tokens,
            },
        })
    }
}

impl Default for Sisyphus {
    fn default() -> Self {
        panic!("Sisyphus requires a provider. Use Sisyphus::new(provider) instead.");
    }
}
