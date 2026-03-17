//! SKILL.md file parser with YAML frontmatter support

use crate::skill::error::SkillError;
use crate::skill::{SkillInfo, SkillMetadata, SkillScope};
use std::path::Path;

fn parse_frontmatter(content: &str) -> Result<(serde_yaml::Value, String), SkillError> {
    if !content.starts_with("---\n") {
        return Ok((serde_yaml::Value::Null, content.to_string()));
    }

    let after_first = &content[4..];
    if let Some(end_idx) = after_first.find("\n---") {
        let yaml_str = &after_first[..end_idx];
        let body = &after_first[end_idx + 5..];

        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(yaml_str).map_err(|e| SkillError::ParseError(e.to_string()))?;

        Ok((yaml_value, body.to_string()))
    } else {
        Err(SkillError::ParseError("未闭合的 frontmatter".to_string()))
    }
}

fn parse_metadata(frontmatter: &serde_yaml::Value) -> Result<SkillMetadata, SkillError> {
    let mut metadata = SkillMetadata::default();

    if let serde_yaml::Value::Mapping(map) = frontmatter {
        if let Some(serde_yaml::Value::String(v)) = map.get("model") {
            metadata.model = Some(v.clone());
        }
        if let Some(serde_yaml::Value::String(v)) = map.get("argument_hint") {
            metadata.argument_hint = Some(v.clone());
        }
        if let Some(serde_yaml::Value::String(v)) = map.get("agent") {
            metadata.agent = Some(v.clone());
        }
        if let Some(v) = map.get("subtask").and_then(|v| v.as_bool()) {
            metadata.subtask = Some(v);
        }
        if let Some(serde_yaml::Value::String(v)) = map.get("license") {
            metadata.license = Some(v.clone());
        }
        if let Some(serde_yaml::Value::String(v)) = map.get("compatibility") {
            metadata.compatibility = Some(v.clone());
        }
        if let Some(serde_yaml::Value::Mapping(m)) = map.get("metadata") {
            let mut meta = std::collections::HashMap::new();
            for (k, v) in m {
                if let (serde_yaml::Value::String(key), serde_yaml::Value::String(value)) = (k, v)
                {
                    meta.insert(key.clone(), value.clone());
                }
            }
            if !meta.is_empty() {
                metadata.metadata = Some(meta);
            }
        }
        if let Some(serde_yaml::Value::Sequence(seq)) = map.get("allowed_tools") {
            let tools: Vec<String> = seq
                .iter()
                .filter_map(|v| {
                    if let serde_yaml::Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect();
            if !tools.is_empty() {
                metadata.allowed_tools = Some(tools);
            }
        }
    }

    Ok(metadata)
}

pub fn parse_skill_content(content: &str) -> Result<(String, String, SkillMetadata), SkillError> {
    let (frontmatter, _body) = parse_frontmatter(content)?;

    let name = if let serde_yaml::Value::Mapping(map) = &frontmatter {
        map.get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| SkillError::ParseError("缺失必须字段: name".to_string()))?
    } else {
        return Err(SkillError::ParseError("缺失必须字段: name".to_string()));
    };

    let description = if let serde_yaml::Value::Mapping(map) = &frontmatter {
        map.get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| SkillError::ParseError("缺失必须字段: description".to_string()))?
    } else {
        return Err(SkillError::ParseError(
            "缺失必须字段: description".to_string(),
        ))?;
    };

    let metadata = parse_metadata(&frontmatter)?;

    Ok((name, description, metadata))
}

pub async fn parse_skill_file(
    path: &Path,
    scope: SkillScope,
) -> Result<SkillInfo, SkillError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| SkillError::IoError(path.to_path_buf(), e))?;

    let (name, description, metadata) = parse_skill_content(&content)?;

    Ok(SkillInfo {
        name,
        description,
        location: path.to_path_buf(),
        content: String::new(),
        scope,
        metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_skill_with_full_frontmatter() {
        let content = r#"---
name: test-skill
description: A test skill for unit testing
model: claude-opus-4-5
argument_hint: <task>
agent: oracle
license: MIT
compatibility: ">=1.0"
metadata:
  version: "1.0"
  author: test
allowed_tools:
  - read
  - write
  - grep
---

# Test Skill

This is the body content.
"#;

        let (name, desc, meta) = parse_skill_content(content).unwrap();
        assert_eq!(name, "test-skill");
        assert_eq!(desc, "A test skill for unit testing");
        assert_eq!(meta.model, Some("claude-opus-4-5".to_string()));
        assert_eq!(meta.argument_hint, Some("<task>".to_string()));
        assert_eq!(meta.agent, Some("oracle".to_string()));
        assert_eq!(meta.license, Some("MIT".to_string()));
        assert_eq!(meta.compatibility, Some(">=1.0".to_string()));
        assert_eq!(
            meta.metadata,
            Some(
                vec![
                    ("version".to_string(), "1.0".to_string()),
                    ("author".to_string(), "test".to_string())
                ]
                .into_iter()
                .collect()
            )
        );
        assert_eq!(
            meta.allowed_tools,
            Some(vec!["read".to_string(), "write".to_string(), "grep".to_string()])
        );
    }

    #[test]
    fn test_skill_with_no_frontmatter() {
        let content = r#"# My Skill

This is a skill without frontmatter.
name: not-actually-name
description: This should not be parsed
"#;

        let (frontmatter, body) = parse_frontmatter(content).unwrap();
        assert!(matches!(frontmatter, serde_yaml::Value::Null));
        assert!(body.contains("My Skill"));
    }

    #[test]
    fn test_missing_required_fields() {
        let content = r#"---
model: claude-opus-4-5
---

Just some content
"#;

        let result = parse_skill_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn test_invalid_yaml() {
        let content = r#"---
name: test
description: test
invalid: [unclosed
---

Content
"#;

        let result = parse_skill_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_unclosed_frontmatter() {
        let content = r#"---
name: test
description: test

This frontmatter is not closed
"#;

        let result = parse_skill_content(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("未闭合"));
    }

    #[tokio::test]
    async fn test_parse_skill_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("SKILL.md");
        std::fs::write(
            &file_path,
            r#"---
name: file-test-skill
description: A skill loaded from file
---

File content
"#,
        )
        .unwrap();

        let result = parse_skill_file(&file_path, SkillScope::Project).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "file-test-skill");
        assert_eq!(info.description, "A skill loaded from file");
        assert_eq!(info.scope, SkillScope::Project);
    }

    #[test]
    fn test_minimal_valid_skill() {
        let content = r#"---
name: minimal
description: A minimal skill
---

Content here
"#;

        let (name, desc, meta) = parse_skill_content(content).unwrap();
        assert_eq!(name, "minimal");
        assert_eq!(desc, "A minimal skill");
        assert!(meta.model.is_none());
        assert!(meta.allowed_tools.is_none());
    }
}