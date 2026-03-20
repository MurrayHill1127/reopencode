//! Glob tool - find files using glob patterns

use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::WalkDir;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

const LIMIT: usize = 100;

/// Glob tool - find files using glob patterns
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "glob"
    }

    fn description(&self) -> &str {
        "Fast file pattern matching tool with safety limits (60s timeout, 100 file limit). Supports glob patterns like \"**/*.js\" or \"src/**/*.ts\". Returns matching file paths sorted by modification time."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in. If not specified, the current working directory will be used. IMPORTANT: Omit this field to use the default directory. DO NOT enter \"undefined\" or \"null\" - simply omit it for the default behavior. Must be a valid directory path if provided."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'pattern' argument".to_string()))?;

        let search_path = args["path"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Build glob matcher
        let glob_pattern = globset::Glob::new(pattern)
            .map_err(|e| ToolError::Parse(format!("Invalid glob pattern: {}", e)))?;
        let matcher = globset::GlobSetBuilder::new()
            .add(glob_pattern)
            .build()
            .map_err(|e| ToolError::Parse(format!("Failed to build glob matcher: {}", e)))?;

        let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in WalkDir::new(&search_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if files.len() >= LIMIT {
                break;
            }

            let path = entry.path();
            let relative = path.strip_prefix(&search_path).unwrap_or(path);

            if matcher.is_match(relative) {
                let mtime = entry.metadata().ok().and_then(|m| m.modified().ok());
                if let Some(mtime) = mtime {
                    files.push((path.to_path_buf(), mtime));
                }
            }
        }

        // Sort by modification time (newest first)
        files.sort_by(|a, b| b.1.cmp(&a.1));

        let truncated = files.len() >= LIMIT;

        let output = if files.is_empty() {
            "No files found".to_string()
        } else {
            let mut lines: Vec<String> = files.iter().map(|(p, _)| p.display().to_string()).collect();
            if truncated {
                lines.push(String::new());
                lines.push(format!(
                    "(Results are truncated: showing first {} results. Consider using a more specific path or pattern.)",
                    LIMIT
                ));
            }
            lines.join("\n")
        };

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "count": files.len(),
            "truncated": truncated
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_glob_no_matches() {
        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.nonexistent_ext_xyz"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("No files found"));
    }

    #[tokio::test]
    async fn test_glob_missing_pattern() {
        let tool = GlobTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_glob_invalid_pattern() {
        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "[invalid"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_glob_with_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        std::fs::write(temp_path.join("test1.rs"), "").unwrap();
        std::fs::write(temp_path.join("test2.rs"), "").unwrap();
        std::fs::write(temp_path.join("test3.txt"), "").unwrap();

        let tool = GlobTool::new();
        let args = serde_json::json!({
            "pattern": "*.rs",
            "path": temp_path.to_str().unwrap()
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("test1.rs"));
        assert!(result.output.contains("test2.rs"));
        assert!(!result.output.contains("test3.txt"));
    }

    #[test]
    fn test_glob_tool_name() {
        let tool = GlobTool::new();
        assert_eq!(tool.name(), "glob");
    }

    #[test]
    fn test_glob_tool_default() {
        let tool: GlobTool = Default::default();
        assert_eq!(tool.name(), "glob");
    }
}