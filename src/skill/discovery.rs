//! Multi-source skill discovery with priority override
//!
//! Provides skill discovery from multiple sources with priority-based merging.
//! Higher priority scopes override lower priority ones.

use crate::skill::{DiscoveryOptions, DiscoveryResult, SkillError, SkillInfo, SkillScope};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Discover skills with custom options
///
/// Discovery paths (in priority order - lower scope value = higher priority):
/// 1. Project: `<directory>/.opencode/skills/**/SKILL.md`
/// 2. OpenCode global: `~/.config/opencode/skills/`
/// 3. User (if include_claude_paths):
///    - `<directory>/.claude/skills/`
///    - `<directory>/.agents/skills/`
///    - `~/.claude/skills/`
///    - `~/.agents/skills/`
pub async fn discover_skills_with_options(
    options: DiscoveryOptions,
) -> Result<DiscoveryResult, SkillError> {
    let mut skills: HashMap<String, SkillInfo> = HashMap::new();
    let mut directories: Vec<PathBuf> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // Get the base directory (use current dir if not specified)
    let base_dir = match options.directory {
        Some(ref dir) => dir.clone(),
        None => std::env::current_dir().map_err(|e| SkillError::IoError(PathBuf::from("."), e))?,
    };

    debug!(
        "Starting skill discovery from base directory: {:?}",
        base_dir
    );

    // 1. Project scope: <directory>/.opencode/skills/
    let project_path = base_dir.join(".opencode/skills");
    if project_path.exists() {
        debug!("Discovering project skills from: {:?}", project_path);
        let project_skills = discover_from_directory(&project_path, SkillScope::Project).await;
        merge_skills(&mut skills, project_skills, &mut warnings);
        directories.push(project_path);
    }

    // 2. OpenCode global: ~/.config/opencode/skills/
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("opencode/skills");
        if global_path.exists() {
            debug!("Discovering global skills from: {:?}", global_path);
            let global_skills = discover_from_directory(&global_path, SkillScope::Opencode).await;
            merge_skills(&mut skills, global_skills, &mut warnings);
            directories.push(global_path);
        }
    }

    // 3. User paths (if enabled)
    if options.include_claude_paths {
        // Project-local Claude paths
        let project_claude_skills = base_dir.join(".claude/skills");
        if project_claude_skills.exists() {
            debug!(
                "Discovering project Claude skills from: {:?}",
                project_claude_skills
            );
            let found = discover_from_directory(&project_claude_skills, SkillScope::User).await;
            merge_skills(&mut skills, found, &mut warnings);
            directories.push(project_claude_skills);
        }

        let project_agents_skills = base_dir.join(".agents/skills");
        if project_agents_skills.exists() {
            debug!(
                "Discovering project agents skills from: {:?}",
                project_agents_skills
            );
            let found = discover_from_directory(&project_agents_skills, SkillScope::User).await;
            merge_skills(&mut skills, found, &mut warnings);
            directories.push(project_agents_skills);
        }

        // User home Claude paths
        if let Some(home_dir) = dirs::home_dir() {
            let user_claude_skills = home_dir.join(".claude/skills");
            if user_claude_skills.exists() {
                debug!(
                    "Discovering user Claude skills from: {:?}",
                    user_claude_skills
                );
                let found = discover_from_directory(&user_claude_skills, SkillScope::User).await;
                merge_skills(&mut skills, found, &mut warnings);
                directories.push(user_claude_skills);
            }

            let user_agents_skills = home_dir.join(".agents/skills");
            if user_agents_skills.exists() {
                debug!(
                    "Discovering user agents skills from: {:?}",
                    user_agents_skills
                );
                let found = discover_from_directory(&user_agents_skills, SkillScope::User).await;
                merge_skills(&mut skills, found, &mut warnings);
                directories.push(user_agents_skills);
            }
        }
    }

    debug!("Skill discovery complete. Found {} skills", skills.len());

    Ok(DiscoveryResult {
        skills,
        directories,
        warnings,
    })
}

/// Discover skills from a directory
///
/// Uses walkdir to find all SKILL.md files up to max_depth=2
async fn discover_from_directory(dir: &Path, scope: SkillScope) -> Vec<SkillInfo> {
    if !dir.exists() {
        debug!("Directory does not exist: {:?}", dir);
        return Vec::new();
    }

    let mut skills = Vec::new();
    let max_depth = 2;

    // Use walkdir to traverse the directory
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(max_depth)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Look for SKILL.md files only
        if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new("SKILL.md")) {
            debug!("Found skill file: {:?}", path);

            match crate::skill::parser::parse_skill_file(path, scope).await {
                Ok(mut skill_info) => {
                    // Set the location field manually since the parser doesn't
                    skill_info.location = path.to_path_buf();
                    skills.push(skill_info);
                }
                Err(e) => {
                    warn!("Failed to parse skill file {:?}: {}", path, e);
                }
            }
        }
    }

    skills
}

/// Merge skills with priority handling
///
/// For each skill in `new`:
/// - If skill doesn't exist in target, add it
/// - If skill exists, compare scopes (lower value = higher priority)
/// - Only add if new skill has higher priority (lower scope value)
fn merge_skills(
    target: &mut HashMap<String, SkillInfo>,
    new: Vec<SkillInfo>,
    warnings: &mut Vec<String>,
) {
    for skill in new {
        let name = skill.name.clone();

        if let Some(existing) = target.get(&name) {
            // Compare scopes - lower ordinal = higher priority
            if skill.scope < existing.scope {
                debug!(
                    "Skill '{}' found with higher priority scope: {:?} > {:?}",
                    name, existing.scope, skill.scope
                );
                target.insert(name, skill);
            } else if skill.scope == existing.scope {
                // Same scope, prefer the one with location
                let existing_has_location = !existing.location.as_os_str().is_empty();
                let new_has_location = !skill.location.as_os_str().is_empty();

                if new_has_location && !existing_has_location {
                    debug!(
                        "Skill '{}' same scope but new has location, replacing",
                        name
                    );
                    target.insert(name, skill);
                } else {
                    warnings.push(format!(
                        "Duplicate skill '{}' with same or lower priority, skipping",
                        name
                    ));
                }
            } else {
                warnings.push(format!(
                    "Duplicate skill '{}' with lower priority scope, skipping",
                    name
                ));
            }
        } else {
            debug!("Adding new skill '{}' with scope {:?}", name, skill.scope);
            target.insert(name, skill);
        }
    }
}

/// Discover skills with default options
///
/// Convenience function that creates default DiscoveryOptions and discovers skills.
pub async fn discover_skills(directory: Option<PathBuf>) -> Result<DiscoveryResult, SkillError> {
    let options = DiscoveryOptions {
        directory,
        ..Default::default()
    };

    discover_skills_with_options(options).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn test_discover_skills_empty_directory() {
        let temp = tempdir().unwrap();
        let result = discover_skills(Some(temp.path().to_path_buf())).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.skills.is_empty());
    }

    #[tokio::test]
    async fn test_discover_from_directory_nonexistent() {
        let skills =
            discover_from_directory(Path::new("/nonexistent/path"), SkillScope::Project).await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_discover_skills_with_nested_skill_file() {
        let temp = tempdir().unwrap();

        // Create .opencode/skills/my-skill/SKILL.md
        let skill_dir = temp.path().join(".opencode/skills/test-skill");
        fs::create_dir_all(&skill_dir).await.unwrap();

        let skill_content = r#"---
name: test-skill
description: A test skill
---

# Test Skill

This is a test.
"#;
        fs::write(skill_dir.join("SKILL.md"), skill_content)
            .await
            .unwrap();

        let result = discover_skills(Some(temp.path().to_path_buf())).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.skills.contains_key("test-skill"));
    }

    #[test]
    fn test_merge_skills_priority() {
        let mut target: HashMap<String, SkillInfo> = HashMap::new();
        let mut warnings = Vec::new();

        // Add initial skill with Project scope
        let existing = SkillInfo {
            name: "test".to_string(),
            description: "existing".to_string(),
            location: PathBuf::from("/existing/path/SKILL.md"),
            content: "content".to_string(),
            scope: SkillScope::Project,
            metadata: Default::default(),
        };
        target.insert("test".to_string(), existing);

        // Try to merge with lower priority (User scope)
        let new_lower = SkillInfo {
            name: "test".to_string(),
            description: "new lower".to_string(),
            location: PathBuf::from("/new/path/SKILL.md"),
            content: "new content".to_string(),
            scope: SkillScope::User,
            metadata: Default::default(),
        };
        merge_skills(&mut target, vec![new_lower], &mut warnings);

        // Should keep the existing (higher priority)
        assert_eq!(target.get("test").unwrap().description, "existing");
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_merge_skills_higher_priority() {
        let mut target: HashMap<String, SkillInfo> = HashMap::new();
        let mut warnings = Vec::new();

        // Add initial skill with User scope (lower priority)
        let existing = SkillInfo {
            name: "test".to_string(),
            description: "existing".to_string(),
            location: PathBuf::from("/existing/path/SKILL.md"),
            content: "content".to_string(),
            scope: SkillScope::User,
            metadata: Default::default(),
        };
        target.insert("test".to_string(), existing);

        // Merge with higher priority (Project scope)
        let new_higher = SkillInfo {
            name: "test".to_string(),
            description: "new higher".to_string(),
            location: PathBuf::from("/new/path/SKILL.md"),
            content: "new content".to_string(),
            scope: SkillScope::Project,
            metadata: Default::default(),
        };
        merge_skills(&mut target, vec![new_higher], &mut warnings);

        // Should use the new higher priority one
        assert_eq!(target.get("test").unwrap().description, "new higher");
    }

    #[test]
    fn test_merge_skills_new() {
        let mut target: HashMap<String, SkillInfo> = HashMap::new();
        let mut warnings = Vec::new();

        let new_skill = SkillInfo {
            name: "new-skill".to_string(),
            description: "brand new".to_string(),
            location: PathBuf::from("/path/SKILL.md"),
            content: "content".to_string(),
            scope: SkillScope::Project,
            metadata: Default::default(),
        };
        merge_skills(&mut target, vec![new_skill], &mut warnings);

        assert_eq!(target.len(), 1);
        assert!(target.contains_key("new-skill"));
    }
}
