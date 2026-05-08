//! Provider and Model ID types
//!
//! This module provides branded types for provider and model identification,
//! using the newtype pattern to prevent string misuse at compile time.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Provider ID error types
#[derive(Debug, Error)]
pub enum ProviderIdError {
    #[error("Invalid provider ID format: '{0}'")]
    InvalidFormat(String),
}

/// Provider identifier (branded type)
///
/// Using newtype pattern to prevent string misuse.
/// All provider IDs are lowercase strings.
///
/// # Example
/// ```
/// use crate::provider::ProviderId;
///
/// let id = ProviderId::new("openai");
/// assert_eq!(id.as_str(), "openai");
/// assert_eq!(format!("{}", id), "openai");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(String);

impl ProviderId {
    /// Create a new ProviderId
    ///
    /// The ID will be converted to lowercase to ensure consistency.
    pub fn new(id: impl Into<String>) -> Self {
        let id_str = id.into().to_lowercase();
        ProviderId(id_str)
    }

    /// Get the underlying string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate provider ID format
    ///
    /// Provider IDs must be non-empty and contain only alphanumeric
    /// characters, hyphens, and underscores.
    pub fn validate(id: &str) -> Result<(), ProviderIdError> {
        if id.is_empty() {
            return Err(ProviderIdError::InvalidFormat(
                "Provider ID cannot be empty".to_string(),
            ));
        }

        if !id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ProviderIdError::InvalidFormat(format!(
                "Provider ID '{}' contains invalid characters",
                id
            )));
        }

        Ok(())
    }

    /// Create an empty ProviderId (used as default).
    pub const fn empty() -> Self {
        ProviderId(String::new())
    }

    /// Get static provider ID for OpenAI
    pub fn openai() -> &'static ProviderId {
        static OPENAI: Lazy<ProviderId> = Lazy::new(|| ProviderId("openai".to_string()));
        &OPENAI
    }

    /// Get static provider ID for Anthropic
    pub fn anthropic() -> &'static ProviderId {
        static ANTHROPIC: Lazy<ProviderId> = Lazy::new(|| ProviderId("anthropic".to_string()));
        &ANTHROPIC
    }

    /// Get static provider ID for Azure
    pub fn azure() -> &'static ProviderId {
        static AZURE: Lazy<ProviderId> = Lazy::new(|| ProviderId("azure".to_string()));
        &AZURE
    }

    /// Get static provider ID for Google
    pub fn google() -> &'static ProviderId {
        static GOOGLE: Lazy<ProviderId> = Lazy::new(|| ProviderId("google".to_string()));
        &GOOGLE
    }

    /// Get static provider ID for Vertex
    pub fn vertex() -> &'static ProviderId {
        static VERTEX: Lazy<ProviderId> = Lazy::new(|| ProviderId("vertex".to_string()));
        &VERTEX
    }

    /// Get static provider ID for OpenRouter
    pub fn openrouter() -> &'static ProviderId {
        static OPENROUTER: Lazy<ProviderId> = Lazy::new(|| ProviderId("openrouter".to_string()));
        &OPENROUTER
    }

    /// Get static provider ID for Copilot
    pub fn copilot() -> &'static ProviderId {
        static COPILOT: Lazy<ProviderId> = Lazy::new(|| ProviderId("copilot".to_string()));
        &COPILOT
    }

    /// Get static provider ID for XAI
    pub fn xai() -> &'static ProviderId {
        static XAI: Lazy<ProviderId> = Lazy::new(|| ProviderId("xai".to_string()));
        &XAI
    }

    /// Get static provider ID for Mistral
    pub fn mistral() -> &'static ProviderId {
        static MISTRAL: Lazy<ProviderId> = Lazy::new(|| ProviderId("mistral".to_string()));
        &MISTRAL
    }

    /// Get static provider ID for Groq
    pub fn groq() -> &'static ProviderId {
        static GROQ: Lazy<ProviderId> = Lazy::new(|| ProviderId("groq".to_string()));
        &GROQ
    }

    /// Get static provider ID for Cerebras
    pub fn cerebras() -> &'static ProviderId {
        static CEREBRAS: Lazy<ProviderId> = Lazy::new(|| ProviderId("cerebras".to_string()));
        &CEREBRAS
    }

    /// Get static provider ID for Cohere
    pub fn cohere() -> &'static ProviderId {
        static COHERE: Lazy<ProviderId> = Lazy::new(|| ProviderId("cohere".to_string()));
        &COHERE
    }

    /// Get static provider ID for Bedrock
    pub fn bedrock() -> &'static ProviderId {
        static BEDROCK: Lazy<ProviderId> = Lazy::new(|| ProviderId("bedrock".to_string()));
        &BEDROCK
    }

    /// Get static provider ID for Zhipu
    pub fn zhipu() -> &'static ProviderId {
        static ZHIPU: Lazy<ProviderId> = Lazy::new(|| ProviderId("zhipu".to_string()));
        &ZHIPU
    }
}

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ProviderId {
    fn from(s: &str) -> Self {
        ProviderId::new(s)
    }
}

impl From<String> for ProviderId {
    fn from(s: String) -> Self {
        ProviderId::new(s)
    }
}

impl AsRef<str> for ProviderId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Model ID parse error types
#[derive(Debug, Error)]
pub enum ModelIdParseError {
    #[error("Invalid model ID format: '{0}' (expected: provider/name)")]
    InvalidFormat(String),

    #[error("Unknown provider: '{0}'")]
    UnknownProvider(String),
}

/// Model identifier (branded type)
///
/// Format: `provider/model-name` (e.g., "openai/gpt-4")
///
/// # Example
/// ```
/// use crate::provider::ModelId;
///
/// let model = ModelId::new("openai", "gpt-4");
/// assert_eq!(model.as_str(), "openai/gpt-4");
/// assert_eq!(model.provider().as_str(), "openai");
/// assert_eq!(model.name(), "gpt-4");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId {
    provider: ProviderId,
    name: String,
    #[serde(skip)]
    full: String, // Cached full string "provider/name"
}

impl ModelId {
    /// Create a new ModelId
    pub fn new(provider: impl Into<ProviderId>, name: impl Into<String>) -> Self {
        let provider = provider.into();
        let name = name.into();
        let full = format!("{}/{}", provider.as_str(), name);

        ModelId {
            provider,
            name,
            full,
        }
    }

    /// Parse a model ID from a string (e.g., "openai/gpt-4")
    pub fn parse(s: &str) -> Result<Self, ModelIdParseError> {
        let parts: Vec<&str> = s.splitn(2, '/').collect();

        if parts.len() != 2 {
            return Err(ModelIdParseError::InvalidFormat(s.to_string()));
        }

        let provider = ProviderId::new(parts[0]);
        let name = parts[1].to_string();

        if name.is_empty() {
            return Err(ModelIdParseError::InvalidFormat(s.to_string()));
        }

        Ok(ModelId::new(provider, name))
    }

    /// Get the provider ID
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Get the model name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the full string representation
    pub fn as_str(&self) -> &str {
        &self.full
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        ModelId::parse(s).unwrap_or_else(|_| ModelId::new("unknown", s))
    }
}

impl From<String> for ModelId {
    fn from(s: String) -> Self {
        ModelId::parse(&s).unwrap_or_else(|_| ModelId::new("unknown", s))
    }
}

/// Parse a model string into a ModelId
///
/// This is a convenience function that wraps ModelId::parse()
pub fn parse_model_string(s: &str) -> Result<ModelId, ModelIdParseError> {
    ModelId::parse(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id_new() {
        let id = ProviderId::new("openai");
        assert_eq!(id.as_str(), "openai");
    }

    #[test]
    fn test_provider_id_lowercase_conversion() {
        let id = ProviderId::new("OpenAI");
        assert_eq!(id.as_str(), "openai");

        let id = ProviderId::new("ANTHROPIC");
        assert_eq!(id.as_str(), "anthropic");
    }

    #[test]
    fn test_provider_id_display() {
        let id = ProviderId::new("anthropic");
        assert_eq!(format!("{}", id), "anthropic");
    }

    #[test]
    fn test_provider_id_from_str() {
        let id: ProviderId = "openai".into();
        assert_eq!(id.as_str(), "openai");

        let id: ProviderId = String::from("google").into();
        assert_eq!(id.as_str(), "google");
    }

    #[test]
    fn test_provider_id_as_ref() {
        let id = ProviderId::new("openai");
        let s: &str = id.as_ref();
        assert_eq!(s, "openai");
    }

    #[test]
    fn test_provider_id_eq_and_hash() {
        let id1 = ProviderId::new("openai");
        let id2 = ProviderId::new("openai");
        let id3 = ProviderId::new("OpenAI");

        assert_eq!(id1, id2);
        assert_eq!(id1, id3);

        let mut set = std::collections::HashSet::new();
        set.insert(id1);
        assert!(set.contains(&id2));
    }

    #[test]
    fn test_provider_id_validate_empty() {
        let result = ProviderId::validate("");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProviderIdError::InvalidFormat(_)
        ));
    }

    #[test]
    fn test_provider_id_validate_invalid_chars() {
        let result = ProviderId::validate("open@ai");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ProviderIdError::InvalidFormat(_)
        ));
    }

    #[test]
    fn test_provider_id_validate_valid() {
        assert!(ProviderId::validate("openai").is_ok());
        assert!(ProviderId::validate("azure-openai").is_ok());
        assert!(ProviderId::validate("google_vertex").is_ok());
    }

    #[test]
    fn test_provider_id_static_accessors() {
        assert_eq!(ProviderId::openai().as_str(), "openai");
        assert_eq!(ProviderId::anthropic().as_str(), "anthropic");
        assert_eq!(ProviderId::azure().as_str(), "azure");
        assert_eq!(ProviderId::google().as_str(), "google");
    }

    #[test]
    fn test_model_id_new() {
        let model = ModelId::new("openai", "gpt-4");
        assert_eq!(model.provider().as_str(), "openai");
        assert_eq!(model.name(), "gpt-4");
        assert_eq!(model.as_str(), "openai/gpt-4");
    }

    #[test]
    fn test_model_id_display() {
        let model = ModelId::new("anthropic", "claude-3-opus");
        assert_eq!(format!("{}", model), "anthropic/claude-3-opus");
    }

    #[test]
    fn test_model_id_parse_valid() {
        let model = ModelId::parse("openai/gpt-4").unwrap();
        assert_eq!(model.provider().as_str(), "openai");
        assert_eq!(model.name(), "gpt-4");
        assert_eq!(model.as_str(), "openai/gpt-4");
    }

    #[test]
    fn test_model_id_parse_complex_names() {
        let model = ModelId::parse("azure/gpt-4-turbo-preview").unwrap();
        assert_eq!(model.provider().as_str(), "azure");
        assert_eq!(model.name(), "gpt-4-turbo-preview");

        let model = ModelId::parse("google/gemini-1.5-pro").unwrap();
        assert_eq!(model.provider().as_str(), "google");
        assert_eq!(model.name(), "gemini-1.5-pro");
    }

    #[test]
    fn test_model_id_parse_invalid_no_slash() {
        let result = ModelId::parse("gpt-4");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ModelIdParseError::InvalidFormat(_)
        ));
    }

    #[test]
    fn test_model_id_parse_invalid_empty_name() {
        let result = ModelId::parse("openai/");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ModelIdParseError::InvalidFormat(_)
        ));
    }

    #[test]
    fn test_model_id_from_str() {
        let model: ModelId = "openai/gpt-4".into();
        assert_eq!(model.as_str(), "openai/gpt-4");

        let model: ModelId = String::from("anthropic/claude-3").into();
        assert_eq!(model.as_str(), "anthropic/claude-3");
    }

    #[test]
    fn test_model_id_from_str_invalid() {
        let model: ModelId = "invalid-format".into();
        // Should fallback to "unknown/invalid-format"
        assert_eq!(model.provider().as_str(), "unknown");
        assert_eq!(model.name(), "invalid-format");
    }

    #[test]
    fn test_model_id_eq_and_hash() {
        let model1 = ModelId::new("openai", "gpt-4");
        let model2 = ModelId::new("openai", "gpt-4");
        let model3 = ModelId::new("anthropic", "claude-3");

        assert_eq!(model1, model2);
        assert_ne!(model1, model3);

        let mut set = std::collections::HashSet::new();
        set.insert(model1);
        assert!(set.contains(&model2));
        assert!(!set.contains(&model3));
    }

    #[test]
    fn test_parse_model_string_function() {
        let model = parse_model_string("openai/gpt-4").unwrap();
        assert_eq!(model.provider().as_str(), "openai");
        assert_eq!(model.name(), "gpt-4");

        let result = parse_model_string("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_model_id_serialization() {
        let model = ModelId::new("openai", "gpt-4");
        let json = serde_json::to_string(&model).unwrap();
        assert!(json.contains("\"provider\":\"openai\""));
        assert!(json.contains("\"name\":\"gpt-4\""));

        let deserialized: ModelId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider().as_str(), "openai");
        assert_eq!(deserialized.name(), "gpt-4");
    }

    #[test]
    fn test_provider_id_serialization() {
        let id = ProviderId::new("anthropic");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"anthropic\"");

        let deserialized: ProviderId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), "anthropic");
    }

    #[test]
    fn test_model_id_with_special_characters() {
        let model = ModelId::new("openai", "gpt-4o-mini");
        assert_eq!(model.name(), "gpt-4o-mini");

        let model = ModelId::parse("xai/grok-beta").unwrap();
        assert_eq!(model.name(), "grok-beta");
    }

    #[test]
    fn test_provider_id_multiple_transformations() {
        let id1 = ProviderId::new("OpenAI");
        let id2 = ProviderId::new("openai");
        let id3 = ProviderId::new("OPENAI");

        assert_eq!(id1, id2);
        assert_eq!(id2, id3);
        assert_eq!(id1, id3);
    }
}
