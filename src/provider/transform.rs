//! Message transformation pipeline
//!
//! This module provides message normalization and transformation
//! for provider-specific formats.

use serde::{Deserialize, Serialize};

use super::id::ProviderId;
use super::message::Message;
use super::provider_trait::ProviderToolCall;
use crate::provider::error::Result;

/// Provider-agnostic message format for transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMessage {
    /// Message role (system, user, assistant, tool)
    pub role: String,

    /// Message content
    pub content: String,

    /// Tool call ID (for tool responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// Tool calls (for assistant messages)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ProviderToolCall>,

    /// Name field (for certain provider formats)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ProviderMessage {
    /// Create a new ProviderMessage
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        ProviderMessage {
            role: role.into(),
            content: content.into(),
            tool_call_id: None,
            tool_calls: vec![],
            name: None,
        }
    }

    /// Set tool call ID
    pub fn with_tool_call_id(mut self, id: impl Into<String>) -> Self {
        self.tool_call_id = Some(id.into());
        self
    }

    /// Set tool calls
    pub fn with_tool_calls(mut self, calls: Vec<ProviderToolCall>) -> Self {
        self.tool_calls = calls;
        self
    }

    /// Set name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Message normalizer trait
///
/// Converts internal Message format to provider-specific format.
pub trait MessageNormalizer: Send + Sync {
    /// Normalize a single message
    fn normalize(&self, message: &Message) -> Result<ProviderMessage>;

    /// Check if this normalizer supports the provider
    fn supports(&self, provider: &ProviderId) -> bool;

    /// Get the normalizer name
    fn name(&self) -> &'static str;
}

/// Message transformation pipeline
///
/// Chains multiple normalizers and applies the appropriate one
/// based on the target provider.
pub struct TransformPipeline {
    normalizers: Vec<Box<dyn MessageNormalizer>>,
}

impl TransformPipeline {
    /// Create a new TransformPipeline with default normalizers
    pub fn new() -> Self {
        let mut pipeline = TransformPipeline {
            normalizers: Vec::new(),
        };

        // Add default normalizers
        pipeline.add_normalizer(Box::new(OpenAiNormalizer));
        pipeline.add_normalizer(Box::new(AnthropicNormalizer));
        pipeline.add_normalizer(Box::new(GenericNormalizer));

        pipeline
    }

    /// Create an empty TransformPipeline
    pub fn empty() -> Self {
        TransformPipeline {
            normalizers: Vec::new(),
        }
    }

    /// Add a normalizer to the pipeline
    pub fn add_normalizer(&mut self, normalizer: Box<dyn MessageNormalizer>) {
        self.normalizers.push(normalizer);
    }

    /// Transform messages for a specific provider
    ///
    /// # Arguments
    /// * `messages` - Vector of internal Message objects
    /// * `provider` - Target provider ID
    ///
    /// # Returns
    /// * `Result<Vec<ProviderMessage>>` - Transformed messages
    pub fn transform(
        &self,
        messages: Vec<Message>,
        provider: &ProviderId,
    ) -> Result<Vec<ProviderMessage>> {
        // Find the appropriate normalizer
        let normalizer = self
            .normalizers
            .iter()
            .find(|n| n.supports(provider))
            .unwrap_or_else(|| {
                // Fallback to generic normalizer
                self.normalizers
                    .iter()
                    .find(|n| n.name() == "generic")
                    .expect("Generic normalizer should always be present")
            });

        // Transform all messages
        messages
            .into_iter()
            .map(|msg| normalizer.normalize(&msg))
            .collect()
    }

    /// Get the number of normalizers in the pipeline
    pub fn len(&self) -> usize {
        self.normalizers.len()
    }

    /// Check if the pipeline is empty
    pub fn is_empty(&self) -> bool {
        self.normalizers.is_empty()
    }
}

impl Default for TransformPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// OpenAI message normalizer
///
/// Direct passthrough - OpenAI format matches our internal format closely.
pub struct OpenAiNormalizer;

impl MessageNormalizer for OpenAiNormalizer {
    fn normalize(&self, message: &Message) -> Result<ProviderMessage> {
        let mut pm = ProviderMessage::new(message.role.as_str(), message.content.clone());

        if let Some(ref id) = message.tool_call_id {
            pm.tool_call_id = Some(id.clone());
        }

        // Copy tool calls directly
        if !message.tool_calls.is_empty() {
            pm.tool_calls = message.tool_calls.clone();
        }

        Ok(pm)
    }

    fn supports(&self, provider: &ProviderId) -> bool {
        matches!(
            provider.as_str(),
            "openai" | "azure" | "openrouter" | "requesty" | "groq" | "cerebras"
        )
    }

    fn name(&self) -> &'static str {
        "openai"
    }
}

/// Anthropic message normalizer
///
/// Special handling for Anthropic:
/// - System messages are extracted separately
/// - User/Assistant messages must alternate
/// - Tool calls use content blocks
pub struct AnthropicNormalizer;

impl AnthropicNormalizer {
    /// Extract system messages from a list
    fn extract_system_messages(messages: &[Message]) -> Vec<String> {
        messages
            .iter()
            .filter(|m| m.role == crate::provider::message::MessageRole::System)
            .map(|m| m.content.clone())
            .collect()
    }
}

impl MessageNormalizer for AnthropicNormalizer {
    fn normalize(&self, message: &Message) -> Result<ProviderMessage> {
        use crate::provider::message::MessageRole;

        match message.role {
            MessageRole::System => {
                // System messages are handled separately in Anthropic
                // Return as-is but they'll be extracted
                Ok(ProviderMessage::new("system", message.content.clone()))
            }
            MessageRole::User | MessageRole::Assistant => {
                let mut pm = ProviderMessage::new(message.role.as_str(), message.content.clone());

                // Copy tool calls directly for Anthropic content blocks
                if !message.tool_calls.is_empty() {
                    pm.tool_calls = message.tool_calls.clone();
                }

                Ok(pm)
            }
            MessageRole::Tool => {
                // Tool responses in Anthropic use special format
                let mut pm = ProviderMessage::new("user", message.content.clone());
                if let Some(ref id) = message.tool_call_id {
                    pm.tool_call_id = Some(id.clone());
                }
                Ok(pm)
            }
        }
    }

    fn supports(&self, provider: &ProviderId) -> bool {
        provider.as_str() == "anthropic"
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }
}

/// Generic normalizer (fallback)
///
/// Basic passthrough for providers without special requirements.
pub struct GenericNormalizer;

impl MessageNormalizer for GenericNormalizer {
    fn normalize(&self, message: &Message) -> Result<ProviderMessage> {
        let mut pm = ProviderMessage::new(message.role.as_str(), message.content.clone());

        if let Some(ref id) = message.tool_call_id {
            pm = pm.with_tool_call_id(id.clone());
        }

        Ok(pm)
    }

    fn supports(&self, _provider: &ProviderId) -> bool {
        // Generic normalizer supports all providers (fallback)
        true
    }

    fn name(&self) -> &'static str {
        "generic"
    }
}

/// Extract system messages from a message list
///
/// Helper function for providers that handle system messages separately.
pub fn extract_system_messages(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .filter(|m| m.role == crate::provider::message::MessageRole::System)
        .map(|m| m.content.clone())
        .collect()
}

/// Filter out system messages from a message list
///
/// Helper function for providers that handle system messages separately.
pub fn filter_system_messages(messages: Vec<Message>) -> Vec<Message> {
    messages
        .into_iter()
        .filter(|m| m.role != crate::provider::message::MessageRole::System)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::message::MessageRole;

    #[test]
    fn test_provider_message_new() {
        let pm = ProviderMessage::new("user", "Hello");
        assert_eq!(pm.role, "user");
        assert_eq!(pm.content, "Hello");
        assert!(pm.tool_call_id.is_none());
        assert!(pm.tool_calls.is_empty());
    }

    #[test]
    fn test_provider_message_with_tool_call_id() {
        let pm = ProviderMessage::new("tool", "Result").with_tool_call_id("call_123");
        assert_eq!(pm.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_provider_message_with_tool_calls() {
        use crate::provider::provider_trait::{ProviderToolCall, ProviderToolCallFunction};

        let tool_call = ProviderToolCall {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: ProviderToolCallFunction {
                name: "test".to_string(),
                arguments: "{}".to_string(),
            },
        };

        let pm =
            ProviderMessage::new("assistant", "Content").with_tool_calls(vec![tool_call.clone()]);

        assert_eq!(pm.tool_calls.len(), 1);
        assert_eq!(pm.tool_calls[0].id, "call_1");
        assert_eq!(pm.tool_calls[0].function.name, "test");
    }

    #[test]
    fn test_openai_normalizer_supports() {
        let normalizer = OpenAiNormalizer;

        assert!(normalizer.supports(&ProviderId::new("openai")));
        assert!(normalizer.supports(&ProviderId::new("azure")));
        assert!(normalizer.supports(&ProviderId::new("openrouter")));
        assert!(normalizer.supports(&ProviderId::new("requesty")));
        assert!(!normalizer.supports(&ProviderId::new("anthropic")));
    }

    #[test]
    fn test_openai_normalizer_normalize() {
        let normalizer = OpenAiNormalizer;
        let msg = Message::user("Hello");

        let result = normalizer.normalize(&msg).unwrap();
        assert_eq!(result.role, "user");
        assert_eq!(result.content, "Hello");
    }

    #[test]
    fn test_anthropic_normalizer_supports() {
        let normalizer = AnthropicNormalizer;

        assert!(normalizer.supports(&ProviderId::new("anthropic")));
        assert!(!normalizer.supports(&ProviderId::new("openai")));
    }

    #[test]
    fn test_anthropic_normalizer_system() {
        let normalizer = AnthropicNormalizer;
        let msg = Message::system("You are helpful");

        let result = normalizer.normalize(&msg).unwrap();
        assert_eq!(result.role, "system");
        assert_eq!(result.content, "You are helpful");
    }

    #[test]
    fn test_generic_normalizer_supports() {
        let normalizer = GenericNormalizer;

        // Generic supports everything
        assert!(normalizer.supports(&ProviderId::new("unknown")));
        assert!(normalizer.supports(&ProviderId::new("openai")));
        assert!(normalizer.supports(&ProviderId::new("custom-provider")));
    }

    #[test]
    fn test_transform_pipeline_new() {
        let pipeline = TransformPipeline::new();
        assert!(!pipeline.is_empty());
        assert!(pipeline.len() >= 3); // Should have at least 3 normalizers
    }

    #[test]
    fn test_transform_pipeline_empty() {
        let pipeline = TransformPipeline::empty();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[test]
    fn test_transform_pipeline_add_normalizer() {
        let mut pipeline = TransformPipeline::empty();
        assert!(pipeline.is_empty());

        pipeline.add_normalizer(Box::new(GenericNormalizer));
        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.len(), 1);
    }

    #[test]
    fn test_transform_pipeline_transform_openai() {
        let pipeline = TransformPipeline::new();
        let messages = vec![Message::user("Hello"), Message::assistant("Hi there")];

        let result = pipeline.transform(messages, &ProviderId::new("openai"));
        assert!(result.is_ok());

        let transformed = result.unwrap();
        assert_eq!(transformed.len(), 2);
        assert_eq!(transformed[0].role, "user");
        assert_eq!(transformed[1].role, "assistant");
    }

    #[test]
    fn test_transform_pipeline_transform_anthropic() {
        let pipeline = TransformPipeline::new();
        let messages = vec![
            Message::system("You are Claude"),
            Message::user("Hello"),
            Message::assistant("Hi"),
        ];

        let result = pipeline.transform(messages, &ProviderId::new("anthropic"));
        assert!(result.is_ok());

        let transformed = result.unwrap();
        assert_eq!(transformed.len(), 3);
    }

    #[test]
    fn test_transform_pipeline_transform_unknown() {
        let pipeline = TransformPipeline::new();
        let messages = vec![Message::user("Test")];

        // Should fallback to generic normalizer
        let result = pipeline.transform(messages, &ProviderId::new("unknown-provider"));
        assert!(result.is_ok());

        let transformed = result.unwrap();
        assert_eq!(transformed.len(), 1);
        assert_eq!(transformed[0].role, "user");
    }

    #[test]
    fn test_extract_system_messages() {
        let messages = vec![
            Message::system("System 1"),
            Message::user("User"),
            Message::system("System 2"),
            Message::assistant("Assistant"),
        ];

        let systems = extract_system_messages(&messages);
        assert_eq!(systems.len(), 2);
        assert_eq!(systems[0], "System 1");
        assert_eq!(systems[1], "System 2");
    }

    #[test]
    fn test_filter_system_messages() {
        let messages = vec![
            Message::system("System"),
            Message::user("User"),
            Message::assistant("Assistant"),
        ];

        let filtered = filter_system_messages(messages);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].role, MessageRole::User);
        assert_eq!(filtered[1].role, MessageRole::Assistant);
    }

    #[test]
    fn test_normalizer_names() {
        assert_eq!(OpenAiNormalizer.name(), "openai");
        assert_eq!(AnthropicNormalizer.name(), "anthropic");
        assert_eq!(GenericNormalizer.name(), "generic");
    }

    #[test]
    fn test_message_normalization_with_tool_calls() {
        use crate::provider::provider_trait::ProviderToolCall;
        use crate::provider::provider_trait::ProviderToolCallFunction;

        let tool_call = ProviderToolCall {
            id: "call_123".to_string(),
            call_type: "function".to_string(),
            function: ProviderToolCallFunction {
                name: "test_func".to_string(),
                arguments: "{}".to_string(),
            },
        };

        let msg = Message::assistant_with_tool_calls("Using tool", vec![tool_call]);
        let normalizer = OpenAiNormalizer;

        let result = normalizer.normalize(&msg).unwrap();
        assert_eq!(result.role, "assistant");
        assert!(!result.tool_calls.is_empty());
    }
}
