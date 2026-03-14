//! AI Provider system

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

/// Message for provider API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

/// Provider response
#[derive(Debug)]
pub struct ProviderResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
}

/// Usage statistics
#[derive(Debug, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Provider error
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Provider trait
#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    
    async fn chat(
        &self,
        messages: Vec<ProviderMessage>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<ProviderResponse, ProviderError>;
}

/// OpenAI Provider
pub struct OpenAiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl OpenAiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }
    
    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("https://api.openai.com/v1")
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }
    
    async fn chat(
        &self,
        messages: Vec<ProviderMessage>,
        model: &str,
        temperature: f32,
        max_tokens: Option<u32>,
    ) -> Result<ProviderResponse, ProviderError> {
        let url = format!("{}/chat/completions", self.get_base_url());
        
        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
        });
        
        if let Some(max_tokens) = max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(ProviderError::Api(error));
        }
        
        let json: Value = response.json().await?;
        
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let usage = Usage {
            prompt_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };
        
        Ok(ProviderResponse {
            content,
            model: model.to_string(),
            usage,
        })
    }
}

/// Provider registry
pub struct ProviderRegistry {
    providers: std::collections::HashMap<String, Box<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }
    
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Provider> {
        self.providers.get(name).map(|p| p.as_ref())
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
