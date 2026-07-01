//! Authentication manager for API keys
//!
//! This module provides credential management with priority resolution:
//! Config → Environment → Auth file

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

use super::config::ProviderConfig;
use super::id::ProviderId;

/// Credential source enum
///
/// Tracks where an API key was obtained from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CredentialSource {
    /// From explicit provider configuration
    Config,

    /// From environment variable (contains env var name)
    Env(&'static str),

    /// From auth file
    AuthFile,
}

impl CredentialSource {
    /// Get a human-readable description
    pub fn as_str(&self) -> &'static str {
        match self {
            CredentialSource::Config => "config",
            CredentialSource::Env(_) => "env",
            CredentialSource::AuthFile => "auth_file",
        }
    }
}

/// Authentication manager
///
/// Manages API keys with priority resolution:
/// 1. Config (explicit provider configuration)
/// 2. Environment variables
/// 3. Auth file (~/.config/roc/auth.toml)
pub struct AuthManager {
    /// Configured API keys from provider config
    config_keys: Arc<RwLock<HashMap<ProviderId, String>>>,

    /// Environment variable mapping (ProviderId -> Env var name)
    env_mapping: HashMap<ProviderId, &'static str>,

    /// Path to auth file (optional)
    auth_file_path: Option<PathBuf>,
}

impl AuthManager {
    /// Create a new AuthManager with default environment mapping
    pub fn new() -> Self {
        let mut env_mapping = HashMap::new();

        // Pre-define environment variable mappings
        env_mapping.insert(ProviderId::new("openai"), "OPENAI_API_KEY");
        env_mapping.insert(ProviderId::new("anthropic"), "ANTHROPIC_API_KEY");
        env_mapping.insert(ProviderId::new("azure"), "AZURE_OPENAI_API_KEY");
        env_mapping.insert(ProviderId::new("google"), "GOOGLE_API_KEY");
        env_mapping.insert(ProviderId::new("vertex"), "GOOGLE_APPLICATION_CREDENTIALS");
        env_mapping.insert(ProviderId::new("openrouter"), "OPENROUTER_API_KEY");
        env_mapping.insert(ProviderId::new("requesty"), "REQUESTY_API_KEY");
        env_mapping.insert(ProviderId::new("copilot"), "GITHUB_TOKEN");
        env_mapping.insert(ProviderId::new("xai"), "XAI_API_KEY");
        env_mapping.insert(ProviderId::new("mistral"), "MISTRAL_API_KEY");
        env_mapping.insert(ProviderId::new("groq"), "GROQ_API_KEY");
        env_mapping.insert(ProviderId::new("cerebras"), "CEREBRAS_API_KEY");
        env_mapping.insert(ProviderId::new("cohere"), "COHERE_API_KEY");
        env_mapping.insert(ProviderId::new("bedrock"), "AWS_ACCESS_KEY_ID");
        env_mapping.insert(ProviderId::new("zhipu"), "ZHIPU_API_KEY");
        env_mapping.insert(ProviderId::new("atlascloud"), "ATLASCLOUD_API_KEY");

        AuthManager {
            config_keys: Arc::new(RwLock::new(HashMap::new())),
            env_mapping,
            auth_file_path: None,
        }
    }

    /// Create AuthManager from provider configuration
    pub fn from_config(configs: &[ProviderConfig]) -> Self {
        let manager = AuthManager::new();

        for config in configs {
            // api_key is always present in ProviderConfig
            manager.set_api_key(&ProviderId::new(&config.name), &config.api_key);
        }

        manager
    }

    /// Set the auth file path
    pub fn set_auth_file_path(&mut self, path: PathBuf) {
        self.auth_file_path = Some(path);
    }

    /// Set an API key for a provider
    pub fn set_api_key(&self, provider: &ProviderId, key: &str) {
        let mut keys = self.config_keys.write().unwrap();
        keys.insert(provider.clone(), key.to_string());
    }

    /// Get mutable access to the config_keys map (for removal).
    pub fn config_keys_mut(&self) -> std::sync::RwLockWriteGuard<'_, HashMap<ProviderId, String>> {
        self.config_keys.write().unwrap()
    }

    /// Get an API key for a provider with priority resolution
    ///
    /// Priority: Config → Environment → Auth file
    ///
    /// # Returns
    /// * `Option<String>` - The API key if found
    pub fn get_api_key(&self, provider: &ProviderId) -> Option<String> {
        // Priority 1: Config keys
        {
            let keys = self.config_keys.read().unwrap();
            if let Some(key) = keys.get(provider) {
                return Some(key.clone());
            }
        }

        // Priority 2: Environment variables
        if let Some(&env_var) = self.env_mapping.get(provider) {
            if let Ok(key) = std::env::var(env_var) {
                return Some(key);
            }
        }

        // Priority 3: Auth file (not implemented yet)
        if let Some(ref _path) = self.auth_file_path {
            // TODO: Load from auth file
            // This would parse ~/.config/roc/auth.toml
        }

        None
    }

    /// Get the credential source for a provider
    pub fn get_credential_source(&self, provider: &ProviderId) -> Option<CredentialSource> {
        // Check config first
        {
            let keys = self.config_keys.read().unwrap();
            if keys.contains_key(provider) {
                return Some(CredentialSource::Config);
            }
        }

        // Check environment
        if let Some(&env_var) = self.env_mapping.get(provider) {
            if std::env::var(env_var).is_ok() {
                return Some(CredentialSource::Env(env_var));
            }
        }

        // Check auth file
        if self.auth_file_path.is_some() {
            // TODO: Check if key exists in auth file
            return Some(CredentialSource::AuthFile);
        }

        None
    }

    /// Clear all cached keys
    pub fn clear(&self) {
        let mut keys = self.config_keys.write().unwrap();
        keys.clear();
    }

    /// Check if a provider has credentials available
    pub fn has_credentials(&self, provider: &ProviderId) -> bool {
        self.get_api_key(provider).is_some()
    }

    /// Get all configured providers
    pub fn get_configured_providers(&self) -> Vec<ProviderId> {
        let keys = self.config_keys.read().unwrap();
        keys.keys().cloned().collect()
    }

    /// Validate API key format for a provider
    ///
    /// Basic validation - providers may have more specific validation
    pub fn validate_key_format(provider: &ProviderId, key: &str) -> bool {
        if key.is_empty() {
            return false;
        }

        // Basic format checks for common providers
        match provider.as_str() {
            "openai" => key.starts_with("sk-"),
            "anthropic" => key.starts_with("sk-ant-"),
            "azure" => !key.is_empty(), // Azure keys vary
            "google" => !key.is_empty(),
            _ => !key.is_empty(),
        }
    }

    /// Get environment variable name for a provider
    pub fn get_env_var_name(provider: &ProviderId) -> Option<&'static str> {
        match provider.as_str() {
            "openai" => Some("OPENAI_API_KEY"),
            "anthropic" => Some("ANTHROPIC_API_KEY"),
            "azure" => Some("AZURE_OPENAI_API_KEY"),
            "google" => Some("GOOGLE_API_KEY"),
            "vertex" => Some("GOOGLE_APPLICATION_CREDENTIALS"),
            "openrouter" => Some("OPENROUTER_API_KEY"),
            "requesty" => Some("REQUESTY_API_KEY"),
            "copilot" => Some("GITHUB_TOKEN"),
            "xai" => Some("XAI_API_KEY"),
            "mistral" => Some("MISTRAL_API_KEY"),
            "groq" => Some("GROQ_API_KEY"),
            "cerebras" => Some("CEREBRAS_API_KEY"),
            "cohere" => Some("COHERE_API_KEY"),
            "bedrock" => Some("AWS_ACCESS_KEY_ID"),
            "zhipu" => Some("ZHIPU_API_KEY"),
            "atlascloud" => Some("ATLASCLOUD_API_KEY"),
            _ => None,
        }
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for AuthManager {
    fn clone(&self) -> Self {
        AuthManager {
            config_keys: Arc::clone(&self.config_keys),
            env_mapping: self.env_mapping.clone(),
            auth_file_path: self.auth_file_path.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_manager_new() {
        let auth = AuthManager::new();
        assert!(auth.get_configured_providers().is_empty());
    }

    #[test]
    fn test_auth_manager_set_get() {
        let auth = AuthManager::new();
        auth.set_api_key(&ProviderId::new("openai"), "sk-test-key");

        let key = auth.get_api_key(&ProviderId::new("openai"));
        assert_eq!(key, Some("sk-test-key".to_string()));
    }

    #[test]
    fn test_validate_key_format_openai() {
        assert!(AuthManager::validate_key_format(
            &ProviderId::new("openai"),
            "sk-abc123"
        ));
        assert!(!AuthManager::validate_key_format(
            &ProviderId::new("openai"),
            "invalid-key"
        ));
        assert!(!AuthManager::validate_key_format(
            &ProviderId::new("openai"),
            ""
        ));
    }

    #[test]
    fn test_validate_key_format_anthropic() {
        assert!(AuthManager::validate_key_format(
            &ProviderId::new("anthropic"),
            "sk-ant-abc123"
        ));
        assert!(!AuthManager::validate_key_format(
            &ProviderId::new("anthropic"),
            "sk-abc123"
        ));
    }

    #[test]
    fn test_validate_key_format_generic() {
        assert!(AuthManager::validate_key_format(
            &ProviderId::new("azure"),
            "any-key"
        ));
        assert!(!AuthManager::validate_key_format(
            &ProviderId::new("azure"),
            ""
        ));
    }

    #[test]
    fn test_get_env_var_name() {
        assert_eq!(
            AuthManager::get_env_var_name(&ProviderId::new("openai")),
            Some("OPENAI_API_KEY")
        );
        assert_eq!(
            AuthManager::get_env_var_name(&ProviderId::new("anthropic")),
            Some("ANTHROPIC_API_KEY")
        );
        assert_eq!(
            AuthManager::get_env_var_name(&ProviderId::new("unknown")),
            None
        );
    }

    #[test]
    fn test_auth_manager_clone() {
        let auth1 = AuthManager::new();
        auth1.set_api_key(&ProviderId::new("openai"), "sk-test");

        let auth2 = auth1.clone();
        assert_eq!(
            auth2.get_api_key(&ProviderId::new("openai")),
            Some("sk-test".to_string())
        );

        auth2.set_api_key(&ProviderId::new("google"), "google-key");
        assert_eq!(
            auth1.get_api_key(&ProviderId::new("google")),
            Some("google-key".to_string())
        );
    }

    #[test]
    fn test_env_mapping_completeness() {
        let auth = AuthManager::new();

        let providers = vec![
            "openai",
            "anthropic",
            "azure",
            "google",
            "vertex",
            "openrouter",
            "copilot",
            "xai",
            "mistral",
            "groq",
        ];

        for provider_name in providers {
            let provider = ProviderId::new(provider_name);
            assert!(
                auth.env_mapping.contains_key(&provider),
                "Provider {} missing env mapping",
                provider_name
            );
        }
    }

    #[test]
    fn test_credential_source_as_str() {
        assert_eq!(CredentialSource::Config.as_str(), "config");
        assert_eq!(CredentialSource::Env("TEST").as_str(), "env");
        assert_eq!(CredentialSource::AuthFile.as_str(), "auth_file");
    }

    #[test]
    fn test_from_config() {
        let configs = vec![
            ProviderConfig::new("openai", "sk-openai-key"),
            ProviderConfig::new("anthropic", "sk-ant-anthropic-key"),
        ];

        let auth = AuthManager::from_config(&configs);
        assert_eq!(auth.get_configured_providers().len(), 2);
        assert_eq!(
            auth.get_api_key(&ProviderId::new("openai")),
            Some("sk-openai-key".to_string())
        );
        assert_eq!(
            auth.get_api_key(&ProviderId::new("anthropic")),
            Some("sk-ant-anthropic-key".to_string())
        );
    }

    #[test]
    fn test_auth_manager_clear() {
        let auth = AuthManager::new();
        auth.set_api_key(&ProviderId::new("openai"), "sk-test");
        auth.set_api_key(&ProviderId::new("google"), "test-key");

        assert_eq!(auth.get_configured_providers().len(), 2);

        auth.clear();

        assert!(auth.get_configured_providers().is_empty());
        assert!(auth.get_api_key(&ProviderId::new("openai")).is_none());
    }

    #[test]
    fn test_auth_manager_has_credentials() {
        let auth = AuthManager::new();
        assert!(!auth.has_credentials(&ProviderId::new("openai")));

        auth.set_api_key(&ProviderId::new("openai"), "sk-test");
        assert!(auth.has_credentials(&ProviderId::new("openai")));
    }

    #[test]
    fn test_auth_manager_credential_source_config() {
        let auth = AuthManager::new();
        auth.set_api_key(&ProviderId::new("openai"), "sk-test");

        let source = auth.get_credential_source(&ProviderId::new("openai"));
        assert_eq!(source, Some(CredentialSource::Config));
    }

    #[test]
    fn test_auth_manager_credential_source_none() {
        let auth = AuthManager::new();
        let source = auth.get_credential_source(&ProviderId::new("unknown-provider"));
        assert!(source.is_none());
    }

    /// Thread safety test - concurrent read/write operations
    #[test]
    fn test_thread_safety_concurrent_access() {
        use std::thread;
        use std::time::Duration;

        let auth = AuthManager::new();
        let provider = ProviderId::new("openai");

        // Set initial key
        auth.set_api_key(&provider, "initial-key");

        let auth_ref = Arc::new(auth);
        let mut handles = vec![];

        // Spawn multiple reader threads
        for _ in 0..5 {
            let auth_clone = Arc::clone(&auth_ref);
            let provider_clone = provider.clone();
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    let _ = auth_clone.get_api_key(&provider_clone);
                    thread::sleep(Duration::from_millis(1));
                }
            });
            handles.push(handle);
        }

        // Spawn writer threads
        for i in 0..3 {
            let auth_clone = Arc::clone(&auth_ref);
            let provider_clone = provider.clone();
            let handle = thread::spawn(move || {
                for j in 0..5 {
                    let key = format!("writer-{}-key-{}", i, j);
                    auth_clone.set_api_key(&provider_clone, &key);
                    thread::sleep(Duration::from_millis(2));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Verify final state is valid
        let final_key = auth_ref.get_api_key(&provider);
        assert!(final_key.is_some());
    }

    /// Test multiple providers concurrently
    #[test]
    fn test_thread_safety_multiple_providers() {
        use std::thread;

        let auth = AuthManager::new();
        let auth_ref = Arc::new(auth);
        let mut handles = vec![];

        let providers = vec![
            ("openai", "sk-openai"),
            ("anthropic", "sk-ant"),
            ("google", "google-key"),
            ("azure", "azure-key"),
            ("xai", "xai-key"),
        ];

        for (name, prefix) in &providers {
            let auth_clone = Arc::clone(&auth_ref);
            let name = name.to_string();
            let prefix = prefix.to_string();
            let handle = thread::spawn(move || {
                let provider = ProviderId::new(&name);
                for i in 0..10 {
                    let key = format!("{}-thread-{}", prefix, i);
                    auth_clone.set_api_key(&provider, &key);
                    let retrieved = auth_clone.get_api_key(&provider);
                    assert!(retrieved.is_some());
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().expect("Thread panicked");
        }

        // Verify all providers have keys
        for (name, _) in &providers {
            let provider = ProviderId::new(*name);
            assert!(auth_ref.has_credentials(&provider));
        }
    }
}
