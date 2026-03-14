use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::provider::config::ProviderConfig;
use crate::provider::error::{ProviderError, Result};
use crate::provider::message::{Message, MessageRole};
use crate::provider::trait::{Provider, ProviderResponse, Usage};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAiProvider {
    config: ProviderConfig,
    client: Client,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessageResponse>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiMessageResponse {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAiStreamResponse {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
}

impl OpenAiProvider {
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

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<OpenAiMessage> {
        messages
            .into_iter()
            .map(|m| OpenAiMessage {
                role: m.role.to_string(),
                content: m.content,
            })
            .collect()
    }

    fn parse_stream_line(&self, line: &str) -> Option<String> {
        if line.starts_with("data: ") {
            let data = &line[6..];
            if data == "[DONE]" {
                return None;
            }
            if let Ok(response) = serde_json::from_str::<OpenAiStreamResponse>(data) {
                if let Some(choice) = response.choices.first() {
                    return choice.delta.content.clone();
                }
            }
        }
        None
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<ProviderResponse> {
        info!("调用 OpenAI API, 模型: {}", model);
        debug!("请求参数: messages={}, temp={}", messages.len(), temperature);

        let url = format!("{}/chat/completions", self.get_base_url());

        let request = OpenAiRequest {
            model: model.to_string(),
            messages: self.convert_messages(messages),
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
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            error!("API 错误: {}", error_text);
            return Err(ProviderError::Api(error_text));
        }

        let json: OpenAiResponse = response.json().await?;

        let choice = json.choices.first().ok_or_else(|| {
            ProviderError::Api("No choices in response".to_string())
        })?;

        let content = choice
            .message
            .as_ref()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        let usage = json.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or_default();

        Ok(ProviderResponse {
            content,
            model: json.model.unwrap_or_else(|| model.to_string()),
            usage,
            finish_reason: choice.finish_reason.clone(),
        })
    }

    fn chat_stream(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> impl Stream<Item = Result<String>> + Send {
        let url = format!("{}/chat/completions", self.get_base_url());
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let openai_messages = self.convert_messages(messages);

        let request = OpenAiRequest {
            model: model.to_string(),
            messages: openai_messages,
            temperature,
            max_tokens,
            stream: Some(true),
        };

        async_stream::stream! {
            info!("调用 OpenAI API (流式), 模型: {}", model);

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

                    if let Some(content) = Self { config: ProviderConfig::default(), client: client.clone() }.parse_stream_line(line) {
                        yield Ok(content);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_new() {
        let config = ProviderConfig::new("openai", "sk-test");
        let provider = OpenAiProvider::new(config);
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_openai_provider_base_url() {
        let config = ProviderConfig::new("openai", "sk-test");
        let provider = OpenAiProvider::new(config);
        assert_eq!(provider.get_base_url(), DEFAULT_BASE_URL);
    }

    #[test]
    fn test_openai_provider_custom_base_url() {
        let config = ProviderConfig::new("openai", "sk-test")
            .with_base_url("https://custom.api.com/v1");
        let provider = OpenAiProvider::new(config);
        assert_eq!(provider.get_base_url(), "https://custom.api.com/v1");
    }

    #[test]
    fn test_openai_request_serialization() {
        let request = OpenAiRequest {
            model: "gpt-4".to_string(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: 0.7,
            max_tokens: Some(100),
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
    }

    #[test]
    fn test_openai_response_deserialization() {
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
            "model": "gpt-4"
        }"#;

        let response: OpenAiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.choices[0].message.as_ref().unwrap().content, Some("Hello!".to_string()));
        assert_eq!(response.usage.unwrap().total_tokens, 15);
    }
}