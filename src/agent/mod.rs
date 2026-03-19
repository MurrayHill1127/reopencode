//! Agent system - AI agents that orchestrate tasks

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, error, info};

use crate::provider::{
    Message as ProviderMessage, MessageRole as ProviderMessageRole, Provider, ProviderToolCall,
    ProviderToolCallFunction, ToolDefinition as ProviderToolDefinition,
};
use crate::tool::registry::ToolRegistry;
use crate::tool::traits::Tool;

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
    pub id: String,
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

    /// Execute with tool calling loop - continues until no more tool calls
    /// or max iterations reached
    pub async fn execute_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolDefinition>,
        tool_registry: &ToolRegistry,
        max_iterations: usize,
    ) -> Result<AgentResponse, AgentError> {
        let mut current_messages: Vec<ProviderMessage> = Self::convert_messages(messages);
        let provider_tools: Vec<ProviderToolDefinition> =
            tools.iter().map(ProviderToolDefinition::from).collect();

        let mut total_usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > max_iterations {
                debug!("Max iterations ({}) reached, stopping", max_iterations);
                break;
            }

            debug!(
                "Iteration {}: calling provider with {} messages",
                iterations,
                current_messages.len()
            );

            let response = self
                .provider
                .chat(
                    current_messages.clone(),
                    &self.config.model,
                    self.config.temperature,
                    self.config.max_tokens,
                    &provider_tools,
                )
                .await
                .map_err(|e| {
                    error!("Provider error: {}", e);
                    AgentError::Provider(e.to_string())
                })?;

            total_usage.prompt_tokens += response.usage.prompt_tokens;
            total_usage.completion_tokens += response.usage.completion_tokens;
            total_usage.total_tokens += response.usage.total_tokens;

            if !response.has_tool_calls() {
                debug!("No tool calls, returning final response");
                return Ok(AgentResponse {
                    content: response.content,
                    tool_calls: vec![],
                    usage: total_usage,
                });
            }

            debug!("Provider returned {} tool calls", response.tool_calls.len());

            let assistant_msg = ProviderMessage::assistant_with_tool_calls(
                response.content.clone(),
                response.tool_calls.clone(),
            );
            current_messages.push(assistant_msg);

            for tool_call in &response.tool_calls {
                let tool_name = &tool_call.function.name;
                let tool_args: Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or(Value::Object(serde_json::Map::new()));

                info!("Executing tool: {} with args: {:?}", tool_name, tool_args);

                let tool_result = if let Some(tool) = tool_registry.get(tool_name) {
                    match tool.execute(tool_args).await {
                        Ok(result) => result.output,
                        Err(e) => {
                            error!("Tool {} execution failed: {}", tool_name, e);
                            format!("Error: {}", e)
                        }
                    }
                } else {
                    error!("Tool not found: {}", tool_name);
                    format!("Error: Tool '{}' not found", tool_name)
                };

                debug!("Tool {} result: {}", tool_name, tool_result);

                let tool_msg = ProviderMessage::tool(tool_result, &tool_call.id);
                current_messages.push(tool_msg);
            }
        }

        Ok(AgentResponse {
            content: "Max iterations reached without final response".to_string(),
            tool_calls: vec![],
            usage: total_usage,
        })
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
        tools: Vec<ToolDefinition>,
    ) -> Result<AgentResponse, AgentError> {
        debug!("Sisyphus executing with {} messages", messages.len());

        let provider_messages = Self::convert_messages(messages);
        let provider_tools: Vec<ProviderToolDefinition> =
            tools.iter().map(ProviderToolDefinition::from).collect();

        let response = self
            .provider
            .chat(
                provider_messages,
                &self.config.model,
                self.config.temperature,
                self.config.max_tokens,
                &provider_tools,
            )
            .await
            .map_err(|e| {
                error!("Provider error: {}", e);
                AgentError::Provider(e.to_string())
            })?;

        debug!(
            "Provider response: {} tokens used",
            response.usage.total_tokens
        );

        let tool_calls: Vec<ToolCall> = response
            .tool_calls
            .into_iter()
            .map(|tc| {
                let args: Value = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(Value::Object(serde_json::Map::new()));
                ToolCall {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: args,
                }
            })
            .collect();

        Ok(AgentResponse {
            content: response.content,
            tool_calls,
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
