use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::provider::error::{ProviderError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
}

impl ProviderConfig {
    pub fn new(name: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            api_key: api_key.into(),
            base_url: None,
            models: Vec::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }

    pub fn supports_model(&self, model: &str) -> bool {
        self.models.is_empty() || self.models.iter().any(|m| m == model)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub providers: HashMap<String, ProviderConfig>,
    #[serde(default = "default_provider")]
    pub default_provider: String,
}

fn default_provider() -> String {
    "openai".to_string()
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();
        providers.insert(
            "openai".to_string(),
            ProviderConfig {
                name: "openai".to_string(),
                api_key: String::new(),
                base_url: Some("https://api.openai.com/v1".to_string()),
                models: vec![
                    "gpt-4".to_string(),
                    "gpt-4-turbo".to_string(),
                    "gpt-3.5-turbo".to_string(),
                ],
            },
        );
        Self {
            providers,
            default_provider: "openai".to_string(),
        }
    }
}

impl ProvidersConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }

    pub fn default_config(&self) -> Option<&ProviderConfig> {
        self.get(&self.default_provider)
    }

    pub fn add(&mut self, config: ProviderConfig) {
        self.providers.insert(config.name.clone(), config);
    }

    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ProviderError::Config(format!("无法读取配置文件：{}", e)))?;

        toml::from_str(&content)
            .map_err(|e| ProviderError::Config(format!("配置文件格式错误：{}", e)))
    }

    pub fn from_toml(toml_str: &str) -> Result<Self> {
        toml::from_str(toml_str)
            .map_err(|e| ProviderError::Config(format!("配置格式错误：{}", e)))
    }

    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("reopencode")
            .join("providers.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            Self::from_file(&path)
        } else {
            Ok(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_config_new() {
        let config = ProviderConfig::new("openai", "sk-test");
        assert_eq!(config.name, "openai");
        assert_eq!(config.api_key, "sk-test");
        assert!(config.base_url.is_none());
        assert!(config.models.is_empty());
    }

    #[test]
    fn test_provider_config_with_base_url() {
        let config = ProviderConfig::new("openai", "sk-test")
            .with_base_url("https://api.example.com/v1");
        assert_eq!(config.base_url, Some("https://api.example.com/v1".to_string()));
    }

    #[test]
    fn test_provider_config_with_models() {
        let config = ProviderConfig::new("openai", "sk-test")
            .with_models(vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()]);
        assert_eq!(config.models.len(), 2);
        assert!(config.supports_model("gpt-4"));
        assert!(config.supports_model("gpt-3.5-turbo"));
        assert!(!config.supports_model("claude"));
    }

    #[test]
    fn test_provider_config_supports_model_empty() {
        let config = ProviderConfig::new("test", "key");
        assert!(config.supports_model("any-model"));
    }

    #[test]
    fn test_providers_config_default() {
        let config = ProvidersConfig::default();
        assert!(config.providers.contains_key("openai"));
        assert_eq!(config.default_provider, "openai");
    }

    #[test]
    fn test_providers_config_from_toml() {
        let toml = r#"
default_provider = "openai"

[providers.openai]
name = "openai"
api_key = "sk-test"
base_url = "https://api.openai.com/v1"
models = ["gpt-4", "gpt-3.5-turbo"]

[providers.anthropic]
name = "anthropic"
api_key = "sk-ant-test"
models = ["claude-3-opus", "claude-3-sonnet"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        assert_eq!(config.providers.len(), 2);
        assert!(config.providers.contains_key("openai"));
        assert!(config.providers.contains_key("anthropic"));
        assert_eq!(config.default_provider, "openai");
    }

    #[test]
    fn test_providers_config_get() {
        let config = ProvidersConfig::default();
        let openai = config.get("openai").unwrap();
        assert_eq!(openai.name, "openai");
        assert!(config.get("nonexistent").is_none());
    }

    #[test]
    fn test_providers_config_add() {
        let mut config = ProvidersConfig::new();
        let provider = ProviderConfig::new("test", "key");
        config.add(provider);
        assert!(config.providers.contains_key("test"));
    }
}