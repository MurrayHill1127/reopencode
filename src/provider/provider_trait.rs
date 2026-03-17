use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::provider::error::Result;
use crate::provider::message::Message;

/// Tool definition for providers (mirrors agent::ToolDefinition for API use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Tool call returned by the provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// The type of tool (always "function" for now)
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function call details
    pub function: ProviderToolCallFunction,
}

/// Function details within a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderToolCallFunction {
    /// Name of the function to call
    pub name: String,
    /// JSON-encoded arguments string
    pub arguments: String,
}

impl From<&crate::agent::ToolDefinition> for ToolDefinition {
    fn from(td: &crate::agent::ToolDefinition) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: td.name.clone(),
                description: td.description.clone(),
                parameters: td.parameters.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }
    }
}

#[derive(Debug)]
pub struct ProviderResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
    pub finish_reason: Option<String>,
    pub tool_calls: Vec<ProviderToolCall>,
}

impl ProviderResponse {
    pub fn new(content: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            model: model.into(),
            usage: Usage::default(),
            finish_reason: None,
            tool_calls: vec![],
        }
    }

    pub fn with_usage(mut self, usage: Usage) -> Self {
        self.usage = usage;
        self
    }

    pub fn with_finish_reason(mut self, reason: impl Into<String>) -> Self {
        self.finish_reason = Some(reason.into());
        self
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ProviderToolCall>) -> Self {
        self.tool_calls = tool_calls;
        self
    }

    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse>;

    fn chat_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Pin<Box<dyn Stream<Item = Result<String>> + Send>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_provider_response_new() {
        let response = ProviderResponse::new("Hello", "gpt-4");
        assert_eq!(response.content, "Hello");
        assert_eq!(response.model, "gpt-4");
        assert!(response.finish_reason.is_none());
    }

    #[test]
    fn test_provider_response_with_usage() {
        let usage = Usage {
            prompt_tokens: 10,
            completion_tokens: 20,
            total_tokens: 30,
        };
        let response = ProviderResponse::new("Hello", "gpt-4").with_usage(usage);
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 20);
        assert_eq!(response.usage.total_tokens, 30);
    }

    #[test]
    fn test_provider_response_with_finish_reason() {
        let response = ProviderResponse::new("Hello", "gpt-4")
            .with_finish_reason("stop");
        assert_eq!(response.finish_reason, Some("stop".to_string()));
    }
}