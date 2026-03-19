use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::provider::config::ProviderConfig;
use crate::provider::error::{ProviderError, Result};
use crate::provider::message::{Message, MessageRole};
use crate::provider::provider_trait::{Provider, ProviderResponse, ToolDefinition, Usage};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";
const API_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    config: ProviderConfig,
    client: Client,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    model: String,
    usage: AnthropicUsage,
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    text: Option<String>,
}

impl AnthropicProvider {
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

    fn split_messages(messages: Vec<Message>) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            if msg.role == MessageRole::System {
                system = Some(msg.content);
            } else {
                anthropic_messages.push(AnthropicMessage {
                    role: msg.role.to_string(),
                    content: msg.content,
                });
            }
        }

        (system, anthropic_messages)
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        _tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        info!("调用 Anthropic API, 模型: {}", model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = format!("{}/messages", self.get_base_url());

        let (system, anthropic_messages) = Self::split_messages(messages);

        let request = AnthropicRequest {
            model: model.to_string(),
            messages: anthropic_messages,
            max_tokens: max_tokens.unwrap_or(4096),
            system,
            temperature: Some(temperature),
            stream: None,
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", API_VERSION)
            .header("Content-Type", "application/json")
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

        let json: AnthropicResponse = response.json().await?;

        let content = json
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let usage = Usage {
            prompt_tokens: json.usage.input_tokens,
            completion_tokens: json.usage.output_tokens,
            total_tokens: json.usage.input_tokens + json.usage.output_tokens,
        };

        Ok(ProviderResponse {
            content,
            model: json.model,
            usage,
            finish_reason: json.stop_reason,
            tool_calls: vec![],
        })
    }

    fn chat_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        _tools: &[ToolDefinition],
    ) -> Pin<Box<dyn futures::Stream<Item = Result<String>> + Send>> {
        let url = format!("{}/messages", self.get_base_url());
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let (system, anthropic_messages) = Self::split_messages(messages);
        let model = model.to_string();

        let request = AnthropicRequest {
            model: model.clone(),
            messages: anthropic_messages,
            max_tokens: max_tokens.unwrap_or(4096),
            system,
            temperature: Some(temperature),
            stream: Some(true),
        };

        Box::pin(async_stream::stream! {
            info!("调用 Anthropic API (流式), 模型: {}", model);

            let response = match client
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", API_VERSION)
                .header("Content-Type", "application/json")
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
                    if line.is_empty() || !line.starts_with("data: ") {
                        continue;
                    }

                    let data = &line[6..];
                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data)
                        && event.event_type == "content_block_delta"
                        && let Some(delta) = event.delta
                        && let Some(text) = delta.text
                    {
                        yield Ok(text);
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
    fn test_anthropic_provider_new() {
        let config = ProviderConfig::new("anthropic", "sk-ant-test");
        let provider = AnthropicProvider::new(config);
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_anthropic_provider_base_url() {
        let config = ProviderConfig::new("anthropic", "sk-ant-test");
        let provider = AnthropicProvider::new(config);
        assert_eq!(provider.get_base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_anthropic_request_serialization() {
        let request = AnthropicRequest {
            model: "claude-3-opus".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            max_tokens: 100,
            system: Some("You are helpful".to_string()),
            temperature: Some(0.7),
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"claude-3-opus\""));
        assert!(json.contains("\"max_tokens\":100"));
        assert!(json.contains("\"system\":\"You are helpful\""));
    }

    #[test]
    fn test_anthropic_response_deserialization() {
        let json = r#"{
            "content": [{"text": "Hello!"}],
            "model": "claude-3-opus",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            },
            "stop_reason": "end_turn"
        }"#;

        let response: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.content.len(), 1);
        assert_eq!(response.content[0].text, "Hello!");
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
        assert_eq!(response.stop_reason, Some("end_turn".to_string()));
    }

    #[test]
    fn test_split_messages() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
            Message::assistant("Hi there"),
        ];

        let (system, anthropic_messages) = AnthropicProvider::split_messages(messages);
        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(anthropic_messages.len(), 2);
        assert_eq!(anthropic_messages[0].role, "user");
        assert_eq!(anthropic_messages[1].role, "assistant");
    }
}
