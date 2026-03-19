//! Core data structures for category module
//!
//! Defines all types used for task classification and model selection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Built-in category names for task classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinCategoryName {
    /// Frontend, UI/UX, design, styling, animation
    VisualEngineering,
    /// Hard logic tasks, architecture decisions
    Ultrabrain,
    /// Autonomous problem-solving with deep understanding
    Deep,
    /// Creative approaches, unconventional solutions
    Artistry,
    /// Trivial tasks - single file changes, typo fixes
    Quick,
    /// General tasks with low complexity
    UnspecifiedLow,
    /// General tasks with high complexity
    UnspecifiedHigh,
    /// Documentation, prose, technical writing
    Writing,
}

impl BuiltinCategoryName {
    /// Get the kebab-case string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VisualEngineering => "visual-engineering",
            Self::Ultrabrain => "ultrabrain",
            Self::Deep => "deep",
            Self::Artistry => "artistry",
            Self::Quick => "quick",
            Self::UnspecifiedLow => "unspecified-low",
            Self::UnspecifiedHigh => "unspecified-high",
            Self::Writing => "writing",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::VisualEngineering => "Frontend, UI/UX, design, styling, animation",
            Self::Ultrabrain => "Hard logic tasks, architecture decisions",
            Self::Deep => "Autonomous problem-solving with deep understanding",
            Self::Artistry => "Creative approaches, unconventional solutions",
            Self::Quick => "Trivial tasks - single file changes, typo fixes",
            Self::UnspecifiedLow => "General tasks with low complexity",
            Self::UnspecifiedHigh => "General tasks with high complexity",
            Self::Writing => "Documentation, prose, technical writing",
        }
    }

    /// Get default model for this category
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::VisualEngineering => "google/gemini-3.1-pro",
            Self::Ultrabrain => "openai/gpt-5.4",
            Self::Deep => "openai/gpt-5.3-codex",
            Self::Artistry => "google/gemini-3.1-pro",
            Self::Quick => "anthropic/claude-haiku-4-5",
            Self::UnspecifiedLow => "anthropic/claude-sonnet-4-6",
            Self::UnspecifiedHigh => "anthropic/claude-opus-4-6",
            Self::Writing => "kimi-for-coding/k2p5",
        }
    }

    /// Get default variant for this category
    pub fn default_variant(&self) -> Option<ModelVariant> {
        match self {
            Self::VisualEngineering => Some(ModelVariant::High),
            Self::Ultrabrain => Some(ModelVariant::Xhigh),
            Self::Deep => Some(ModelVariant::Medium),
            Self::Artistry => Some(ModelVariant::High),
            Self::Quick => None,
            Self::UnspecifiedLow => None,
            Self::UnspecifiedHigh => Some(ModelVariant::Max),
            Self::Writing => None,
        }
    }

    /// Iterate over all built-in categories
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Self::VisualEngineering,
            Self::Ultrabrain,
            Self::Deep,
            Self::Artistry,
            Self::Quick,
            Self::UnspecifiedLow,
            Self::UnspecifiedHigh,
            Self::Writing,
        ]
        .into_iter()
    }
}

impl fmt::Display for BuiltinCategoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BuiltinCategoryName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "visual-engineering" => Ok(Self::VisualEngineering),
            "ultrabrain" => Ok(Self::Ultrabrain),
            "deep" => Ok(Self::Deep),
            "artistry" => Ok(Self::Artistry),
            "quick" => Ok(Self::Quick),
            "unspecified-low" => Ok(Self::UnspecifiedLow),
            "unspecified-high" => Ok(Self::UnspecifiedHigh),
            "writing" => Ok(Self::Writing),
            _ => Err(format!("Unknown category: {}", s)),
        }
    }
}

/// Category name (supports both built-in and custom)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CategoryName {
    /// Built-in category
    BuiltIn(BuiltinCategoryName),
    /// Custom user-defined category
    Custom(String),
}

impl CategoryName {
    /// Create a built-in category name
    pub fn builtin(name: BuiltinCategoryName) -> Self {
        Self::BuiltIn(name)
    }

    /// Create a custom category name
    pub fn custom(name: impl Into<String>) -> Self {
        Self::Custom(name.into())
    }

    /// Check if this is a built-in category
    pub fn is_builtin(&self) -> bool {
        matches!(self, Self::BuiltIn(_))
    }

    /// Get the string representation
    pub fn as_str(&self) -> &str {
        match self {
            Self::BuiltIn(name) => name.as_str(),
            Self::Custom(name) => name.as_str(),
        }
    }
}

impl fmt::Display for CategoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuiltIn(name) => write!(f, "{}", name),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl FromStr for CategoryName {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match BuiltinCategoryName::from_str(s) {
            Ok(name) => Ok(Self::BuiltIn(name)),
            Err(_) => Ok(Self::Custom(s.to_string())),
        }
    }
}

impl From<BuiltinCategoryName> for CategoryName {
    fn from(name: BuiltinCategoryName) -> Self {
        Self::BuiltIn(name)
    }
}

/// Model variant enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelVariant {
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
    Max,
}

impl ModelVariant {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

impl fmt::Display for ModelVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Thinking type enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingType {
    Enabled,
    #[default]
    Disabled,
}

/// Thinking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ThinkingConfig {
    /// Enable/disable thinking mode
    pub r#type: ThinkingType,
    /// Thinking budget tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

/// Reasoning effort enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    #[default]
    Medium,
    High,
    Xhigh,
}

/// Text verbosity enum
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextVerbosity {
    Low,
    #[default]
    Medium,
    High,
}

/// Category configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CategoryConfig {
    /// Category description (shown in task prompt)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Model identifier (format: "provider/model")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Fallback model list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_models: Option<Vec<String>>,

    /// Model variant (high, medium, low, xhigh, max)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ModelVariant>,

    /// Temperature parameter (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-p sampling (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Max tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Thinking configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,

    /// Reasoning effort
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,

    /// Text verbosity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_verbosity: Option<TextVerbosity>,

    /// Tool enable/disable config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashMap<String, bool>>,

    /// Prompt append text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_append: Option<String>,

    /// Max prompt tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_prompt_tokens: Option<u32>,

    /// Mark as unstable agent (force background monitoring)
    #[serde(default)]
    pub is_unstable_agent: bool,

    /// Disable this category
    #[serde(default)]
    pub disable: bool,
}

impl CategoryConfig {
    /// Create a new category config with a model
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: Some(model.into()),
            ..Default::default()
        }
    }

    /// Create a builder-style config with variant
    pub fn with_variant(mut self, variant: ModelVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Create a builder-style config with description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Merge another config into this one (other takes precedence)
    pub fn merge(&mut self, other: &Self) {
        if other.description.is_some() {
            self.description = other.description.clone();
        }
        if other.model.is_some() {
            self.model = other.model.clone();
        }
        if other.fallback_models.is_some() {
            self.fallback_models = other.fallback_models.clone();
        }
        if other.variant.is_some() {
            self.variant = other.variant;
        }
        if other.temperature.is_some() {
            self.temperature = other.temperature;
        }
        if other.top_p.is_some() {
            self.top_p = other.top_p;
        }
        if other.max_tokens.is_some() {
            self.max_tokens = other.max_tokens;
        }
        if other.thinking.is_some() {
            self.thinking = other.thinking.clone();
        }
        if other.reasoning_effort.is_some() {
            self.reasoning_effort = other.reasoning_effort;
        }
        if other.text_verbosity.is_some() {
            self.text_verbosity = other.text_verbosity;
        }
        if other.tools.is_some() {
            self.tools = other.tools.clone();
        }
        if other.prompt_append.is_some() {
            self.prompt_append = other.prompt_append.clone();
        }
        if other.max_prompt_tokens.is_some() {
            self.max_prompt_tokens = other.max_prompt_tokens;
        }
        if other.is_unstable_agent {
            self.is_unstable_agent = true;
        }
        if other.disable {
            self.disable = true;
        }
    }
}

/// Fallback chain entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackEntry {
    /// Provider list (any one available is sufficient)
    pub providers: Vec<String>,
    /// Model identifier
    pub model: String,
    /// Entry-specific variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ModelVariant>,
}

impl FallbackEntry {
    /// Create a new fallback entry
    pub fn new(providers: Vec<String>, model: impl Into<String>) -> Self {
        Self {
            providers,
            model: model.into(),
            variant: None,
        }
    }

    /// Add variant to the entry
    pub fn with_variant(mut self, variant: ModelVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

/// Model requirement specification
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelRequirement {
    /// Fallback chain
    #[serde(default)]
    pub fallback_chain: Vec<FallbackEntry>,
    /// Default variant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<ModelVariant>,
    /// Required model (fuzzy match)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_model: Option<String>,
    /// Require any model available
    #[serde(default)]
    pub requires_any_model: bool,
    /// Required providers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_provider: Option<Vec<String>>,
}

impl ModelRequirement {
    /// Create a new model requirement with a fallback chain
    pub fn with_fallback_chain(chain: Vec<FallbackEntry>) -> Self {
        Self {
            fallback_chain: chain,
            ..Default::default()
        }
    }
}

/// Model source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelSourceType {
    UserDefined,
    Inherited,
    CategoryDefault,
    SystemDefault,
}

/// Model source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelSource {
    Override,
    CategoryDefault,
    SystemDefault,
}

/// Resolved model information
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    /// Provider ID
    pub provider_id: String,
    /// Model ID
    pub model_id: String,
    /// Variant
    pub variant: Option<ModelVariant>,
}

impl ResolvedModel {
    /// Create a new resolved model
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider_id: provider.into(),
            model_id: model.into(),
            variant: None,
        }
    }

    /// Add variant
    pub fn with_variant(mut self, variant: ModelVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Get full model string (provider/model)
    pub fn full_model(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}

/// Model info for resolution result
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model string
    pub model: String,
    /// Source type
    pub source_type: ModelSourceType,
    /// Source
    pub source: ModelSource,
}

/// Category resolution result
#[derive(Debug, Clone, Default)]
pub struct CategoryResolutionResult {
    /// Agent name to use
    pub agent_to_use: String,
    /// Resolved model
    pub category_model: Option<ResolvedModel>,
    /// Prompt append text
    pub prompt_append: Option<String>,
    /// Max prompt tokens
    pub max_prompt_tokens: Option<u32>,
    /// Model info
    pub model_info: Option<ModelInfo>,
    /// Actual model string
    pub actual_model: Option<String>,
    /// Is unstable agent
    pub is_unstable_agent: bool,
    /// Fallback chain
    pub fallback_chain: Option<Vec<FallbackEntry>>,
    /// Error message
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_category_name_str() {
        assert_eq!(
            BuiltinCategoryName::VisualEngineering.as_str(),
            "visual-engineering"
        );
        assert_eq!(BuiltinCategoryName::Ultrabrain.as_str(), "ultrabrain");
        assert_eq!(BuiltinCategoryName::Quick.as_str(), "quick");
    }

    #[test]
    fn test_builtin_category_name_from_str() {
        assert!("visual-engineering".parse::<BuiltinCategoryName>().is_ok());
        assert!("quick".parse::<BuiltinCategoryName>().is_ok());
        assert!("invalid".parse::<BuiltinCategoryName>().is_err());
    }

    #[test]
    fn test_builtin_category_name_iter() {
        let categories: Vec<_> = BuiltinCategoryName::iter().collect();
        assert_eq!(categories.len(), 8);
    }

    #[test]
    fn test_category_name_custom() {
        let name = CategoryName::custom("my-custom-category");
        assert!(!name.is_builtin());
        assert_eq!(name.as_str(), "my-custom-category");
    }

    #[test]
    fn test_model_variant() {
        assert_eq!(ModelVariant::default(), ModelVariant::Medium);
        assert_eq!(ModelVariant::High.as_str(), "high");
    }

    #[test]
    fn test_category_config_merge() {
        let mut base = CategoryConfig::with_model("provider/model-a");
        let override_config = CategoryConfig {
            model: Some("provider/model-b".to_string()),
            variant: Some(ModelVariant::High),
            ..Default::default()
        };
        base.merge(&override_config);
        assert_eq!(base.model, Some("provider/model-b".to_string()));
        assert_eq!(base.variant, Some(ModelVariant::High));
    }

    #[test]
    fn test_fallback_entry() {
        let entry = FallbackEntry::new(
            vec!["google".to_string(), "opencode".to_string()],
            "gemini-3.1-pro",
        )
        .with_variant(ModelVariant::High);

        assert_eq!(entry.providers.len(), 2);
        assert_eq!(entry.model, "gemini-3.1-pro");
        assert_eq!(entry.variant, Some(ModelVariant::High));
    }

    #[test]
    fn test_resolved_model() {
        let model =
            ResolvedModel::new("anthropic", "claude-opus-4-6").with_variant(ModelVariant::Max);
        assert_eq!(model.full_model(), "anthropic/claude-opus-4-6");
        assert_eq!(model.variant, Some(ModelVariant::Max));
    }

    #[test]
    fn test_serde_category_config() {
        let config = CategoryConfig {
            model: Some("anthropic/claude-sonnet-4-6".to_string()),
            variant: Some(ModelVariant::High),
            temperature: Some(0.7),
            disable: false,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: CategoryConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.model, config.model);
        assert_eq!(parsed.variant, config.variant);
        assert_eq!(parsed.temperature, config.temperature);
    }
}
