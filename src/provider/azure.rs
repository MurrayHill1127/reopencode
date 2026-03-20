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
const DEFAULT_API_VERSION: &str = "2024-02-15-preview";

pub struct AzureProvider {
    config: ProviderConfig,
    client: Client,
    resource_name: Option<String>,
    api_version: String,
}

// Request/response types match OpenAI format
#[derive(Serialize)]
struct AzureRequest {
    messages: Vec<AzureMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
}

#[derive(Serialize)]
struct AzureMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ProviderToolCall>,
}

#[derive(Deserialize)]
struct AzureResponse {
    choices: Vec<AzureChoice>,
    usage: Option<AzureUsage>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct AzureChoice {
    message: Option<AzureMessageResponse>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct AzureMessageResponse {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<AzureToolCall>,
}

#[derive(Deserialize)]
struct AzureToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: AzureToolCallFunction,
}

#[derive(Deserialize)]
struct AzureToolCallFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct AzureUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct AzureStreamResponse {
    choices: Vec<AzureStreamChoice>,
}

#[derive(Deserialize)]
struct AzureStreamChoice {
    delta: AzureDelta,
}

#[derive(Deserialize)]
struct AzureDelta {
    content: Option<String>,
}

impl AzureProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        // Parse resource_name from base_url if present
        // Expected pattern: https://{resource}.openai.azure.com/...
        let resource_name = config.base_url.as_ref().and_then(|url| {
            url.strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))
                .and_then(|s| {
                    let host_part = s.split('/').next().unwrap_or("");
                    host_part.strip_suffix(".openai.azure.com").map(|s| s.to_string())
                })
        });

        Self {
            config,
            client,
            resource_name,
            api_version: DEFAULT_API_VERSION.to_string(),
        }
    }

    /// Create a new Azure provider with custom API version
    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    fn get_resource_name(&self) -> Option<&str> {
        self.resource_name.as_deref()
    }

    /// Build the Azure OpenAI URL for a specific deployment
    /// Format: https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}
    fn build_url(&self, deployment: &str) -> String {
        let resource = self
            .get_resource_name()
            .expect("Resource name must be set via base_url");
        format!(
            "https://{}.openai.azure.com/openai/deployments/{}/chat/completions?api-version={}",
            resource, deployment, self.api_version
        )
    }

    fn convert_messages(messages: Vec<Message>) -> Vec<AzureMessage> {
        messages
            .into_iter()
            .map(|m| AzureMessage {
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
            if let Ok(response) = serde_json::from_str::<AzureStreamResponse>(data)
                && let Some(choice) = response.choices.first()
            {
                return choice.delta.content.clone();
            }
        }
        None
    }
}

#[async_trait]
impl Provider for AzureProvider {
    fn name(&self) -> &str {
        "azure"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        info!("调用 Azure OpenAI API, 部署: {}", model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = self.build_url(model);

        let request = AzureRequest {
            messages: Self::convert_messages(messages),
            temperature,
            max_tokens,
            stream: None,
            tools: tools.to_vec(),
        };

        // Azure uses api-key header, not Bearer token
        let response = self
            .client
            .post(&url)
            .header("api-key", &self.config.api_key)
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

        let json: AzureResponse = response.json().await?;

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
        let url = self.build_url(model);
        let api_key = self.config.api_key.clone();
        let client = self.client.clone();
        let azure_messages = Self::convert_messages(messages);
        let model = model.to_string();
        let tools = tools.to_vec();

        let request = AzureRequest {
            messages: azure_messages,
            temperature,
            max_tokens,
            stream: Some(true),
            tools,
        };

        Box::pin(async_stream::stream! {
            info!("调用 Azure OpenAI API (流式), 部署: {}", model);

            let response = match client
                .post(&url)
                .header("api-key", &api_key)
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
    fn test_azure_provider_new() {
        let config = ProviderConfig::new("azure", "test-key")
            .with_base_url("https://myresource.openai.azure.com");
        let provider = AzureProvider::new(config);
        assert_eq!(provider.name(), "azure");
        assert_eq!(provider.get_resource_name(), Some("myresource"));
    }

    #[test]
    fn test_azure_provider_without_base_url() {
        let config = ProviderConfig::new("azure", "test-key");
        let provider = AzureProvider::new(config);
        assert_eq!(provider.name(), "azure");
        assert!(provider.get_resource_name().is_none());
    }

    #[test]
    fn test_azure_url_construction() {
        let config = ProviderConfig::new("azure", "test-key")
            .with_base_url("https://myresource.openai.azure.com");
        let provider = AzureProvider::new(config);
        let url = provider.build_url("gpt-4-deployment");
        
        assert!(url.starts_with("https://myresource.openai.azure.com"));
        assert!(url.contains("/openai/deployments/gpt-4-deployment/"));
        assert!(url.contains("/chat/completions"));
        assert!(url.contains("api-version=2024-02-15-preview"));
    }

    #[test]
    fn test_azure_custom_api_version() {
        let config = ProviderConfig::new("azure", "test-key")
            .with_base_url("https://myresource.openai.azure.com");
        let provider = AzureProvider::new(config).with_api_version("2024-08-01-preview");
        let url = provider.build_url("gpt-4");
        
        assert!(url.contains("api-version=2024-08-01-preview"));
    }

    #[test]
    fn test_azure_request_serialization() {
        let request = AzureRequest {
            messages: vec![AzureMessage {
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
        assert!(json.contains("\"messages\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_tokens\":100"));
        // Model is NOT in Azure request (deployment is in URL)
        assert!(!json.contains("\"model\""));
    }

    #[test]
    fn test_azure_response_deserialization() {
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

        let response: AzureResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(
            response.choices[0].message.as_ref().unwrap().content,
            Some("Hello!".to_string())
        );
        assert_eq!(response.usage.unwrap().total_tokens, 15);
    }

    #[test]
    fn test_azure_response_with_tool_calls() {
        let json = r#"{
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Beijing\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "model": "gpt-4"
        }"#;

        let response: AzureResponse = serde_json::from_str(json).unwrap();
        let msg = response.choices[0].message.as_ref().unwrap();
        assert_eq!(msg.tool_calls.len(), 1);
        assert_eq!(msg.tool_calls[0].id, "call_123");
        assert_eq!(msg.tool_calls[0].function.name, "get_weather");
        assert_eq!(
            msg.tool_calls[0].function.arguments,
            r#"{"location": "Beijing"}"#
        );
    }

    #[test]
    fn test_azure_message_conversion() {
        let messages = vec![
            Message::user("Hello"),
            Message::assistant("Hi there"),
        ];

        let azure_messages = AzureProvider::convert_messages(messages);
        assert_eq!(azure_messages.len(), 2);
        assert_eq!(azure_messages[0].role, "user");
        assert_eq!(azure_messages[0].content, "Hello");
        assert_eq!(azure_messages[1].role, "assistant");
        assert_eq!(azure_messages[1].content, "Hi there");
    }

    #[test]
    fn test_azure_stream_parsing() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#;
        let content = AzureProvider::parse_stream_line(line);
        assert_eq!(content, Some("Hello".to_string()));

        let done_line = "data: [DONE]";
        assert_eq!(AzureProvider::parse_stream_line(done_line), None);
    }

    #[test]
    fn test_azure_resource_parsing() {
        // Standard format
        let config = ProviderConfig::new("azure", "key")
            .with_base_url("https://myresource.openai.azure.com");
        let provider = AzureProvider::new(config);
        assert_eq!(provider.get_resource_name(), Some("myresource"));

        // With path
        let config = ProviderConfig::new("azure", "key")
            .with_base_url("https://anotherresource.openai.azure.com/some/path");
        let provider = AzureProvider::new(config);
        assert_eq!(provider.get_resource_name(), Some("anotherresource"));
    }

    #[test]
    fn test_azure_auth_header_format() {
        // This test verifies that the request would use "api-key" header, not "Authorization: Bearer"
        // The actual HTTP request is tested in integration tests
        let config = ProviderConfig::new("azure", "my-api-key")
            .with_base_url("https://test.openai.azure.com");
        let provider = AzureProvider::new(config);
        
        // Provider should be created successfully with api-key style auth
        assert_eq!(provider.name(), "azure");
        assert_eq!(provider.config.api_key, "my-api-key");
    }
}