//! Vertex AI Provider
//!
//! Implements the Provider trait for Google Cloud Vertex AI platform.
//! Supports Gemini models via API key authentication (MVP) and OAuth (future).

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
const DEFAULT_LOCATION: &str = "us-central1";
const API_KEY_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1";
const OAUTH_BASE_URL: &str = "https://aiplatform.googleapis.com/v1";

/// Authentication strategy for Vertex AI
#[derive(Debug, Clone)]
pub enum VertexAuth {
    /// API Key authentication (GOOGLE_API_KEY)
    ApiKey(String),
    /// OAuth authentication with project ID
    #[allow(dead_code)]
    OAuth {
        project_id: String,
    },
}

impl VertexAuth {
    /// Create authentication from environment variables
    ///
    /// Checks GOOGLE_API_KEY first, falls back to ProviderConfig's api_key
    pub fn from_env(config: &ProviderConfig) -> Self {
        // Check environment variable first
        if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
            if !key.is_empty() {
                return VertexAuth::ApiKey(key);
            }
        }

        // Fall back to config api_key
        if !config.api_key.is_empty() {
            return VertexAuth::ApiKey(config.api_key.clone());
        }

        // Default to empty key (will fail at runtime)
        VertexAuth::ApiKey(String::new())
    }

    /// Get the API key if available
    pub fn api_key(&self) -> Option<&str> {
        match self {
            VertexAuth::ApiKey(key) => Some(key),
            VertexAuth::OAuth { .. } => None,
        }
    }
}

pub struct VertexProvider {
    config: ProviderConfig,
    client: Client,
    auth: VertexAuth,
}

// ============== Gemini Request Types ==============

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GeminiTool>,
}

#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Serialize)]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

// ============== Gemini Response Types ==============

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Deserialize)]
struct GeminiFunctionCall {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    args: serde_json::Value,
}

#[derive(Deserialize)]
struct GeminiUsageMetadata {
    prompt_token_count: u32,
    candidates_token_count: u32,
    total_token_count: u32,
}

// ============== Gemini Stream Types ==============

#[derive(Deserialize)]
struct GeminiStreamResponse {
    candidates: Vec<GeminiStreamCandidate>,
}

#[derive(Deserialize)]
struct GeminiStreamCandidate {
    content: GeminiContentResponse,
}

impl VertexProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .expect("Failed to create HTTP client");

        let auth = VertexAuth::from_env(&config);

        Self { config, client, auth }
    }

    /// Get base URL for API key authentication
    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or(API_KEY_BASE_URL)
    }

    /// Convert MessageRole to Gemini role string
    fn convert_role(role: &MessageRole) -> &'static str {
        match role {
            MessageRole::User => "user",
            MessageRole::Assistant => "model",
            MessageRole::System => "user",
            MessageRole::Tool => "user",
        }
    }

    /// Convert messages to Gemini contents format
    fn convert_messages(messages: Vec<Message>) -> (Option<String>, Vec<GeminiContent>) {
        let mut system_instruction = None;
        let mut gemini_contents = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    system_instruction = Some(msg.content);
                }
                MessageRole::Tool => {
                    let content = format!(
                        "Tool result (id: {}): {}",
                        msg.tool_call_id.as_deref().unwrap_or("unknown"),
                        msg.content
                    );
                    gemini_contents.push(GeminiContent {
                        role: "user".to_string(),
                        parts: vec![GeminiPart { text: content }],
                    });
                }
                _ => {
                    gemini_contents.push(GeminiContent {
                        role: Self::convert_role(&msg.role).to_string(),
                        parts: vec![GeminiPart { text: msg.content }],
                    });
                }
            }
        }

        (system_instruction, gemini_contents)
    }

    /// Convert tools to Gemini format
    fn convert_tools(tools: &[ToolDefinition]) -> Vec<GeminiTool> {
        if tools.is_empty() {
            return vec![];
        }

        let declarations: Vec<GeminiFunctionDeclaration> = tools
            .iter()
            .map(|t| GeminiFunctionDeclaration {
                name: t.function.name.clone(),
                description: t.function.description.clone(),
                parameters: t.function.parameters.clone(),
            })
            .collect();

        vec![GeminiTool {
            function_declarations: declarations,
        }]
    }

    /// Parse SSE stream line for Gemini format
    fn parse_stream_line(line: &str) -> Option<String> {
        if let Some(data) = line.strip_prefix("data: ") {
            if data.is_empty() {
                return None;
            }
            if let Ok(response) = serde_json::from_str::<GeminiStreamResponse>(data) {
                if let Some(candidate) = response.candidates.first() {
                    if let Some(part) = candidate.content.parts.first() {
                        return part.text.clone();
                    }
                }
            }
        }
        None
    }

    /// Build URL for Gemini API (API key auth)
    fn build_url(&self, model: &str, streaming: bool) -> String {
        let endpoint = if streaming {
            "streamGenerateContent"
        } else {
            "generateContent"
        };
        let base = self.get_base_url();
        let api_key = self.auth.api_key().unwrap_or("");

        let alt_sse = if streaming { "&alt=sse" } else { "" };
        format!(
            "{}/models/{}:{}?key={}{}",
            base, model, endpoint, api_key, alt_sse
        )
    }
}

#[async_trait]
impl Provider for VertexProvider {
    fn name(&self) -> &str {
        "vertex"
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
        tools: &[ToolDefinition],
    ) -> Result<ProviderResponse> {
        info!("调用 Vertex AI API, 模型: {}", model);
        debug!(
            "请求参数: messages={}, temp={}",
            messages.len(),
            temperature
        );

        let url = self.build_url(model, false);

        let (system_instruction, gemini_contents) = Self::convert_messages(messages);

        let generation_config = GeminiGenerationConfig {
            temperature: Some(temperature),
            max_output_tokens: max_tokens,
        };

        let request = GeminiRequest {
            contents: gemini_contents,
            generation_config: Some(generation_config),
            tools: Self::convert_tools(tools),
        };

        let mut request_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request);

        // Add system instruction if present
        if let Some(system) = system_instruction {
            let system_request = serde_json::json!({
                "system_instruction": {
                    "parts": [{"text": system}]
                }
            });
            let mut combined = serde_json::to_value(&request).map_err(ProviderError::Json)?;
            if let Some(obj) = combined.as_object_mut() {
                if let Some(sys_obj) = system_request.as_object() {
                    for (k, v) in sys_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
            request_builder = request_builder.json(&combined);
        }

        let response = request_builder.send().await?;

        let status = response.status();
        if status.as_u16() == 429 {
            error!("速率限制");
            return Err(ProviderError::RateLimit);
        }
        if status.as_u16() == 401 || status.as_u16() == 403 {
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

        let json: GeminiResponse = response.json().await?;

        let candidate = json
            .candidates
            .first()
            .ok_or_else(|| ProviderError::Api("No candidates in response".to_string()))?;

        let content = candidate
            .content
            .parts
            .first()
            .and_then(|p| p.text.clone())
            .unwrap_or_default();

        let usage = json
            .usage_metadata
            .map(|u| Usage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
            })
            .unwrap_or_default();

        let finish_reason = candidate.finish_reason.clone();

        Ok(ProviderResponse {
            content,
            model: model.to_string(),
            usage,
            finish_reason,
            tool_calls: vec![],
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
        let model = model.to_string();
        let url = self.build_url(&model, true);
        let client = self.client.clone();
        let (system_instruction, gemini_contents) = Self::convert_messages(messages);

        let generation_config = GeminiGenerationConfig {
            temperature: Some(temperature),
            max_output_tokens: max_tokens,
        };

        let request = GeminiRequest {
            contents: gemini_contents,
            generation_config: Some(generation_config),
            tools: Self::convert_tools(tools),
        };

        let system_instruction = system_instruction;

        Box::pin(async_stream::stream! {
            info!("调用 Vertex AI API (流式), 模型: {}", model);

            let request_body = if let Some(system) = system_instruction {
                let mut combined = serde_json::to_value(&request).map_err(ProviderError::Json)?;
                if let Some(obj) = combined.as_object_mut() {
                    let system_json = serde_json::json!({
                        "system_instruction": {
                            "parts": [{"text": system}]
                        }
                    });
                    if let Some(sys_obj) = system_json.as_object() {
                        for (k, v) in sys_obj {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }
                combined
            } else {
                serde_json::to_value(&request).map_err(ProviderError::Json)?
            };

            let response = match client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request_body)
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
    fn test_vertex_provider_new() {
        let config = ProviderConfig::new("vertex", "test-api-key");
        let provider = VertexProvider::new(config);
        assert_eq!(provider.name(), "vertex");
    }

    #[test]
    fn test_vertex_base_url_api_key() {
        let config = ProviderConfig::new("vertex", "test-api-key");
        let provider = VertexProvider::new(config);
        assert_eq!(provider.get_base_url(), API_KEY_BASE_URL);
    }

    #[test]
    fn test_vertex_request_serialization() {
        let request = GeminiRequest {
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiPart {
                    text: "Hello".to_string(),
                }],
            }],
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(0.7),
                max_output_tokens: Some(1024),
            }),
            tools: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"contents\""));
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"parts\""));
        assert!(json.contains("\"text\":\"Hello\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(json.contains("\"max_output_tokens\":1024"));
    }

    #[test]
    fn test_vertex_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello!"}]
                },
                "finish_reason": "STOP"
            }],
            "usage_metadata": {
                "prompt_token_count": 10,
                "candidates_token_count": 5,
                "total_token_count": 15
            }
        }"#;

        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(
            response.candidates[0].content.parts[0].text,
            Some("Hello!".to_string())
        );
        assert_eq!(
            response.candidates[0].finish_reason,
            Some("STOP".to_string())
        );
        let usage = response.usage_metadata.unwrap();
        assert_eq!(usage.prompt_token_count, 10);
        assert_eq!(usage.candidates_token_count, 5);
        assert_eq!(usage.total_token_count, 15);
    }

    #[test]
    fn test_vertex_message_conversion() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
            Message::assistant("Hi there"),
        ];

        let (system, gemini_contents) = VertexProvider::convert_messages(messages);

        assert_eq!(system, Some("You are helpful".to_string()));
        assert_eq!(gemini_contents.len(), 2);
        assert_eq!(gemini_contents[0].role, "user");
        assert_eq!(gemini_contents[0].parts[0].text, "Hello");
        assert_eq!(gemini_contents[1].role, "model");
        assert_eq!(gemini_contents[1].parts[0].text, "Hi there");
    }

    #[test]
    fn test_vertex_auth_from_env_api_key() {
        // Test with config api_key
        let config = ProviderConfig::new("vertex", "config-api-key");
        let auth = VertexAuth::from_env(&config);
        assert_eq!(auth.api_key(), Some("config-api-key"));

        // Test with empty config (should still work)
        let empty_config = ProviderConfig::new("vertex", "");
        let empty_auth = VertexAuth::from_env(&empty_config);
        assert_eq!(empty_auth.api_key(), Some("")); // Empty key for MVP
    }

    #[test]
    fn test_vertex_role_conversion() {
        assert_eq!(VertexProvider::convert_role(&MessageRole::User), "user");
        assert_eq!(
            VertexProvider::convert_role(&MessageRole::Assistant),
            "model"
        );
        assert_eq!(VertexProvider::convert_role(&MessageRole::System), "user");
        assert_eq!(VertexProvider::convert_role(&MessageRole::Tool), "user");
    }

    #[test]
    fn test_vertex_custom_base_url() {
        let config = ProviderConfig::new("vertex", "test-api-key")
            .with_base_url("https://custom.googleapis.com/v1");
        let provider = VertexProvider::new(config);
        assert_eq!(provider.get_base_url(), "https://custom.googleapis.com/v1");
    }

    #[test]
    fn test_vertex_tool_conversion() {
        let tools = vec![ToolDefinition {
            tool_type: "function".to_string(),
            function: crate::provider::provider_trait::ToolFunction {
                name: "get_weather".to_string(),
                description: "Get weather info".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                }),
            },
        }];

        let gemini_tools = VertexProvider::convert_tools(&tools);
        assert_eq!(gemini_tools.len(), 1);
        assert_eq!(gemini_tools[0].function_declarations.len(), 1);
        assert_eq!(
            gemini_tools[0].function_declarations[0].name,
            "get_weather"
        );
        assert_eq!(
            gemini_tools[0].function_declarations[0].description,
            "Get weather info"
        );
    }

    #[test]
    fn test_vertex_tool_conversion_empty() {
        let tools: Vec<ToolDefinition> = vec![];
        let gemini_tools = VertexProvider::convert_tools(&tools);
        assert!(gemini_tools.is_empty());
    }

    #[test]
    fn test_vertex_parse_stream_line() {
        let line = r#"data: {"candidates":[{"content":{"parts":[{"text":"Hello"}]}}]}"#;
        let result = VertexProvider::parse_stream_line(line);
        assert_eq!(result, Some("Hello".to_string()));
    }

    #[test]
    fn test_vertex_parse_stream_line_empty() {
        let line = "data: ";
        let result = VertexProvider::parse_stream_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_vertex_url_building() {
        let config = ProviderConfig::new("vertex", "my-secret-key");
        let provider = VertexProvider::new(config);

        let url_non_streaming = provider.build_url("gemini-pro", false);
        assert!(url_non_streaming.contains("/models/gemini-pro:generateContent"));
        assert!(url_non_streaming.contains("key=my-secret-key"));
        assert!(!url_non_streaming.contains("alt=sse"));

        let url_streaming = provider.build_url("gemini-pro", true);
        assert!(url_streaming.contains("/models/gemini-pro:streamGenerateContent"));
        assert!(url_streaming.contains("key=my-secret-key"));
        assert!(url_streaming.contains("alt=sse"));
    }

    #[test]
    fn test_vertex_stream_response_deserialization() {
        let json = r#"{
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello"}]
                }
            }]
        }"#;

        let response: GeminiStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.candidates.len(), 1);
        assert_eq!(
            response.candidates[0].content.parts[0].text,
            Some("Hello".to_string())
        );
    }
}