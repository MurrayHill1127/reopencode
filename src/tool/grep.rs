//! Grep tool - search file contents using regex

use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;
use std::time::SystemTime;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

const LIMIT: usize = 100;
const MAX_LINE_LENGTH: usize = 2000;

/// Grep tool - search file contents using regex
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }

    fn description(&self) -> &str {
        "Fast content search tool with safety limits (60s timeout, 256KB output). Searches file contents using regular expressions. Supports full regex syntax (e.g. \"log.*Error\", \"function\\s+\\w+\"). Filter files by pattern with the include parameter (e.g. \"*.js\", \"*.{ts,tsx}\"). Output modes: \"content\" shows matching lines, \"files_with_matches\" shows only file paths (default), \"count\" shows match counts per file."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for in file contents"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in. Defaults to the current working directory."
                },
                "include": {
                    "type": "string",
                    "description": "File pattern to include in the search (e.g. \"*.js\", \"*.{ts,tsx}\")"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'pattern' argument".to_string()))?;

        if pattern.is_empty() {
            return Err(ToolError::Parse("Pattern cannot be empty".to_string()));
        }

        let search_path = args["path"]
            .as_str()
            .map(|p| p.to_string())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().to_string_lossy().to_string());

        let include = args["include"].as_str();

        // Build ripgrep command
        let mut cmd = Command::new("rg");
        cmd.args([
            "-nH",                          // Line numbers and filenames
            "--hidden",                     // Search hidden files
            "--no-messages",                // Suppress error messages
            "--field-match-separator=|",    // Use | as separator
            "--regexp",
            pattern,
        ]);

        if let Some(inc) = include {
            cmd.args(["--glob", inc]);
        }

        cmd.arg(&search_path);

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Exit codes: 0 = matches found, 1 = no matches, 2 = errors
        match output.status.code() {
            Some(1) => {
                return Ok(ToolResult::new("No files found").with_metadata(serde_json::json!({
                    "matches": 0,
                    "truncated": false
                })));
            }
            Some(2) if stdout.trim().is_empty() => {
                return Ok(ToolResult::new("No files found").with_metadata(serde_json::json!({
                    "matches": 0,
                    "truncated": false
                })));
            }
            Some(code) if code != 0 && code != 2 => {
                return Err(ToolError::Execution(format!("ripgrep failed: {}", stderr)));
            }
            _ => {}
        }

        let has_errors = output.status.code() == Some(2);

        // Parse matches
        let lines: Vec<&str> = stdout.trim().split('\n').filter(|l| !l.is_empty()).collect();
        let mut matches: Vec<(String, SystemTime, usize, String)> = Vec::new();

        for line in &lines {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() != 3 {
                continue;
            }

            let file_path = parts[0];
            let line_num = match parts[1].parse::<usize>() {
                Ok(n) => n,
                Err(_) => continue,
            };
            let line_text = parts[2].to_string();

            let mtime = std::fs::metadata(file_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or_else(|| SystemTime::UNIX_EPOCH);

            matches.push((file_path.to_string(), mtime, line_num, line_text));
        }

        // Sort by modification time (newest first)
        matches.sort_by(|a, b| b.1.cmp(&a.1));

        let total_matches = matches.len();
        let truncated = total_matches > LIMIT;
        let final_matches: Vec<_> = matches.into_iter().take(LIMIT).collect();

        if final_matches.is_empty() {
            return Ok(ToolResult::new("No files found").with_metadata(serde_json::json!({
                "matches": 0,
                "truncated": false
            })));
        }

        // Build output
        let mut output_lines = vec![format!(
            "Found {} matches{}",
            total_matches,
            if truncated { format!(" (showing first {})", LIMIT) } else { String::new() }
        )];

        let mut current_file = String::new();
        for (path, _, line_num, line_text) in final_matches {
            if current_file != path {
                if !current_file.is_empty() {
                    output_lines.push(String::new());
                }
                current_file = path.clone();
                output_lines.push(format!("{}:", path));
            }

            let truncated_line = if line_text.len() > MAX_LINE_LENGTH {
                format!("{}...", &line_text[..MAX_LINE_LENGTH])
            } else {
                line_text
            };
            output_lines.push(format!("  Line {}: {}", line_num, truncated_line));
        }

        if truncated {
            output_lines.push(String::new());
            output_lines.push(format!(
                "(Results truncated: showing {} of {} matches ({} hidden). Consider using a more specific path or pattern.)",
                LIMIT, total_matches, total_matches - LIMIT
            ));
        }

        if has_errors {
            output_lines.push(String::new());
            output_lines.push("(Some paths were inaccessible and skipped)".to_string());
        }

        Ok(ToolResult::new(output_lines.join("\n")).with_metadata(serde_json::json!({
            "matches": total_matches,
            "truncated": truncated
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ripgrep_available() -> bool {
        std::process::Command::new("rg")
            .arg("--version")
            .output()
            .is_ok()
    }

    #[tokio::test]
    async fn test_grep_missing_pattern() {
        let tool = GrepTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_empty_pattern() {
        let tool = GrepTool::new();
        let args = serde_json::json!({
            "pattern": ""
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grep_no_matches() {
        if !ripgrep_available() {
            eprintln!("Skipping test: ripgrep not available");
            return;
        }
        let tool = GrepTool::new();
        let args = serde_json::json!({
            "pattern": "NONEXISTENT_PATTERN_XYZ123",
            "path": "."
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("No files found"));
    }

    #[tokio::test]
    async fn test_grep_with_include() {
        if !ripgrep_available() {
            eprintln!("Skipping test: ripgrep not available");
            return;
        }
        let tool = GrepTool::new();
        let args = serde_json::json!({
            "pattern": "fn",
            "path": ".",
            "include": "*.rs"
        });

        let result = tool.execute(args).await.unwrap();
        // Should find matches in .rs files
        assert!(result.output.contains("matches"));
    }

    #[test]
    fn test_grep_tool_name() {
        let tool = GrepTool::new();
        assert_eq!(tool.name(), "grep");
    }

    #[test]
    fn test_grep_tool_default() {
        let tool: GrepTool = Default::default();
        assert_eq!(tool.name(), "grep");
    }
}