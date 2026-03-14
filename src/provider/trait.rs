use async_trait::async_trait;
use futures::Stream;

use crate::provider::error::{ProviderError, Result};
use crate::provider::message::Message;

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
}

impl ProviderResponse {
    pub fn new(content: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            model: model.into(),
            usage: Usage::default(),
            finish_reason: None,
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
    ) -> Result<ProviderResponse>;

    fn chat_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> impl Stream<Item = Result<String>> + Send;
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