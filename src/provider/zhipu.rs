use std::pin::Pin;

use async_trait::async_trait;
use futures::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::provider::config::ProviderConfig;
use crate::provider::error::{ProviderError, Result};
use crate::provider::message::Message;
use crate::provider::provider_trait::{Provider, ProviderResponse, ToolDefinition, Usage};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

pub struct ZhipuProvider {
    config: ProviderConfig,
    client: Client,
}

#[derive(Serialize)]
struct ZhipuRequest {
    model: String,
    messages: Vec<ZhipuMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct ZhipuMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ZhipuResponse {
    choices: Option<Vec<ZhipuChoice>>,
    usage: Option<ZhipuUsage>,
    model: Option<String>,
    error: Option<ZhipuError>,
}

#[derive(Deserialize)]
struct ZhipuChoice {
    message: Option<ZhipuMessage>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ZhipuUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct ZhipuError {
    message: String,
    #[allow(dead_code)]
    code: Option<String>,
}

#[derive(Deserialize)]
struct ZhipuStreamResponse {
    choices: Vec<ZhipuStreamChoice>,
}

#[derive(Deserialize)]
struct ZhipuStreamChoice {
    delta: ZhipuDelta,
}

#[derive(Deserialize)]
struct ZhipuDelta {
    content: Option<String>,
}

impl ZhipuProvider {
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

    fn convert_messages(messages: Vec<Message>) -> Vec<ZhipuMessage> {
        messages
            .into_iter()
            .map(|m| ZhipuMessage {
                role: m.role.to_string(),
                content: m.content,
            })
            .collect()
    }

    fn parse_stream_line(line: &str) -> Option<String> {
        if line.starts_with("data: ") {
            let data = &line[6..];
            if data == "[DONE]" {
                return None;
            }
            if let Ok(response) = serde_json::from_str::<ZhipuStreamResponse>(data) {
                if let Some(choice) = response.choices.first() {
                    return choice.delta.content.clone();
                }
            }
        }
        None
    }
}

#[async_trait]
impl Provider for ZhipuProvider {
    fn name(&self) -> &str {
        "zhipu"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        _tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        info!("调用智谱 AI API, 模型: {}", model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = format!("{}/chat/completions", self.get_base_url());

        let request = ZhipuRequest {
            model: model.to_string(),
            messages: Self::convert_messages(messages),
            temperature,
            max_tokens,
            stream: None,
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

        let json: ZhipuResponse = response.json().await?;

        if let Some(error) = json.error {
            error!("智谱 API 错误: {}", error.message);
            return Err(ProviderError::Api(error.message));
        }

        let choices = json.choices.unwrap_or_default();
        let choice = choices
            .first()
            .ok_or_else(|| ProviderError::Api("No choices in response".to_string()))?;

        let content = choice
            .message
            .as_ref()
            .map(|m| m.content.clone())
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
        let url = format!("{}/chat/completions", self.get_base_url());
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let zhipu_messages = Self::convert_messages(messages);
        let model = model.to_string();

        let request = ZhipuRequest {
            model: model.clone(),
            messages: zhipu_messages,
            temperature,
            max_tokens,
            stream: Some(true),
        };

        Box::pin(async_stream::stream! {
            info!("调用智谱 AI API (流式), 模型: {}", model);

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
    fn test_zhipu_provider_new() {
        let config = ProviderConfig::new("zhipu", "test-api-key");
        let provider = ZhipuProvider::new(config);
        assert_eq!(provider.name(), "zhipu");
    }

    #[test]
    fn test_zhipu_provider_base_url() {
        let config = ProviderConfig::new("zhipu", "test-api-key");
        let provider = ZhipuProvider::new(config);
        assert_eq!(provider.get_base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_zhipu_provider_custom_base_url() {
        let config = ProviderConfig::new("zhipu", "test-api-key")
            .with_base_url("https://custom.zhipu.com/v1");
        let provider = ZhipuProvider::new(config);
        assert_eq!(provider.get_base_url(), "https://custom.zhipu.com/v1");
    }

    #[test]
    fn test_zhipu_request_serialization() {
        let request = ZhipuRequest {
            model: "glm-4".to_string(),
            messages: vec![ZhipuMessage {
                role: "user".to_string(),
                content: "你好".to_string(),
            }],
            temperature: 0.7,
            max_tokens: Some(100),
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"glm-4\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
    }

    #[test]
    fn test_zhipu_response_deserialization() {
        let json = r#"{
            "choices": [{
                "message": {"role": "assistant", "content": "你好！"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            },
            "model": "glm-4"
        }"#;

        let response: ZhipuResponse = serde_json::from_str(json).unwrap();
        let choices = response.choices.unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].message.as_ref().unwrap().content, "你好！");
        assert_eq!(response.usage.unwrap().total_tokens, 15);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_zhipu_error_response() {
        let json = r#"{
            "error": {
                "message": "API key 无效",
                "code": "INVALID_API_KEY"
            }
        }"#;

        let response: ZhipuResponse = serde_json::from_str(json).unwrap();
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.message, "API key 无效");
    }

    #[test]
    fn test_zhipu_message_serialization() {
        let msg = ZhipuMessage {
            role: "user".to_string(),
            content: "测试消息".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"测试消息\""));
    }
}
