//! List tool - list directory contents

use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::tool::error::Result;
use crate::tool::traits::{Tool, ToolResult};

const LIMIT: usize = 100;

/// Default ignore patterns for directory listing
pub const IGNORE_PATTERNS: &[&str] = &[
    "node_modules",
    "__pycache__",
    ".git",
    "dist",
    "build",
    "target",
    "vendor",
    "bin",
    "obj",
    ".idea",
    ".vscode",
    ".zig-cache",
    "zig-out",
    ".coverage",
    "coverage",
    "tmp",
    "temp",
    ".cache",
    "cache",
    "logs",
    ".venv",
    "venv",
    "env",
];

/// List tool - list directory contents as tree
pub struct ListTool;

impl ListTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ListTool {
    fn name(&self) -> &str {
        "list"
    }

    fn description(&self) -> &str {
        "Read a file or directory from the local filesystem. If the path does not exist, an error is returned. By default, this tool returns up to 2000 lines from the start of the file. The offset parameter is the line number to start from (1-indexed). To read later sections, call this tool again with a larger offset. Use the grep tool to find specific content in large files or files with long lines. If you are unsure of the correct file path, use the glob tool to look up filenames by glob pattern. Contents are returned with each line prefixed by its line number as `<line>: <content>`. For example, if a file has contents \"foo\\n\", you will receive \"1: foo\\n\". For directories, entries are returned one per line (without line numbers) with a trailing `/` for subdirectories. Any line longer than 2000 characters is truncated."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the directory to list (must be absolute, not relative)"
                },
                "ignore": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of glob patterns to ignore"
                }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let search_path = args["path"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Build ignore set
        let mut ignore_set: BTreeSet<String> = IGNORE_PATTERNS.iter().map(|s| s.to_string()).collect();
        if let Some(ignore_arr) = args["ignore"].as_array() {
            for pattern in ignore_arr {
                if let Some(s) = pattern.as_str() {
                    ignore_set.insert(s.to_string());
                }
            }
        }

        // Collect files
        let mut files: Vec<String> = Vec::new();

        for entry in WalkDir::new(&search_path)
            .follow_links(false)
            .min_depth(1)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if files.len() >= LIMIT {
                break;
            }
            
            if !entry.file_type().is_file() {
                continue;
            }
            
            // Only check relative path components for ignore patterns
            let relative = entry.path().strip_prefix(&search_path).unwrap_or(entry.path());
            if is_ignored(relative, &ignore_set) {
                continue;
            }

            files.push(relative.display().to_string());
        }

        let truncated = files.len() >= LIMIT;

        let mut output = format!("{}/\n", search_path.display());
        
        files.sort();
        
        for file in &files {
            output.push_str(&format!("  {}\n", file));
        }

        if truncated {
            output.push_str(&format!(
                "\n(Results truncated: showing first {} files. Consider using a more specific path.)\n",
                LIMIT
            ));
        }

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "count": files.len(),
            "truncated": truncated
        })))
    }
}

fn is_ignored(path: &std::path::Path, ignore_set: &BTreeSet<String>) -> bool {
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if let Some(s) = os_str.to_str() {
                if ignore_set.contains(s) {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_with_ignore() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::create_dir_all(temp_path.join("node_modules/pkg")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "").unwrap();
        fs::write(temp_path.join("node_modules/pkg/index.js"), "").unwrap();

        let tool = ListTool::new();
        let args = serde_json::json!({
            "path": temp_path.to_str().unwrap()
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("src"), "missing 'src': {}", result.output);
        assert!(!result.output.contains("node_modules"), "found ignored 'node_modules': {}", result.output);
    }

    #[tokio::test]
    async fn test_list_with_temp_structure() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test structure
        fs::create_dir_all(temp_path.join("src")).unwrap();
        fs::create_dir_all(temp_path.join("tests")).unwrap();
        fs::write(temp_path.join("src/main.rs"), "").unwrap();
        fs::write(temp_path.join("src/lib.rs"), "").unwrap();
        fs::write(temp_path.join("tests/test.rs"), "").unwrap();
        fs::write(temp_path.join("Cargo.toml"), "").unwrap();

        let tool = ListTool::new();
        let args = serde_json::json!({
            "path": temp_path.to_str().unwrap()
        });

        let result = tool.execute(args).await.unwrap();
        let output = &result.output;
        assert!(output.contains("src"), "missing 'src': {}", output);
        assert!(output.contains("tests"), "missing 'tests': {}", output);
        assert!(output.contains("Cargo.toml"), "missing 'Cargo.toml': {}", output);
    }

    #[test]
    fn test_list_tool_name() {
        let tool = ListTool::new();
        assert_eq!(tool.name(), "list");
    }

    #[test]
    fn test_list_tool_default() {
        let tool: ListTool = Default::default();
        assert_eq!(tool.name(), "list");
    }

    #[test]
    fn test_is_ignored() {
        let mut ignore_set: BTreeSet<String> = BTreeSet::new();
        ignore_set.insert("node_modules".to_string());
        ignore_set.insert("target".to_string());

        assert!(is_ignored(std::path::Path::new("node_modules/pkg"), &ignore_set));
        assert!(is_ignored(std::path::Path::new("target/debug"), &ignore_set));
        assert!(!is_ignored(std::path::Path::new("src/main.rs"), &ignore_set));
    }
}