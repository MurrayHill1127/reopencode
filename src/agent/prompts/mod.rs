//! Prompt templates for builtin agents
//!
//! Templates are loaded at compile time using `include_str!` macro.

#![allow(dead_code)]

/// Get prompt template by name
///
/// # Arguments
/// * `name` - Template name (explore, compaction, summary, title)
///
/// # Returns
/// Template content if found, None otherwise
pub fn get_template(name: &str) -> Option<&'static str> {
    match name {
        "explore" => Some(EXPLORE_TEMPLATE),
        "compaction" => Some(COMPACTION_TEMPLATE),
        "summary" => Some(SUMMARY_TEMPLATE),
        "title" => Some(TITLE_TEMPLATE),
        _ => None,
    }
}

/// List all available template names
pub fn list_templates() -> &'static [&'static str] {
    &["explore", "compaction", "summary", "title"]
}

// Templates loaded at compile time
const EXPLORE_TEMPLATE: &str = include_str!("explore.txt");
const COMPACTION_TEMPLATE: &str = include_str!("compaction.txt");
const SUMMARY_TEMPLATE: &str = include_str!("summary.txt");
const TITLE_TEMPLATE: &str = include_str!("title.txt");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_not_empty() {
        assert!(!EXPLORE_TEMPLATE.is_empty());
        assert!(!COMPACTION_TEMPLATE.is_empty());
        assert!(!SUMMARY_TEMPLATE.is_empty());
        assert!(!TITLE_TEMPLATE.is_empty());
    }

    #[test]
    fn test_get_template_valid() {
        assert!(get_template("explore").is_some());
        assert!(get_template("compaction").is_some());
        assert!(get_template("summary").is_some());
        assert!(get_template("title").is_some());
    }

    #[test]
    fn test_get_template_invalid() {
        assert!(get_template("invalid").is_none());
        assert!(get_template("").is_none());
    }

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert_eq!(templates.len(), 4);
        assert!(templates.contains(&"explore"));
        assert!(templates.contains(&"compaction"));
        assert!(templates.contains(&"summary"));
        assert!(templates.contains(&"title"));
    }
}
