//! Write tool - write file contents

use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Write tool - write file contents
pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write file contents"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'path' argument".to_string()))?;

        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'content' argument".to_string()))?;

        fs::write(path, content).await?;

        Ok(ToolResult {
            output: format!("Successfully wrote to {}", path),
            metadata: None,
        })
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_write_new_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "path": path,
            "content": "Hello, World!"
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result.output, format!("Successfully wrote to {}", path));

        // Verify content was written
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_overwrite_existing() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        // Write initial content
        tokio::fs::write(path, "Initial content").await.unwrap();

        let tool = WriteTool::new();
        let args = serde_json::json!({
            "path": path,
            "content": "New content"
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result.output, format!("Successfully wrote to {}", path));

        // Verify content was overwritten
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "New content");
    }

    #[tokio::test]
    async fn test_write_missing_args() {
        let tool = WriteTool::new();

        // Missing path
        let args = serde_json::json!({
            "content": "test"
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));

        // Missing content
        let args = serde_json::json!({
            "path": "/tmp/test.txt"
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }
}
