//! Skill tool - load specialized skills
//!
//! This is a placeholder implementation. Full implementation requires
//! skill module integration for loading skill content from files.

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Available skills placeholder
const AVAILABLE_SKILLS: &[(&str, &str)] = &[
    ("playwright", "Browser automation and testing with Playwright"),
    ("frontend-ui-ux", "Frontend development, UI/UX design and implementation"),
    ("git-master", "Advanced git operations and workflows"),
    ("dev-browser", "Development browser automation"),
];

/// Skill tool - load specialized skills
pub struct SkillTool;

impl SkillTool {
    pub fn new() -> Self {
        Self
    }

    /// Get list of available skills
    pub fn available_skills() -> Vec<(&'static str, &'static str)> {
        AVAILABLE_SKILLS.to_vec()
    }
}

#[async_trait]
impl Tool for SkillTool {
    fn name(&self) -> &str {
        "skill"
    }

    fn description(&self) -> &str {
        r#"Load a specialized skill that provides domain-specific instructions and workflows.

When you recognize that a task matches one of the available skills listed below, use this tool to load the full skill instructions.

The skill will inject detailed instructions, workflows, and access to bundled resources (scripts, references, templates) into the conversation context.

Available skills:
- playwright: Browser automation and testing with Playwright
- frontend-ui-ux: Frontend development, UI/UX design and implementation
- git-master: Advanced git operations and workflows
- dev-browser: Development browser automation

Usage notes:
- Tool output includes a `<skill_content name="...">` block with the loaded content
- Skills provide specialized sets of instructions for particular tasks
- Invoke this tool to load a skill when a task matches one of the available skills"#
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name of the skill from available_skills (e.g., playwright, frontend-ui-ux, git-master, dev-browser)"
                }
            },
            "required": ["name"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'name' argument".to_string()))?;

        // Check if skill exists in available skills
        let skill = AVAILABLE_SKILLS.iter().find(|(n, _)| *n == name);

        let output = if let Some((skill_name, description)) = skill {
            // Placeholder implementation - skill file loading required
            format!(
                r#"<skill_content name="{}">
# Skill: {}

{}

## Placeholder Implementation

Full skill loading requires skill module integration.
This would load the actual skill content from the skill file.

Base directory for this skill would be determined by the skill's location.
Relative paths in this skill (e.g., scripts/, reference/) are relative to that base directory.
</skill_content>"#,
                skill_name, skill_name, description
            )
        } else {
            let available: String = AVAILABLE_SKILLS
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ");

            return Err(ToolError::NotFound(format!(
                "Skill '{}' not found. Available skills: {}",
                name, available
            )));
        };

        Ok(ToolResult {
            output,
            metadata: Some(serde_json::json!({
                "name": name,
                "placeholder": true
            })),
        })
    }
}

impl Default for SkillTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_tool_new() {
        let tool = SkillTool::new();
        assert_eq!(tool.name(), "skill");
    }

    #[test]
    fn test_skill_tool_default() {
        let tool: SkillTool = Default::default();
        assert_eq!(tool.name(), "skill");
    }

    #[test]
    fn test_skill_tool_parameters() {
        let tool = SkillTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["name"].is_object());
        assert_eq!(params["properties"]["name"]["type"], "string");

        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("name")));
    }

    #[test]
    fn test_available_skills() {
        let skills = SkillTool::available_skills();
        assert!(!skills.is_empty());
        assert!(skills.iter().any(|(name, _)| *name == "playwright"));
        assert!(skills.iter().any(|(name, _)| *name == "frontend-ui-ux"));
    }

    #[tokio::test]
    async fn test_skill_execute_existing_skill() {
        let tool = SkillTool::new();
        let args = serde_json::json!({
            "name": "playwright"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("playwright"));
        assert!(result.output.contains("<skill_content"));
        assert!(result.metadata.is_some());
    }

    #[tokio::test]
    async fn test_skill_execute_nonexistent_skill() {
        let tool = SkillTool::new();
        let args = serde_json::json!({
            "name": "nonexistent_skill"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }

    #[tokio::test]
    async fn test_skill_execute_missing_name() {
        let tool = SkillTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }

    #[test]
    fn test_skill_description_contains_skills() {
        let tool = SkillTool::new();
        let desc = tool.description();
        assert!(desc.contains("playwright"));
        assert!(desc.contains("frontend-ui-ux"));
    }
}