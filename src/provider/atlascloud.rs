//! Atlas Cloud provider (OpenAI-compatible)
//!
//! Atlas Cloud exposes a unified OpenAI-compatible endpoint
//! (`https://api.atlascloud.ai/v1`) that provides access to many models
//! (Claude, GPT, Gemini, DeepSeek, Qwen, Kimi, ...). Because the API mirrors
//! the OpenAI chat-completions contract, this provider reuses the same
//! request/response shape as the other OpenAI-compatible providers.

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

const DEFAULT_TIMEOUT_SECS: u64 = 60; // Longer timeout for reasoning models
const DEFAULT_BASE_URL: &str = "https://api.atlascloud.ai/v1";
/// Default model: DeepSeek V4 Pro reasoning model served via Atlas Cloud.
pub const DEFAULT_MODEL: &str = "deepseek-ai/deepseek-v4-pro";

pub struct AtlasCloudProvider {
    config: ProviderConfig,
    client: Client,
}

#[derive(Serialize)]
struct AtlasRequest {
    model: String,
    messages: Vec<AtlasMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize)]
struct AtlasMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ProviderToolCall>,
}

#[derive(Deserialize)]
struct AtlasResponse {
    choices: Vec<AtlasChoice>,
    usage: Option<AtlasUsage>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct AtlasChoice {
    message: Option<AtlasMessageResponse>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct AtlasMessageResponse {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<AtlasToolCall>,
}

#[derive(Deserialize)]
struct AtlasToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: AtlasToolCallFunction,
}

#[derive(Deserialize)]
struct AtlasToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct AtlasUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct AtlasStreamResponse {
    choices: Vec<AtlasStreamChoice>,
}

#[derive(Deserialize)]
struct AtlasStreamChoice {
    delta: AtlasDelta,
}

#[derive(Deserialize)]
struct AtlasDelta {
    content: Option<String>,
}

impl AtlasCloudProvider {
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

    /// Resolve the model to use, falling back to the Atlas Cloud default.
    fn resolve_model(model: &str) -> String {
        if model.is_empty() {
            DEFAULT_MODEL.to_string()
        } else {
            model.to_string()
        }
    }

    fn convert_messages(messages: Vec<Message>) -> Vec<AtlasMessage> {
        messages
            .into_iter()
            .map(|m| AtlasMessage {
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
            if let Ok(response) = serde_json::from_str::<AtlasStreamResponse>(data)
                && let Some(choice) = response.choices.first()
            {
                return choice.delta.content.clone();
            }
        }
        None
    }
}

#[async_trait]
impl Provider for AtlasCloudProvider {
    fn name(&self) -> &str {
        "atlascloud"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        let model = Self::resolve_model(model);
        info!("调用 Atlas Cloud API, 模型: {}", model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = format!("{}/chat/completions", self.get_base_url());

        let request = AtlasRequest {
            model: model.clone(),
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

        let json: AtlasResponse = response.json().await?;

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
            model: json.model.unwrap_or(model),
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
        let atlas_messages = Self::convert_messages(messages);
        let model = Self::resolve_model(model);
        let tools = tools.to_vec();

        let request = AtlasRequest {
            model: model.clone(),
            messages: atlas_messages,
            temperature,
            max_tokens,
            stream: Some(true),
            tools,
        };

        Box::pin(async_stream::stream! {
            info!("调用 Atlas Cloud API (流式), 模型: {}", model);

            let response = match client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
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
    fn test_atlascloud_provider_new() {
        let config = ProviderConfig::new("atlascloud", "apikey-test");
        let provider = AtlasCloudProvider::new(config);
        assert_eq!(provider.name(), "atlascloud");
    }

    #[test]
    fn test_atlascloud_base_url() {
        let config = ProviderConfig::new("atlascloud", "apikey-test");
        let provider = AtlasCloudProvider::new(config);
        assert_eq!(provider.get_base_url(), DEFAULT_BASE_URL);
        assert_eq!(DEFAULT_BASE_URL, "https://api.atlascloud.ai/v1");
    }

    #[test]
    fn test_atlascloud_custom_base_url() {
        let config = ProviderConfig::new("atlascloud", "apikey-test")
            .with_base_url("https://custom.atlas.api/v1");
        let provider = AtlasCloudProvider::new(config);
        assert_eq!(provider.get_base_url(), "https://custom.atlas.api/v1");
    }

    #[test]
    fn test_atlascloud_resolve_model_default() {
        assert_eq!(
            AtlasCloudProvider::resolve_model(""),
            "deepseek-ai/deepseek-v4-pro"
        );
        assert_eq!(DEFAULT_MODEL, "deepseek-ai/deepseek-v4-pro");
    }

    #[test]
    fn test_atlascloud_resolve_model_explicit() {
        assert_eq!(
            AtlasCloudProvider::resolve_model("anthropic/claude-3-opus"),
            "anthropic/claude-3-opus"
        );
    }

    #[test]
    fn test_atlascloud_request_serialization() {
        let request = AtlasRequest {
            model: DEFAULT_MODEL.to_string(),
            messages: vec![AtlasMessage {
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
        assert!(json.contains("\"model\":\"deepseek-ai/deepseek-v4-pro\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
    }

    #[test]
    fn test_atlascloud_response_deserialization() {
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
            "model": "deepseek-ai/deepseek-v4-pro"
        }"#;

        let response: AtlasResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.as_ref().unwrap().content,
            Some("Hello!".to_string())
        );
        assert_eq!(response.usage.unwrap().total_tokens, 15);
    }
}
