//! Model capabilities, costs, and limits
//!
//! This module defines structures for describing model capabilities,
//! pricing, and usage limits.

use serde::{Deserialize, Serialize};

use super::id::ModelId;

/// Model capabilities description
///
/// Defines what features and modalities a model supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Context window size in tokens
    pub context_window: u32,

    /// Maximum output tokens
    pub max_output_tokens: u32,

    /// Whether the model supports streaming responses
    #[serde(default)]
    pub supports_streaming: bool,

    /// Whether the model supports tool/function calling
    #[serde(default)]
    pub supports_tools: bool,

    /// Whether the model supports vision/image input
    #[serde(default)]
    pub supports_vision: bool,

    /// Whether the model supports function calling
    #[serde(default, rename = "supports_function_call")]
    pub supports_function_call: bool,

    /// Supported media types for vision models (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_media_types: Option<Vec<String>>,
}

impl ModelCapabilities {
    /// Create a new ModelCapabilities with minimal defaults
    pub fn new(context_window: u32, max_output_tokens: u32) -> Self {
        ModelCapabilities {
            context_window,
            max_output_tokens,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
            supports_function_call: false,
            supported_media_types: None,
        }
    }

    /// Check if the model supports a specific media type
    pub fn supports_media_type(&self, media_type: &str) -> bool {
        self.supported_media_types
            .as_ref()
            .map(|types| types.iter().any(|t| t == media_type))
            .unwrap_or(false)
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        ModelCapabilities {
            context_window: 4096,
            max_output_tokens: 2048,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: false,
            supports_function_call: false,
            supported_media_types: None,
        }
    }
}

/// Cost structure (per 1M tokens)
///
/// All costs are in USD.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    /// Input cost (USD per 1M tokens)
    pub input: f64,

    /// Output cost (USD per 1M tokens)
    pub output: f64,

    /// Cache read cost (USD per 1M tokens, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<f64>,

    /// Cache write cost (USD per 1M tokens, optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<f64>,
}

impl Cost {
    /// Create a new Cost with input and output prices
    pub fn new(input: f64, output: f64) -> Self {
        Cost {
            input,
            output,
            cache_read: None,
            cache_write: None,
        }
    }

    /// Create a new Cost with all pricing tiers
    pub fn with_cache(input: f64, output: f64, cache_read: f64, cache_write: f64) -> Self {
        Cost {
            input,
            output,
            cache_read: Some(cache_read),
            cache_write: Some(cache_write),
        }
    }

    /// Calculate the cost for a single request
    ///
    /// # Arguments
    /// * `input_tokens` - Number of input tokens
    /// * `output_tokens` - Number of output tokens
    ///
    /// # Returns
    /// * `f64` - Total cost in USD
    ///
    /// # Example
    /// ```
    /// use crate::provider::capability::Cost;
    ///
    /// let cost = Cost::new(0.03, 0.06); // $0.03/$0.06 per 1M tokens
    /// let total = cost.calculate(1_000_000, 500_000);
    /// assert!((total - 0.06).abs() < 1e-10); // $0.03 + $0.03 = $0.06
    /// ```
    pub fn calculate(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output;
        input_cost + output_cost
    }

    /// Calculate cost with cache tokens
    ///
    /// # Arguments
    /// * `input_tokens` - Number of input tokens
    /// * `output_tokens` - Number of output tokens
    /// * `cache_read_tokens` - Number of cache read tokens
    /// * `cache_write_tokens` - Number of cache write tokens
    ///
    /// # Returns
    /// * `f64` - Total cost in USD
    pub fn calculate_with_cache(
        &self,
        input_tokens: u32,
        output_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
    ) -> f64 {
        let base_cost = self.calculate(input_tokens, output_tokens);

        let cache_read_cost = self
            .cache_read
            .map(|rate| (cache_read_tokens as f64 / 1_000_000.0) * rate)
            .unwrap_or(0.0);

        let cache_write_cost = self
            .cache_write
            .map(|rate| (cache_write_tokens as f64 / 1_000_000.0) * rate)
            .unwrap_or(0.0);

        base_cost + cache_read_cost + cache_write_cost
    }
}

impl Default for Cost {
    fn default() -> Self {
        Cost {
            input: 0.0,
            output: 0.0,
            cache_read: None,
            cache_write: None,
        }
    }
}

/// Model rate limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelLimits {
    /// Requests per minute limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u32>,

    /// Tokens per minute limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tpm: Option<u32>,

    /// Requests per day limit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpd: Option<u32>,

    /// Maximum concurrent requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
}

impl ModelLimits {
    /// Create a new ModelLimits
    pub fn new() -> Self {
        ModelLimits {
            rpm: None,
            tpm: None,
            rpd: None,
            max_concurrent: None,
        }
    }

    /// Set RPM limit
    pub fn with_rpm(mut self, rpm: u32) -> Self {
        self.rpm = Some(rpm);
        self
    }

    /// Set TPM limit
    pub fn with_tpm(mut self, tpm: u32) -> Self {
        self.tpm = Some(tpm);
        self
    }

    /// Set RPD limit
    pub fn with_rpd(mut self, rpd: u32) -> Self {
        self.rpd = Some(rpd);
        self
    }

    /// Set max concurrent requests
    pub fn with_max_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent = Some(max);
        self
    }
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete model definition
///
/// Combines capabilities, costs, and limits for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDefinition {
    /// Model ID
    pub id: ModelId,

    /// Display name for the model
    pub display_name: String,

    /// Model capabilities
    pub capabilities: ModelCapabilities,

    /// Cost structure (optional for free models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<Cost>,

    /// Rate limits (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<ModelLimits>,

    /// Whether the model is enabled by default
    #[serde(default)]
    pub enabled: bool,

    /// Deprecation warning message (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation_warning: Option<String>,
}

impl ModelDefinition {
    /// Create a new ModelDefinition
    pub fn new(
        id: ModelId,
        display_name: impl Into<String>,
        capabilities: ModelCapabilities,
    ) -> Self {
        ModelDefinition {
            id,
            display_name: display_name.into(),
            capabilities,
            cost: None,
            limits: None,
            enabled: true,
            deprecation_warning: None,
        }
    }

    /// Set the cost structure
    pub fn with_cost(mut self, cost: Cost) -> Self {
        self.cost = Some(cost);
        self
    }

    /// Set the rate limits
    pub fn with_limits(mut self, limits: ModelLimits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Set whether the model is enabled
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set a deprecation warning
    pub fn with_deprecation_warning(mut self, warning: impl Into<String>) -> Self {
        self.deprecation_warning = Some(warning.into());
        self
    }

    /// Check if the model is deprecated
    pub fn is_deprecated(&self) -> bool {
        self.deprecation_warning.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_capabilities_new() {
        let caps = ModelCapabilities::new(8192, 4096);
        assert_eq!(caps.context_window, 8192);
        assert_eq!(caps.max_output_tokens, 4096);
        assert!(caps.supports_streaming);
        assert!(!caps.supports_tools);
    }

    #[test]
    fn test_model_capabilities_default() {
        let caps = ModelCapabilities::default();
        assert_eq!(caps.context_window, 4096);
        assert_eq!(caps.max_output_tokens, 2048);
    }

    #[test]
    fn test_model_capabilities_supports_media_type() {
        let caps = ModelCapabilities {
            context_window: 8192,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_tools: false,
            supports_vision: true,
            supports_function_call: false,
            supported_media_types: Some(vec!["image/jpeg".to_string(), "image/png".to_string()]),
        };

        assert!(caps.supports_media_type("image/jpeg"));
        assert!(caps.supports_media_type("image/png"));
        assert!(!caps.supports_media_type("image/gif"));
    }

    #[test]
    fn test_cost_new() {
        let cost = Cost::new(0.03, 0.06);
        assert_eq!(cost.input, 0.03);
        assert_eq!(cost.output, 0.06);
        assert!(cost.cache_read.is_none());
        assert!(cost.cache_write.is_none());
    }

    #[test]
    fn test_cost_with_cache() {
        let cost = Cost::with_cache(0.015, 0.075, 0.0075, 0.015);
        assert_eq!(cost.input, 0.015);
        assert_eq!(cost.output, 0.075);
        assert_eq!(cost.cache_read, Some(0.0075));
        assert_eq!(cost.cache_write, Some(0.015));
    }

    #[test]
    fn test_cost_calculation() {
        let cost = Cost::new(0.03, 0.06);

        // 1M input tokens = $0.03
        let calculated = cost.calculate(1_000_000, 0);
        assert!((calculated - 0.03).abs() < 1e-10);

        // 500K output tokens = $0.03
        let calculated = cost.calculate(0, 500_000);
        assert!((calculated - 0.03).abs() < 1e-10);

        // 1M input + 500K output = $0.06
        let calculated = cost.calculate(1_000_000, 500_000);
        assert!((calculated - 0.06).abs() < 1e-10);
    }

    #[test]
    fn test_cost_calculation_with_cache() {
        let cost = Cost::with_cache(0.03, 0.06, 0.01, 0.02);

        let calculated = cost.calculate_with_cache(1_000_000, 500_000, 200_000, 100_000);
        // Input: $0.03, Output: $0.03, Cache read: $0.002, Cache write: $0.002
        // Total: $0.064
        assert!((calculated - 0.064).abs() < 1e-10);
    }

    #[test]
    fn test_cost_calculation_small_tokens() {
        let cost = Cost::new(0.03, 0.06);

        // 1000 input + 500 output
        let calculated = cost.calculate(1000, 500);
        // (1000/1M * 0.03) + (500/1M * 0.06) = 0.00003 + 0.00003 = 0.00006
        assert!((calculated - 0.00006).abs() < 1e-10);
    }

    #[test]
    fn test_cost_default() {
        let cost = Cost::default();
        assert_eq!(cost.input, 0.0);
        assert_eq!(cost.output, 0.0);
    }

    #[test]
    fn test_model_limits_new() {
        let limits = ModelLimits::new();
        assert!(limits.rpm.is_none());
        assert!(limits.tpm.is_none());
        assert!(limits.rpd.is_none());
        assert!(limits.max_concurrent.is_none());
    }

    #[test]
    fn test_model_limits_builder() {
        let limits = ModelLimits::new()
            .with_rpm(100)
            .with_tpm(50_000)
            .with_rpd(10_000)
            .with_max_concurrent(10);

        assert_eq!(limits.rpm, Some(100));
        assert_eq!(limits.tpm, Some(50_000));
        assert_eq!(limits.rpd, Some(10_000));
        assert_eq!(limits.max_concurrent, Some(10));
    }

    #[test]
    fn test_model_limits_default() {
        let limits = ModelLimits::default();
        assert!(limits.rpm.is_none());
    }

    #[test]
    fn test_model_definition_new() {
        let id = ModelId::new("openai", "gpt-4");
        let caps = ModelCapabilities::new(8192, 4096);
        let def = ModelDefinition::new(id, "GPT-4", caps);

        assert_eq!(def.id.as_str(), "openai/gpt-4");
        assert_eq!(def.display_name, "GPT-4");
        assert_eq!(def.capabilities.context_window, 8192);
        assert!(def.enabled);
        assert!(def.cost.is_none());
        assert!(def.limits.is_none());
    }

    #[test]
    fn test_model_definition_builder() {
        let id = ModelId::new("anthropic", "claude-3-opus");
        let caps = ModelCapabilities::new(200_000, 4096);
        let cost = Cost::new(0.015, 0.075);
        let limits = ModelLimits::new().with_rpm(50);

        let def = ModelDefinition::new(id, "Claude 3 Opus", caps)
            .with_cost(cost)
            .with_limits(limits)
            .with_deprecation_warning("Use claude-4-opus instead");

        assert_eq!(def.display_name, "Claude 3 Opus");
        assert!(def.cost.is_some());
        assert!(def.limits.is_some());
        assert!(def.is_deprecated());
        assert_eq!(
            def.deprecation_warning,
            Some("Use claude-4-opus instead".to_string())
        );
    }

    #[test]
    fn test_model_definition_with_enabled() {
        let id = ModelId::new("openai", "gpt-3.5-turbo");
        let caps = ModelCapabilities::new(16385, 4096);
        let def = ModelDefinition::new(id, "GPT-3.5 Turbo", caps).with_enabled(false);

        assert!(!def.enabled);
    }

    #[test]
    fn test_model_definition_serialization() {
        let id = ModelId::new("google", "gemini-pro");
        let caps = ModelCapabilities::new(32768, 8192);
        let def = ModelDefinition::new(id, "Gemini Pro", caps);

        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("\"display_name\":\"Gemini Pro\""));
        assert!(json.contains("\"context_window\":32768"));

        let deserialized: ModelDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.display_name, "Gemini Pro");
        assert_eq!(deserialized.capabilities.context_window, 32768);
    }

    #[test]
    fn test_model_definition_clone() {
        let id = ModelId::new("openai", "gpt-4");
        let caps = ModelCapabilities::new(8192, 4096);
        let def1 = ModelDefinition::new(id, "GPT-4", caps);

        let def2 = def1.clone();
        assert_eq!(def1.id.as_str(), def2.id.as_str());
        assert_eq!(def1.display_name, def2.display_name);
    }

    #[test]
    fn test_cost_serialization() {
        let cost = Cost::with_cache(0.03, 0.06, 0.01, 0.02);
        let json = serde_json::to_string(&cost).unwrap();

        assert!(json.contains("\"input\":0.03"));
        assert!(json.contains("\"output\":0.06"));
        assert!(json.contains("\"cache_read\":0.01"));
        assert!(json.contains("\"cache_write\":0.02"));

        let deserialized: Cost = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input, 0.03);
        assert_eq!(deserialized.cache_read, Some(0.01));
    }

    #[test]
    fn test_capabilities_with_all_features() {
        let caps = ModelCapabilities {
            context_window: 128000,
            max_output_tokens: 4096,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
            supports_function_call: true,
            supported_media_types: Some(vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/webp".to_string(),
            ]),
        };

        assert!(caps.supports_streaming);
        assert!(caps.supports_tools);
        assert!(caps.supports_vision);
        assert!(caps.supports_function_call);
        assert!(caps.supports_media_type("image/jpeg"));
        assert!(caps.supports_media_type("image/webp"));
        assert!(!caps.supports_media_type("video/mp4"));
    }
}
