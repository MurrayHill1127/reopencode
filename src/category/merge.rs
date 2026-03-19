//! Category merge logic
//!
//! Merges user-defined categories with defaults, filtering disabled ones.

use std::collections::HashMap;

use super::defaults::DEFAULT_CATEGORIES;
use super::types::CategoryConfig;

/// Merge user categories with defaults
///
/// User categories override defaults. Disabled categories are filtered out.
pub fn merge_categories(
    user_categories: Option<&HashMap<String, CategoryConfig>>,
) -> HashMap<String, CategoryConfig> {
    let mut merged = match user_categories {
        Some(user) => {
            let mut m = DEFAULT_CATEGORIES.clone();
            for (key, value) in user {
                m.insert(key.clone(), value.clone());
            }
            m
        }
        None => DEFAULT_CATEGORIES.clone(),
    };

    // Filter out disabled categories
    merged.retain(|_, config| !config.disable);

    merged
}

/// Merge a single category config with defaults
pub fn merge_single_category(
    category_name: &str,
    user_config: Option<&CategoryConfig>,
) -> Option<CategoryConfig> {
    let default = DEFAULT_CATEGORIES.get(category_name).cloned();

    match (default, user_config) {
        (Some(mut base), Some(override_config)) => {
            base.merge(override_config);
            if base.disable { None } else { Some(base) }
        }
        (Some(base), None) => {
            if base.disable {
                None
            } else {
                Some(base)
            }
        }
        (None, Some(user)) => {
            if user.disable {
                None
            } else {
                Some(user.clone())
            }
        }
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_categories_no_user() {
        let merged = merge_categories(None);
        assert_eq!(merged.len(), 8);
        assert!(merged.contains_key("quick"));
        assert!(merged.contains_key("visual-engineering"));
    }

    #[test]
    fn test_merge_categories_with_override() {
        let mut user = HashMap::new();
        user.insert(
            "quick".to_string(),
            CategoryConfig {
                model: Some("custom/model".to_string()),
                ..Default::default()
            },
        );

        let merged = merge_categories(Some(&user));

        assert_eq!(
            merged.get("quick").unwrap().model,
            Some("custom/model".to_string())
        );
        assert!(merged.contains_key("visual-engineering"));
    }

    #[test]
    fn test_merge_categories_filter_disabled() {
        let mut user = HashMap::new();
        user.insert(
            "quick".to_string(),
            CategoryConfig {
                disable: true,
                ..Default::default()
            },
        );

        let merged = merge_categories(Some(&user));

        assert!(!merged.contains_key("quick"));
        assert!(merged.contains_key("visual-engineering"));
    }

    #[test]
    fn test_merge_categories_new_category() {
        let mut user = HashMap::new();
        user.insert(
            "custom-category".to_string(),
            CategoryConfig {
                model: Some("custom/model".to_string()),
                ..Default::default()
            },
        );

        let merged = merge_categories(Some(&user));

        assert!(merged.contains_key("custom-category"));
        assert_eq!(merged.len(), 9);
    }

    #[test]
    fn test_merge_single_category_override() {
        let user_config = CategoryConfig {
            model: Some("custom/model".to_string()),
            ..Default::default()
        };

        let result = merge_single_category("quick", Some(&user_config));

        assert!(result.is_some());
        assert_eq!(result.unwrap().model, Some("custom/model".to_string()));
    }

    #[test]
    fn test_merge_single_category_disabled() {
        let user_config = CategoryConfig {
            disable: true,
            ..Default::default()
        };

        let result = merge_single_category("quick", Some(&user_config));

        assert!(result.is_none());
    }

    #[test]
    fn test_merge_single_category_nonexistent() {
        let result = merge_single_category("nonexistent", None);
        assert!(result.is_none());
    }
}
