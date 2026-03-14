//! Read tool - read file contents

use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;

use crate::tool::error::{ToolError, Result};
use crate::tool::traits::{Tool, ToolResult};

/// Read tool - read file contents
pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read file contents"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'path' argument".to_string()))?;

        let content = fs::read_to_string(path).await?;

        Ok(ToolResult {
            output: content,
            metadata: None,
        })
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_existing_file() {
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "path": "Cargo.toml"
        });

        let result = tool.execute(args).await;
        assert!(result.is_ok());

        let tool_result = result.unwrap();
        assert!(tool_result.output.contains("reopencode"));
        assert!(tool_result.output.contains("[package]"));
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool::new();
        let args = serde_json::json!({
            "path": "nonexistent_file_12345.txt"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_io());
    }

    #[tokio::test]
    async fn test_read_missing_path_arg() {
        let tool = ReadTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Parse(_)));
    }

    #[test]
    fn test_read_tool_new() {
        let tool = ReadTool::new();
        assert_eq!(tool.name(), "read");
        assert_eq!(tool.description(), "Read file contents");
    }

    #[test]
    fn test_read_tool_default() {
        let tool: ReadTool = Default::default();
        assert_eq!(tool.name(), "read");
    }

    #[test]
    fn test_read_tool_parameters() {
        let tool = ReadTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["path"].is_object());
        assert_eq!(params["properties"]["path"]["type"], "string");
        assert!(params["required"].is_array());
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("path")));
    }
}