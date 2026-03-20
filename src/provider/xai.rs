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
const DEFAULT_BASE_URL: &str = "https://api.x.ai/v1";

pub struct XaiProvider {
    config: ProviderConfig,
    client: Client,
}

#[derive(Serialize)]
struct XaiRequest {
    model: String,
    messages: Vec<XaiMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize)]
struct XaiMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ProviderToolCall>,
}

#[derive(Deserialize)]
struct XaiResponse {
    choices: Vec<XaiChoice>,
    usage: Option<XaiUsage>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct XaiChoice {
    message: Option<XaiMessageResponse>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct XaiMessageResponse {
    content: Option<String>,
    /// xAI-specific: may contain refusal message
    #[serde(default)]
    refusal: Option<String>,
    #[serde(default)]
    tool_calls: Vec<XaiToolCall>,
}

#[derive(Deserialize)]
struct XaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: XaiToolCallFunction,
}

#[derive(Deserialize)]
struct XaiToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct XaiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    /// xAI-specific: detailed token breakdown
    #[serde(default)]
    prompt_tokens_details: Option<XaiPromptTokensDetails>,
}

/// xAI-specific: detailed prompt token breakdown
#[derive(Deserialize, Debug)]
struct XaiPromptTokensDetails {
    #[serde(default)]
    text_tokens: Option<u32>,
    #[serde(default)]
    audio_tokens: Option<u32>,
    #[serde(default)]
    image_tokens: Option<u32>,
    #[serde(default)]
    cached_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct XaiStreamResponse {
    choices: Vec<XaiStreamChoice>,
}

#[derive(Deserialize)]
struct XaiStreamChoice {
    delta: XaiDelta,
}

#[derive(Deserialize)]
struct XaiDelta {
    content: Option<String>,
}

impl XaiProvider {
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

    fn convert_messages(messages: Vec<Message>) -> Vec<XaiMessage> {
        messages
            .into_iter()
            .map(|m| XaiMessage {
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
            if let Ok(response) = serde_json::from_str::<XaiStreamResponse>(data)
                && let Some(choice) = response.choices.first()
            {
                return choice.delta.content.clone();
            }
        }
        None
    }
}

#[async_trait]
impl Provider for XaiProvider {
    fn name(&self) -> &str {
        "xai"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        info!("调用 xAI API, 模型: {}", model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = format!("{}/chat/completions", self.get_base_url());

        let request = XaiRequest {
            model: model.to_string(),
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

        let json: XaiResponse = response.json().await?;

        let choice = json
            .choices
            .first()
            .ok_or_else(|| ProviderError::Api("No choices in response".to_string()))?;

        // Handle xAI-specific: use refusal as content if content is null
        let content = choice
            .message
            .as_ref()
            .and_then(|m| {
                m.content.clone().or_else(|| m.refusal.clone())
            })
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
            model: json.model.unwrap_or_else(|| model.to_string()),
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
        let xai_messages = Self::convert_messages(messages);
        let model = model.to_string();
        let tools = tools.to_vec();

        let request = XaiRequest {
            model: model.clone(),
            messages: xai_messages,
            temperature,
            max_tokens,
            stream: Some(true),
            tools,
        };

        Box::pin(async_stream::stream! {
            info!("调用 xAI API (流式), 模型: {}", model);

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
    fn test_xai_provider_new() {
        let config = ProviderConfig::new("xai", "xai-test-key");
        let provider = XaiProvider::new(config);
        assert_eq!(provider.name(), "xai");
    }

    #[test]
    fn test_xai_base_url() {
        let config = ProviderConfig::new("xai", "xai-test-key");
        let provider = XaiProvider::new(config);
        assert_eq!(provider.get_base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_xai_custom_base_url() {
        let config =
            ProviderConfig::new("xai", "xai-test-key").with_base_url("https://custom.xai.api/v1");
        let provider = XaiProvider::new(config);
        assert_eq!(provider.get_base_url(), "https://custom.xai.api/v1");
    }

    #[test]
    fn test_xai_request_serialization() {
        let request = XaiRequest {
            model: "grok-beta".to_string(),
            messages: vec![XaiMessage {
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
        assert!(json.contains("\"model\":\"grok-beta\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
    }

    #[test]
    fn test_xai_response_deserialization() {
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
            "model": "grok-beta"
        }"#;

        let response: XaiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.as_ref().unwrap().content,
            Some("Hello!".to_string())
        );
        assert_eq!(response.usage.unwrap().total_tokens, 15);
    }

    #[test]
    fn test_xai_response_with_refusal() {
        let json = r#"{
            "choices": [{
                "message": {
                    "content": null,
                    "refusal": "I cannot comply with this request."
                },
                "finish_reason": "stop"
            }],
            "model": "grok-beta"
        }"#;

        let response: XaiResponse = serde_json::from_str(json).unwrap();
        let msg = response.choices[0].message.as_ref().unwrap();
        assert_eq!(msg.content, None);
        assert_eq!(msg.refusal, Some("I cannot comply with this request.".to_string()));
    }

    #[test]
    fn test_xai_response_with_token_details() {
        let json = r#"{
            "choices": [{
                "message": {"content": "Response"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150,
                "prompt_tokens_details": {
                    "text_tokens": 80,
                    "audio_tokens": 10,
                    "image_tokens": 10,
                    "cached_tokens": 20
                }
            },
            "model": "grok-beta"
        }"#;

        let response: XaiResponse = serde_json::from_str(json).unwrap();
        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);

        let details = usage.prompt_tokens_details.unwrap();
        assert_eq!(details.text_tokens, Some(80));
        assert_eq!(details.audio_tokens, Some(10));
        assert_eq!(details.image_tokens, Some(10));
        assert_eq!(details.cached_tokens, Some(20));
    }
}