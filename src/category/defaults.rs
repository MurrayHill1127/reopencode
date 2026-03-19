//! Default category definitions
//!
//! Contains built-in category configurations with model mappings.

use lazy_static::lazy_static;
use std::collections::HashMap;
use std::str::FromStr;

use super::types::{BuiltinCategoryName, CategoryConfig, ModelVariant};

lazy_static! {
    /// Default category configurations
    pub static ref DEFAULT_CATEGORIES: HashMap<String, CategoryConfig> = {
        let mut m = HashMap::new();

        m.insert("visual-engineering".to_string(), CategoryConfig {
            model: Some("google/gemini-3.1-pro".to_string()),
            variant: Some(ModelVariant::High),
            description: Some("Frontend, UI/UX, design, styling, animation".to_string()),
            ..Default::default()
        });

        m.insert("ultrabrain".to_string(), CategoryConfig {
            model: Some("openai/gpt-5.4".to_string()),
            variant: Some(ModelVariant::Xhigh),
            description: Some("Hard logic tasks, architecture decisions".to_string()),
            ..Default::default()
        });

        m.insert("deep".to_string(), CategoryConfig {
            model: Some("openai/gpt-5.3-codex".to_string()),
            variant: Some(ModelVariant::Medium),
            description: Some("Autonomous problem-solving with deep understanding".to_string()),
            ..Default::default()
        });

        m.insert("artistry".to_string(), CategoryConfig {
            model: Some("google/gemini-3.1-pro".to_string()),
            variant: Some(ModelVariant::High),
            description: Some("Creative approaches, unconventional solutions".to_string()),
            ..Default::default()
        });

        m.insert("quick".to_string(), CategoryConfig {
            model: Some("anthropic/claude-haiku-4-5".to_string()),
            description: Some("Trivial tasks - single file changes, typo fixes".to_string()),
            ..Default::default()
        });

        m.insert("unspecified-low".to_string(), CategoryConfig {
            model: Some("anthropic/claude-sonnet-4-6".to_string()),
            description: Some("General tasks with low complexity".to_string()),
            ..Default::default()
        });

        m.insert("unspecified-high".to_string(), CategoryConfig {
            model: Some("anthropic/claude-opus-4-6".to_string()),
            variant: Some(ModelVariant::Max),
            description: Some("General tasks with high complexity".to_string()),
            ..Default::default()
        });

        m.insert("writing".to_string(), CategoryConfig {
            model: Some("kimi-for-coding/k2p5".to_string()),
            description: Some("Documentation, prose, technical writing".to_string()),
            ..Default::default()
        });

        m
    };

    /// Category descriptions
    pub static ref CATEGORY_DESCRIPTIONS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("visual-engineering", "Frontend, UI/UX, design, styling, animation");
        m.insert("ultrabrain", "Use ONLY for genuinely hard, logic-heavy tasks. Give clear goals only, not step-by-step instructions.");
        m.insert("deep", "Goal-oriented autonomous problem-solving. Thorough research before action. For hairy problems requiring deep understanding.");
        m.insert("artistry", "Complex problem-solving with unconventional, creative approaches - beyond standard patterns");
        m.insert("quick", "Trivial tasks - single file changes, typo fixes, simple modifications");
        m.insert("unspecified-low", "Tasks that don't fit other categories, low effort required");
        m.insert("unspecified-high", "Tasks that don't fit other categories, high effort required");
        m.insert("writing", "Documentation, prose, technical writing");
        m
    };
}

/// Get default model for a category
pub fn get_default_model(category: &str) -> Option<&'static str> {
    DEFAULT_CATEGORIES
        .get(category)
        .and_then(|c| c.model.as_deref())
}

/// Get default variant for a category
pub fn get_default_variant(category: &str) -> Option<ModelVariant> {
    DEFAULT_CATEGORIES.get(category).and_then(|c| c.variant)
}

/// Check if a category is a built-in category
pub fn is_builtin_category(name: &str) -> bool {
    BuiltinCategoryName::from_str(name).is_ok()
}

/// Get all built-in category names
pub fn builtin_category_names() -> Vec<&'static str> {
    vec![
        "visual-engineering",
        "ultrabrain",
        "deep",
        "artistry",
        "quick",
        "unspecified-low",
        "unspecified-high",
        "writing",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_categories_exist() {
        assert!(DEFAULT_CATEGORIES.contains_key("visual-engineering"));
        assert!(DEFAULT_CATEGORIES.contains_key("ultrabrain"));
        assert!(DEFAULT_CATEGORIES.contains_key("deep"));
        assert!(DEFAULT_CATEGORIES.contains_key("artistry"));
        assert!(DEFAULT_CATEGORIES.contains_key("quick"));
        assert!(DEFAULT_CATEGORIES.contains_key("unspecified-low"));
        assert!(DEFAULT_CATEGORIES.contains_key("unspecified-high"));
        assert!(DEFAULT_CATEGORIES.contains_key("writing"));
    }

    #[test]
    fn test_default_categories_count() {
        assert_eq!(DEFAULT_CATEGORIES.len(), 8);
    }

    #[test]
    fn test_category_descriptions() {
        assert!(CATEGORY_DESCRIPTIONS.contains_key("visual-engineering"));
        assert!(CATEGORY_DESCRIPTIONS.contains_key("quick"));
    }

    #[test]
    fn test_get_default_model() {
        assert_eq!(
            get_default_model("quick"),
            Some("anthropic/claude-haiku-4-5")
        );
        assert_eq!(get_default_model("ultrabrain"), Some("openai/gpt-5.4"));
        assert_eq!(get_default_model("invalid"), None);
    }

    #[test]
    fn test_get_default_variant() {
        assert_eq!(
            get_default_variant("visual-engineering"),
            Some(ModelVariant::High)
        );
        assert_eq!(get_default_variant("ultrabrain"), Some(ModelVariant::Xhigh));
        assert_eq!(get_default_variant("quick"), None);
    }

    #[test]
    fn test_is_builtin_category() {
        assert!(is_builtin_category("quick"));
        assert!(is_builtin_category("visual-engineering"));
        assert!(!is_builtin_category("custom-category"));
    }

    #[test]
    fn test_builtin_category_names() {
        let names = builtin_category_names();
        assert_eq!(names.len(), 8);
        assert!(names.contains(&"quick"));
        assert!(names.contains(&"visual-engineering"));
    }
}
