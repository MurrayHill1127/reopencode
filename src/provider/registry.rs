use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tracing::{debug, info, warn};

use crate::provider::anthropic::AnthropicProvider;
use crate::provider::atlascloud::AtlasCloudProvider;
use crate::provider::azure::AzureProvider;
use crate::provider::config::ProvidersConfig;
use crate::provider::error::{ProviderError, Result};
use crate::provider::google::GoogleProvider;
use crate::provider::openai::OpenAiProvider;
use crate::provider::openrouter::OpenRouterProvider;
use crate::provider::provider_trait::Provider;
use crate::provider::requesty::RequestyProvider;
use crate::provider::vertex::VertexProvider;
use crate::provider::xai::XaiProvider;
use crate::provider::zhipu::ZhipuProvider;

pub struct ProviderRegistry {
    providers: Arc<RwLock<HashMap<String, Arc<dyn Provider>>>>,
    default_provider: Arc<RwLock<String>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            default_provider: Arc::new(RwLock::new("openai".to_string())),
        }
    }

    pub fn register(&self, provider: Arc<dyn Provider>) {
        let name = provider.name().to_string();
        info!("注册 Provider: {}", name);

        let mut providers = self
            .providers
            .write()
            .expect("Failed to acquire write lock");
        providers.insert(name.clone(), provider);

        if providers.len() == 1 {
            let mut default = self
                .default_provider
                .write()
                .expect("Failed to acquire write lock");
            *default = name;
            debug!("设置默认 Provider: {}", *default);
        }
    }

    /// Get a provider by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        let providers = self.providers.read().expect("Failed to acquire read lock");
        providers.get(name).cloned()
    }

    /// Get the default provider
    pub fn get_default(&self) -> Option<Arc<dyn Provider>> {
        let default = self
            .default_provider
            .read()
            .expect("Failed to acquire read lock");
        self.get(&default)
    }

    pub fn contains(&self, name: &str) -> bool {
        let providers = self.providers.read().expect("Failed to acquire read lock");
        providers.contains_key(name)
    }

    pub fn get_provider_name(&self, name: &str) -> Option<String> {
        let providers = self.providers.read().expect("Failed to acquire read lock");
        providers.get(name).map(|p| p.name().to_string())
    }

    pub fn default_provider(&self) -> String {
        self.default_provider
            .read()
            .expect("Failed to acquire read lock")
            .clone()
    }

    pub fn set_default(&self, name: &str) -> Result<()> {
        let providers = self.providers.read().expect("Failed to acquire read lock");
        if !providers.contains_key(name) {
            warn!("尝试设置不存在的默认 Provider: {}", name);
            return Err(ProviderError::Config(format!("Provider 不存在: {}", name)));
        }
        drop(providers);

        let mut default = self
            .default_provider
            .write()
            .expect("Failed to acquire write lock");
        *default = name.to_string();
        info!("设置默认 Provider: {}", name);
        Ok(())
    }

    pub fn list(&self) -> Vec<String> {
        let providers = self.providers.read().expect("Failed to acquire read lock");
        providers.keys().cloned().collect()
    }

    pub fn remove(&self, name: &str) -> Option<String> {
        let mut providers = self
            .providers
            .write()
            .expect("Failed to acquire write lock");
        providers.remove(name).map(|_| {
            info!("移除 Provider: {}", name);
            name.to_string()
        })
    }

    pub fn clear(&self) {
        let mut providers = self
            .providers
            .write()
            .expect("Failed to acquire write lock");
        providers.clear();
        info!("清空所有 Provider");
    }

    pub fn len(&self) -> usize {
        let providers = self.providers.read().expect("Failed to acquire read lock");
        providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn from_config(config: &ProvidersConfig) -> Self {
        let registry = Self::new();

        for (name, provider_config) in &config.providers {
            match name.as_str() {
                "openai" => {
                    let provider = OpenAiProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "anthropic" => {
                    let provider = AnthropicProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "azure" => {
                    let provider = AzureProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "google" => {
                    let provider = GoogleProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "zhipu" => {
                    let provider = ZhipuProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "openrouter" => {
                    let provider = OpenRouterProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "requesty" => {
                    let provider = RequestyProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "xai" => {
                    let provider = XaiProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "atlascloud" => {
                    let provider = AtlasCloudProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                "vertex" => {
                    let provider = VertexProvider::new(provider_config.clone());
                    registry.register(Arc::new(provider));
                }
                _ => {
                    warn!("未知的 Provider 类型: {}", name);
                }
            }
        }

        if !config.default_provider.is_empty()
            && let Err(e) = registry.set_default(&config.default_provider)
        {
            warn!("设置默认 Provider 失败: {}", e);
        }

        registry
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ProviderRegistry {
    fn clone(&self) -> Self {
        Self {
            providers: Arc::clone(&self.providers),
            default_provider: Arc::clone(&self.default_provider),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::config::ProviderConfig;

    #[test]
    fn test_registry_new() {
        let registry = ProviderRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.default_provider(), "openai");
    }

    #[test]
    fn test_registry_register() {
        let registry = ProviderRegistry::new();
        let config = ProviderConfig::new("openai", "sk-test");
        let provider = OpenAiProvider::new(config);

        registry.register(Arc::new(provider));

        assert_eq!(registry.len(), 1);
        assert!(registry.list().contains(&"openai".to_string()));
    }

    #[test]
    fn test_registry_get_provider() {
        let registry = ProviderRegistry::new();
        let config = ProviderConfig::new("openai", "sk-test");
        let provider = OpenAiProvider::new(config);
        registry.register(Arc::new(provider));

        let result = registry.get_provider_name("openai");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "openai");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = ProviderRegistry::new();
        let result = registry.get_provider_name("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_registry_set_default() {
        let registry = ProviderRegistry::new();

        let config1 = ProviderConfig::new("openai", "sk-test1");
        let config2 = ProviderConfig::new("anthropic", "sk-ant-test2");

        registry.register(Arc::new(OpenAiProvider::new(config1)));
        registry.register(Arc::new(AnthropicProvider::new(config2)));

        assert_eq!(registry.default_provider(), "openai");

        registry.set_default("anthropic").unwrap();
        assert_eq!(registry.default_provider(), "anthropic");
    }

    #[test]
    fn test_registry_set_default_nonexistent() {
        let registry = ProviderRegistry::new();
        let result = registry.set_default("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_remove() {
        let registry = ProviderRegistry::new();
        let config = ProviderConfig::new("openai", "sk-test");
        registry.register(Arc::new(OpenAiProvider::new(config)));

        let removed = registry.remove("openai");
        assert_eq!(removed, Some("openai".to_string()));
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_clear() {
        let registry = ProviderRegistry::new();
        registry.register(Arc::new(OpenAiProvider::new(ProviderConfig::new(
            "openai", "sk-test",
        ))));
        registry.register(Arc::new(AnthropicProvider::new(ProviderConfig::new(
            "anthropic",
            "sk-ant-test",
        ))));

        assert_eq!(registry.len(), 2);
        registry.clear();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_from_config() {
        let toml = r#"
default_provider = "anthropic"

[providers.openai]
name = "openai"
api_key = "sk-test"
models = ["gpt-4"]

[providers.anthropic]
name = "anthropic"
api_key = "sk-ant-test"
models = ["claude-3-opus"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert_eq!(registry.len(), 2);
        assert!(registry.list().contains(&"openai".to_string()));
        assert!(registry.list().contains(&"anthropic".to_string()));
        assert_eq!(registry.default_provider(), "anthropic");
    }

    #[test]
    fn test_registry_clone() {
        let registry = ProviderRegistry::new();
        registry.register(Arc::new(OpenAiProvider::new(ProviderConfig::new(
            "openai", "sk-test",
        ))));

        let cloned = registry.clone();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned.default_provider(), registry.default_provider());
    }

    #[test]
    fn test_registry_tier2_openrouter() {
        let toml = r#"
[providers.openrouter]
name = "openrouter"
api_key = "sk-or-test"
models = ["openai/gpt-4"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert!(registry.get("openrouter").is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_requesty() {
        let toml = r#"
[providers.requesty]
name = "requesty"
api_key = "sk-req-test"
models = ["openai/gpt-4"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert!(registry.get("requesty").is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_tier2_xai() {
        let toml = r#"
[providers.xai]
name = "xai"
api_key = "xai-test"
models = ["grok-beta"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert!(registry.get("xai").is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_atlascloud() {
        let toml = r#"
[providers.atlascloud]
name = "atlascloud"
api_key = "apikey-test"
base_url = "https://api.atlascloud.ai/v1"
models = ["deepseek-ai/deepseek-v4-pro"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert!(registry.get("atlascloud").is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_tier2_vertex() {
        let toml = r#"
[providers.vertex]
name = "vertex"
api_key = "google-api-key"
models = ["gemini-pro"]
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert!(registry.get("vertex").is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_all_tier2_providers() {
        let toml = r#"
default_provider = "openrouter"

[providers.openrouter]
name = "openrouter"
api_key = "sk-or-test"

[providers.xai]
name = "xai"
api_key = "xai-test"

[providers.vertex]
name = "vertex"
api_key = "google-api-key"
"#;
        let config = ProvidersConfig::from_toml(toml).unwrap();
        let registry = ProviderRegistry::from_config(&config);

        assert_eq!(registry.len(), 3);
        assert!(registry.list().contains(&"openrouter".to_string()));
        assert!(registry.list().contains(&"xai".to_string()));
        assert!(registry.list().contains(&"vertex".to_string()));
        assert_eq!(registry.default_provider(), "openrouter");
    }
}
