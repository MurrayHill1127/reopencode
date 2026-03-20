use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::provider::config::ProviderConfig;
use crate::provider::error::{ProviderError, Result};
use crate::provider::message::Message;
use crate::provider::provider_trait::{
    Provider, ProviderResponse, ProviderToolCall, ToolDefinition, Usage,
};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_BASE_URL: &str = "https://openrouter.ai/api/v1";

pub struct OpenRouterProvider {
    config: ProviderConfig,
    client: Client,
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ProviderToolCall>,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterChoice {
    message: Option<OpenRouterMessageResponse>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterMessageResponse {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenRouterToolCall>,
}

#[derive(Deserialize)]
struct OpenRouterToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenRouterToolCallFunction,
}

#[derive(Deserialize)]
struct OpenRouterToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenRouterStreamResponse {
    choices: Vec<OpenRouterStreamChoice>,
}

#[derive(Deserialize)]
struct OpenRouterStreamChoice {
    delta: OpenRouterDelta,
}

#[derive(Deserialize)]
struct OpenRouterDelta {
    content: Option<String>,
}

impl OpenRouterProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL)
    }

    fn normalize_model(model: &str) -> String {
        if model.contains('/') {
            model.to_string()
        } else {
            format!("openai/{}", model)
        }
    }

    fn convert_messages(messages: Vec<Message>) -> Vec<OpenRouterMessage> {
        messages
            .into_iter()
            .map(|m| OpenRouterMessage {
                role: m.role.to_string(),
                content: m.content,
                tool_call_id: m.tool_call_id,
                tool_calls: m.tool_calls,
            })
            .collect()
    }

    fn parse_stream_line(line: &str) -> Option<String> {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                return None;
            }
            if let Ok(response) = serde_json::from_str::<OpenRouterStreamResponse>(data)
                && let Some(choice) = response.choices.first()
            {
                return choice.delta.content.clone();
            }
        }
        None
    }
}

#[async_trait]
impl Provider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        let normalized_model = Self::normalize_model(model);
        info!("调用 OpenRouter API, 模型: {}", normalized_model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = format!("{}/chat/completions", self.get_base_url());

        let request = OpenRouterRequest {
            model: normalized_model.clone(),
            messages: Self::convert_messages(messages),
            temperature,
            max_tokens,
            stream: None,
            tools: tools.to_vec(),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://opencode.ai/")
            .header("X-Title", "opencode")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.as_u16() == 429 {
            error!("速率限制");
            return Err(ProviderError::RateLimit);
        }
        if status.as_u16() == 401 {
            error!("认证失败");
            return Err(ProviderError::Authentication);
        }
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("API 错误: {}", error_text);
            return Err(ProviderError::Api(error_text));
        }

        let json: OpenRouterResponse = response.json().await?;

        let choice = json
            .choices
            .first()
            .ok_or_else(|| ProviderError::Api("No choices in response".to_string()))?;

        let content = choice
            .message
            .as_ref()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        let tool_calls: Vec<ProviderToolCall> = choice
            .message
            .as_ref()
            .map(|m| {
                m.tool_calls
                    .iter()
                    .map(|tc| ProviderToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: crate::provider::provider_trait::ProviderToolCallFunction {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            })
            .unwrap_or_default();

        let usage = json
            .usage
            .map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_default();

        Ok(ProviderResponse {
            content,
            model: json.model.unwrap_or_else(|| normalized_model),
            usage,
            finish_reason: choice.finish_reason.clone(),
            tool_calls,
        })
    }

    fn chat_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Pin<Box<dyn futures::Stream<Item = Result<String>> + Send>> {
        let url = format!("{}/chat/completions", self.get_base_url());
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let openrouter_messages = Self::convert_messages(messages);
        let normalized_model = Self::normalize_model(model);
        let tools = tools.to_vec();

        let request = OpenRouterRequest {
            model: normalized_model.clone(),
            messages: openrouter_messages,
            temperature,
            max_tokens,
            stream: Some(true),
            tools,
        };

        Box::pin(async_stream::stream! {
            info!("调用 OpenRouter API (流式), 模型: {}", normalized_model);

            let response = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .header("HTTP-Referer", "https://opencode.ai/")
                .header("X-Title", "opencode")
                .json(&request)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(ProviderError::Network(e));
                    return;
                }
            };

            if !response.status().is_success() {
                yield Err(ProviderError::Api(format!("HTTP {}", response.status())));
                return;
            }

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(ProviderError::Network(e));
                        continue;
                    }
                };

                let text = match std::str::from_utf8(&chunk) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                buffer.push_str(text);

                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].to_string();
                    buffer = buffer[pos + 1..].to_string();

                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    if let Some(content) = Self::parse_stream_line(line) {
                        yield Ok(content);
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openrouter_provider_new() {
        let config = ProviderConfig::new("openrouter", "sk-or-test");
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.name(), "openrouter");
    }

    #[test]
    fn test_openrouter_base_url() {
        let config = ProviderConfig::new("openrouter", "sk-or-test");
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.get_base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_openrouter_custom_base_url() {
        let config = ProviderConfig::new("openrouter", "sk-or-test")
            .with_base_url("https://custom.openrouter.com/v1");
        let provider = OpenRouterProvider::new(config);
        assert_eq!(provider.get_base_url(), "https://custom.openrouter.com/v1");
    }

    #[test]
    fn test_openrouter_request_serialization() {
        let request = OpenRouterRequest {
            model: "openai/gpt-4".to_string(),
            messages: vec![OpenRouterMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                tool_call_id: None,
                tool_calls: vec![],
            }],
            temperature: 0.7,
            max_tokens: Some(100),
            stream: None,
            tools: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"openai/gpt-4\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
    }

    #[test]
    fn test_openrouter_response_deserialization() {
        let json = r#"{
            "choices": [{
                "message": {"content": "Hello!"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            },
            "model": "openai/gpt-4"
        }"#;

        let response: OpenRouterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.as_ref().unwrap().content,
            Some("Hello!".to_string())
        );
        assert_eq!(response.usage.unwrap().total_tokens, 15);
    }

    #[test]
    fn test_openrouter_normalize_model() {
        // Without "/" - should prepend "openai/"
        assert_eq!(
            OpenRouterProvider::normalize_model("gpt-4"),
            "openai/gpt-4"
        );
        assert_eq!(
            OpenRouterProvider::normalize_model("claude-3-opus"),
            "openai/claude-3-opus"
        );

        // With "/" - should remain unchanged
        assert_eq!(
            OpenRouterProvider::normalize_model("anthropic/claude-3-opus"),
            "anthropic/claude-3-opus"
        );
        assert_eq!(
            OpenRouterProvider::normalize_model("openai/gpt-4"),
            "openai/gpt-4"
        );
        assert_eq!(
            OpenRouterProvider::normalize_model("meta-llama/llama-3-70b"),
            "meta-llama/llama-3-70b"
        );
    }
}