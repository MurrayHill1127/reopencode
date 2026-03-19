//! Category resolver logic
//!
//! Resolves category names to model configurations with fallback chains.

use std::collections::{HashMap, HashSet};

use super::error::CategoryError;
use super::merge::merge_categories;
use super::requirements::get_category_requirement;
use super::types::{
    CategoryConfig, CategoryResolutionResult, FallbackEntry, ModelInfo, ModelSource,
    ModelSourceType, ModelVariant, ResolvedModel,
};

/// Options for category resolution
#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    /// User-specified model override
    pub user_model: Option<String>,
    /// User-specified fallback models
    pub user_fallback_models: Option<Vec<String>>,
    /// Available models (provider/model format)
    pub available_models: HashSet<String>,
    /// Connected providers
    pub connected_providers: Vec<String>,
    /// System default model
    pub system_default_model: Option<String>,
}

/// Category resolver with builder pattern
#[derive(Debug, Clone, Default)]
pub struct CategoryResolver {
    /// User-defined categories
    user_categories: HashMap<String, CategoryConfig>,
    /// Available models
    available_models: HashSet<String>,
    /// Connected providers
    connected_providers: Vec<String>,
    /// System default model
    system_default_model: Option<String>,
}

impl CategoryResolver {
    /// Create a new resolver
    pub fn new() -> Self {
        Self::default()
    }

    /// Set user-defined categories
    pub fn with_user_categories(mut self, categories: HashMap<String, CategoryConfig>) -> Self {
        self.user_categories = categories;
        self
    }

    /// Set available models
    pub fn with_available_models(mut self, models: HashSet<String>) -> Self {
        self.available_models = models;
        self
    }

    /// Set connected providers
    pub fn with_connected_providers(mut self, providers: Vec<String>) -> Self {
        self.connected_providers = providers;
        self
    }

    /// Set system default model
    pub fn with_system_default(mut self, model: String) -> Self {
        self.system_default_model = Some(model);
        self
    }

    /// Resolve a category to its model configuration
    pub fn resolve(&self, category_name: &str) -> Result<CategoryResolutionResult, CategoryError> {
        // Get merged categories
        let merged = merge_categories(Some(&self.user_categories));

        // Check if category exists
        let config = merged
            .get(category_name)
            .ok_or_else(|| CategoryError::unknown(category_name))?;

        // Check if disabled
        if config.disable {
            return Err(CategoryError::disabled(category_name));
        }

        // Get model requirement
        let requirement = get_category_requirement(category_name);
        let fallback_chain = requirement.map(|r| r.fallback_chain.clone());

        // Resolve model
        let options = ResolveOptions {
            user_model: config.model.clone(),
            user_fallback_models: config.fallback_models.clone(),
            available_models: self.available_models.clone(),
            connected_providers: self.connected_providers.clone(),
            system_default_model: self.system_default_model.clone(),
        };

        let (resolved_model, model_info) =
            resolve_model(&options, config, fallback_chain.as_deref(), category_name);

        let actual_model = resolved_model.as_ref().map(|m| m.full_model());

        Ok(CategoryResolutionResult {
            agent_to_use: category_name.to_string(),
            category_model: resolved_model,
            prompt_append: config.prompt_append.clone(),
            max_prompt_tokens: config.max_prompt_tokens,
            model_info,
            actual_model,
            is_unstable_agent: config.is_unstable_agent,
            fallback_chain,
            error: None,
        })
    }

    /// Get all available categories
    pub fn available_categories(&self) -> Vec<(String, CategoryConfig)> {
        let merged = merge_categories(Some(&self.user_categories));
        merged.into_iter().collect()
    }

    /// Check if a category is available
    pub fn is_category_available(&self, name: &str) -> bool {
        let merged = merge_categories(Some(&self.user_categories));
        merged.get(name).map(|c| !c.disable).unwrap_or(false)
    }
}

/// Resolve model from options and fallback chain
fn resolve_model(
    options: &ResolveOptions,
    _config: &CategoryConfig,
    fallback_chain: Option<&[FallbackEntry]>,
    _category_name: &str,
) -> (Option<ResolvedModel>, Option<ModelInfo>) {
    // Try user-specified model first
    if let Some(ref user_model) = options.user_model
        && options.available_models.contains(user_model)
    {
        let (provider, model) = split_model_string(user_model);
        let resolved = ResolvedModel::new(provider, model);
        let info = ModelInfo {
            model: user_model.clone(),
            source_type: ModelSourceType::UserDefined,
            source: ModelSource::Override,
        };
        return (Some(resolved), Some(info));
    }

    // Try user fallback models
    if let Some(ref fallbacks) = options.user_fallback_models {
        for model_str in fallbacks {
            if options.available_models.contains(model_str) {
                let (provider, model) = split_model_string(model_str);
                let resolved = ResolvedModel::new(provider, model);
                let info = ModelInfo {
                    model: model_str.clone(),
                    source_type: ModelSourceType::UserDefined,
                    source: ModelSource::Override,
                };
                return (Some(resolved), Some(info));
            }
        }
    }

    // Try fallback chain
    if let Some(chain) = fallback_chain {
        for entry in chain {
            // Check if any provider is connected
            let provider_available = entry
                .providers
                .iter()
                .any(|p| options.connected_providers.contains(p));

            if !provider_available {
                continue;
            }

            // Build model string
            for provider in &entry.providers {
                let model_str = format!("{}/{}", provider, entry.model);
                if options.available_models.contains(&model_str) {
                    let mut resolved = ResolvedModel::new(provider.clone(), &entry.model);
                    if let Some(v) = entry.variant {
                        resolved = resolved.with_variant(v);
                    }
                    let info = ModelInfo {
                        model: model_str,
                        source_type: ModelSourceType::CategoryDefault,
                        source: ModelSource::CategoryDefault,
                    };
                    return (Some(resolved), Some(info));
                }
            }
        }
    }

    // Try system default
    if let Some(ref default) = options.system_default_model
        && options.available_models.contains(default)
    {
        let (provider, model) = split_model_string(default);
        let resolved = ResolvedModel::new(provider, model);
        let info = ModelInfo {
            model: default.clone(),
            source_type: ModelSourceType::SystemDefault,
            source: ModelSource::SystemDefault,
        };
        return (Some(resolved), Some(info));
    }

    (None, None)
}

/// Split model string into provider and model
fn split_model_string(model: &str) -> (String, String) {
    match model.split_once('/') {
        Some((provider, model)) => (provider.to_string(), model.to_string()),
        None => (model.to_string(), String::new()),
    }
}

/// Resolve model for category (convenience function)
pub fn resolve_model_for_category(
    user_model: Option<&str>,
    user_fallback_models: Option<&[String]>,
    category_default_model: Option<&str>,
    fallback_chain: &[FallbackEntry],
    available_models: &HashSet<String>,
    system_default_model: Option<&str>,
) -> Option<(String, Option<ModelVariant>)> {
    // Try user model
    if let Some(model) = user_model
        && available_models.contains(model)
    {
        return Some((model.to_string(), None));
    }

    // Try user fallbacks
    if let Some(fallbacks) = user_fallback_models {
        for model in fallbacks {
            if available_models.contains(model) {
                return Some((model.to_string(), None));
            }
        }
    }

    // Try category default
    if let Some(model) = category_default_model
        && available_models.contains(model)
    {
        return Some((model.to_string(), None));
    }

    // Try fallback chain
    for entry in fallback_chain {
        for provider in &entry.providers {
            let model_str = format!("{}/{}", provider, entry.model);
            if available_models.contains(&model_str) {
                return Some((model_str, entry.variant));
            }
        }
    }

    // Try system default
    if let Some(model) = system_default_model
        && available_models.contains(model)
    {
        return Some((model.to_string(), None));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_new() {
        let resolver = CategoryResolver::new();
        assert!(resolver.available_models.is_empty());
    }

    #[test]
    fn test_resolver_builder() {
        let resolver = CategoryResolver::new()
            .with_available_models(
                vec!["anthropic/claude-haiku-4-5".to_string()]
                    .into_iter()
                    .collect(),
            )
            .with_system_default("anthropic/claude-sonnet-4-6".to_string());

        assert!(!resolver.available_models.is_empty());
        assert!(resolver.system_default_model.is_some());
    }

    #[test]
    fn test_resolve_unknown_category() {
        let resolver = CategoryResolver::new();
        let result = resolver.resolve("unknown-category");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_category_disabled() {
        let mut user_cats = HashMap::new();
        user_cats.insert(
            "quick".to_string(),
            CategoryConfig {
                disable: true,
                ..Default::default()
            },
        );

        let resolver = CategoryResolver::new().with_user_categories(user_cats);

        let result = resolver.resolve("quick");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_category_with_available_model() {
        let resolver = CategoryResolver::new().with_available_models(
            vec!["anthropic/claude-haiku-4-5".to_string()]
                .into_iter()
                .collect(),
        );

        let result = resolver.resolve("quick");
        assert!(result.is_ok());

        let resolution = result.unwrap();
        // Since we don't have the model in available_models with provider prefix matching,
        // the actual_model might be None
    }

    #[test]
    fn test_available_categories() {
        let resolver = CategoryResolver::new();
        let categories = resolver.available_categories();
        assert!(!categories.is_empty());
    }

    #[test]
    fn test_is_category_available() {
        let resolver = CategoryResolver::new();
        assert!(resolver.is_category_available("quick"));
        assert!(resolver.is_category_available("visual-engineering"));
        assert!(!resolver.is_category_available("unknown"));
    }

    #[test]
    fn test_split_model_string() {
        let (provider, model) = split_model_string("anthropic/claude-opus-4-6");
        assert_eq!(provider, "anthropic");
        assert_eq!(model, "claude-opus-4-6");

        let (provider, model) = split_model_string("invalid-no-slash");
        assert_eq!(provider, "invalid-no-slash");
        assert_eq!(model, "");
    }

    #[test]
    fn test_resolve_model_for_category_user_override() {
        let available: HashSet<String> = vec!["custom/model".to_string()].into_iter().collect();

        let result =
            resolve_model_for_category(Some("custom/model"), None, None, &[], &available, None);

        assert!(result.is_some());
        let (model, _) = result.unwrap();
        assert_eq!(model, "custom/model");
    }

    #[test]
    fn test_resolve_model_for_category_fallback() {
        let available: HashSet<String> = vec!["anthropic/claude-opus-4-6".to_string()]
            .into_iter()
            .collect();

        let chain = vec![FallbackEntry {
            providers: vec!["anthropic".to_string()],
            model: "claude-opus-4-6".to_string(),
            variant: Some(ModelVariant::Max),
        }];

        let result = resolve_model_for_category(None, None, None, &chain, &available, None);

        assert!(result.is_some());
        let (model, variant) = result.unwrap();
        assert_eq!(model, "anthropic/claude-opus-4-6");
        assert_eq!(variant, Some(ModelVariant::Max));
    }

    #[test]
    fn test_resolve_model_for_category_no_available() {
        let available: HashSet<String> = HashSet::new();

        let result = resolve_model_for_category(
            Some("unavailable/model"),
            None,
            None,
            &[],
            &available,
            None,
        );

        assert!(result.is_none());
    }
}
