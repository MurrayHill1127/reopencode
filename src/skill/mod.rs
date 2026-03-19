//! Skill module for ROC (reopencode)
//!
//! Provides a skill system for loading and managing specialized instructions.
//!
//! Features:
//! - Multi-source skill discovery (project, user, global, remote)
//! - YAML frontmatter parsing for SKILL.md files
//! - Priority-based skill override
//! - Skill registry with async-safe access
//!
//! # Example
//!
//! ```rust,no_run
//! use reopencode::skill::{SkillRegistry, discover_skills, DiscoveryOptions};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Discover skills from all sources
//!     let result = discover_skills(None).await.unwrap();
//!     
//!     // Use the global registry
//!     let registry = SkillRegistry::global();
//!     registry.register_all(result.skills).await;
//!     
//!     // Get a skill by name
//!     if let Some(skill) = registry.get("my-skill").await {
//!         println!("Found skill: {}", skill.name);
//!     }
//! }
//! ```

pub mod discovery;
pub mod error;
pub mod loader;
pub mod parser;
pub mod registry;
pub mod types;

// Re-export error types
pub use error::{ParseError, SkillError};

// Re-export types
pub use types::{
    DiscoveryOptions, DiscoveryResult, SkillConfig, SkillInfo, SkillMetadata, SkillScope,
};

// Re-export parser functions
pub use parser::{parse_skill_content, parse_skill_file};

// Re-export discovery functions
pub use discovery::{discover_skills, discover_skills_with_options};

// Re-export registry
pub use registry::SkillRegistry;

// Re-export loader
pub use loader::{SkillLoader, pull_skills_from_url};

/// Get a skill by name from the global registry
pub async fn get(name: &str) -> Option<SkillInfo> {
    SkillRegistry::global().get(name).await
}

/// Get all skills from the global registry
pub async fn all() -> Vec<SkillInfo> {
    SkillRegistry::global().all().await
}

/// Get all skill directories from the global registry
pub async fn directories() -> Vec<std::path::PathBuf> {
    SkillRegistry::global().directories().await
}

/// Format skills list for display
pub fn format_skills(skills: &[SkillInfo], verbose: bool) -> String {
    if skills.is_empty() {
        return "No skills are currently available.".to_string();
    }

    if verbose {
        let items: Vec<String> = skills
            .iter()
            .flat_map(|s| {
                vec![
                    "  <skill>".to_string(),
                    format!("    <name>{}</name>", s.name),
                    format!("    <description>{}</description>", s.description),
                    format!("    <location>{}</location>", s.location.display()),
                    "  </skill>".to_string(),
                ]
            })
            .collect();

        format!(
            "<available_skills>\n{}\n</available_skills>",
            items.join("\n")
        )
    } else {
        let items: Vec<String> = skills
            .iter()
            .map(|s| format!("- **{}**: {}", s.name, s.description))
            .collect();

        format!("## Available Skills\n{}", items.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_skills_empty() {
        let output = format_skills(&[], false);
        assert_eq!(output, "No skills are currently available.");
    }

    #[test]
    fn test_format_skills_verbose() {
        let skills = vec![SkillInfo::new(
            "test-skill".to_string(),
            "A test skill".to_string(),
            SkillScope::Project,
        )];

        let output = format_skills(&skills, true);
        assert!(output.contains("<available_skills>"));
        assert!(output.contains("<name>test-skill</name>"));
        assert!(output.contains("<description>A test skill</description>"));
    }

    #[test]
    fn test_format_skills_simple() {
        let skills = vec![SkillInfo::new(
            "my-skill".to_string(),
            "My description".to_string(),
            SkillScope::User,
        )];

        let output = format_skills(&skills, false);
        assert!(output.contains("## Available Skills"));
        assert!(output.contains("- **my-skill**: My description"));
    }

    #[tokio::test]
    async fn test_global_registry() {
        let registry = SkillRegistry::global();

        // Clear first to ensure clean state
        registry.clear().await;

        let skill = SkillInfo::new(
            "global-test".to_string(),
            "Test".to_string(),
            SkillScope::Project,
        );
        registry.register(skill).await.unwrap();

        let retrieved = get("global-test").await;
        assert!(retrieved.is_some());

        // Clean up
        registry.clear().await;
    }
}
