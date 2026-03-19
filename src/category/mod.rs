//! Category module for task classification and model selection
//!
//! Provides 8 built-in categories with model fallback chains:
//! - `visual-engineering` - Frontend, UI/UX, design
//! - `ultrabrain` - Hard logic, architecture
//! - `deep` - Autonomous problem-solving
//! - `artistry` - Creative approaches
//! - `quick` - Trivial tasks
//! - `unspecified-low` - Low complexity general tasks
//! - `unspecified-high` - High complexity general tasks
//! - `writing` - Documentation, prose

mod defaults;
mod error;
mod merge;
mod requirements;
mod resolver;
mod types;

// Re-export error types
pub use error::{CategoryError, ResolutionError};

// Re-export types
pub use types::{
    BuiltinCategoryName, CategoryConfig, CategoryName, CategoryResolutionResult, FallbackEntry,
    ModelInfo, ModelRequirement, ModelSource, ModelSourceType, ModelVariant, ReasoningEffort,
    ResolvedModel, TextVerbosity, ThinkingConfig, ThinkingType,
};

// Re-export defaults
pub use defaults::{
    CATEGORY_DESCRIPTIONS, DEFAULT_CATEGORIES, builtin_category_names, get_default_model,
    get_default_variant, is_builtin_category,
};

// Re-export requirements
pub use requirements::{
    AGENT_MODEL_REQUIREMENTS, CATEGORY_MODEL_REQUIREMENTS, get_agent_requirement,
    get_category_requirement,
};

// Re-export resolver
pub use resolver::{CategoryResolver, ResolveOptions, resolve_model_for_category};

// Re-export merge
pub use merge::{merge_categories, merge_single_category};

/// Get list of all built-in categories
pub fn builtin_categories() -> Vec<BuiltinCategoryName> {
    BuiltinCategoryName::iter().collect()
}

/// Check if a category exists (built-in or user-defined)
pub fn category_exists(
    name: &str,
    user_categories: Option<&std::collections::HashMap<String, CategoryConfig>>,
) -> bool {
    if is_builtin_category(name) {
        return true;
    }
    user_categories
        .map(|c| c.contains_key(name))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_categories_count() {
        let categories = builtin_categories();
        assert_eq!(categories.len(), 8);
    }

    #[test]
    fn test_category_exists_builtin() {
        assert!(category_exists("quick", None));
        assert!(category_exists("visual-engineering", None));
    }

    #[test]
    fn test_category_exists_user() {
        let mut user = std::collections::HashMap::new();
        user.insert("custom".to_string(), CategoryConfig::default());

        assert!(category_exists("custom", Some(&user)));
        assert!(!category_exists("other", Some(&user)));
    }

    #[test]
    fn test_all_exports_accessible() {
        // Verify all public types are accessible
        let _error = CategoryError::unknown("test");
        let _name = BuiltinCategoryName::Quick;
        let _config = CategoryConfig::default();
        let _variant = ModelVariant::High;
    }
}
