//! Edit tool - edit file contents with string replacement

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Edit tool - edit file contents with string replacement
pub struct EditTool;

impl EditTool {
    /// Create a new EditTool instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit file contents with string replacement"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "File path to edit"
                },
                "oldString": {
                    "type": "string",
                    "description": "String to find and replace"
                },
                "newString": {
                    "type": "string",
                    "description": "Replacement string"
                },
                "replaceAll": {
                    "type": "boolean",
                    "description": "Replace all occurrences (default: false)"
                }
            },
            "required": ["filePath", "oldString", "newString"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Extract required arguments
        let file_path = args["filePath"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'filePath' argument".to_string()))?;

        let old_string = args["oldString"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'oldString' argument".to_string()))?;

        let new_string = args["newString"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'newString' argument".to_string()))?;

        // Extract optional replaceAll argument (defaults to false)
        let replace_all = args["replaceAll"].as_bool().unwrap_or(false);

        // Read file content
        let content = tokio::fs::read_to_string(file_path).await?;

        // Check if old_string exists in content
        if !content.contains(old_string) {
            return Err(ToolError::NotFound(format!(
                "String not found: {}",
                old_string
            )));
        }

        // Perform replacement
        let (modified_content, occurrences_replaced) = if replace_all {
            let count = content.matches(old_string).count();
            let modified = content.replace(old_string, new_string);
            (modified, count)
        } else {
            let modified = content.replacen(old_string, new_string, 1);
            (modified, 1)
        };

        // Write modified content back
        tokio::fs::write(file_path, modified_content).await?;

        // Return result with metadata
        Ok(
            ToolResult::new(format!("Successfully edited {}", file_path)).with_metadata(
                serde_json::json!({
                    "occurrencesReplaced": occurrences_replaced
                }),
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_edit_single_replacement() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world!").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "world",
            "newString": "Rust"
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result.output, format!("Successfully edited {}", path));
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({"occurrencesReplaced": 1}))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "Hello Rust!\n");
    }

    #[tokio::test]
    async fn test_edit_replace_all() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "foo bar foo baz foo").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "foo",
            "newString": "hello",
            "replaceAll": true
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({"occurrencesReplaced": 3}))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "hello bar hello baz hello\n");
    }

    #[tokio::test]
    async fn test_edit_string_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world!").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "nonexistent",
            "newString": "replacement"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_not_found());
        assert!(err.to_string().contains("String not found: nonexistent"));
    }

    #[tokio::test]
    async fn test_edit_file_not_found() {
        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": "/nonexistent/path/file.txt",
            "oldString": "old",
            "newString": "new"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_io());
    }

    #[tokio::test]
    async fn test_edit_multiline_string() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line1\nline2\nline3").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "line2\nline3",
            "newString": "replaced"
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({"occurrencesReplaced": 1}))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "line1\nreplaced\n");
    }

    #[tokio::test]
    async fn test_edit_preserves_indentation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "    indented line\n        more indented").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "    indented line",
            "newString": "    replaced line"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Successfully edited"));

        // Verify file content preserves indentation
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "    replaced line\n        more indented");
    }

    #[tokio::test]
    async fn test_edit_special_characters() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"path = "/foo/bar", pattern = "\n\t""#).unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": r#"/foo/bar"#,
            "newString": r#"/baz/qux"#
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Successfully edited"));

        // Verify file content (writeln adds newline)
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(
            content,
            r#"path = "/baz/qux", pattern = "\n\t"
"#
        );
    }

    #[tokio::test]
    async fn test_edit_missing_required_args() {
        let tool = EditTool::new();

        // Missing filePath
        let args = serde_json::json!({
            "oldString": "old",
            "newString": "new"
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'filePath'")
        );

        // Missing oldString
        let args = serde_json::json!({
            "filePath": "/tmp/test.txt",
            "newString": "new"
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'oldString'")
        );

        // Missing newString
        let args = serde_json::json!({
            "filePath": "/tmp/test.txt",
            "oldString": "old"
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'newString'")
        );
    }

    #[tokio::test]
    async fn test_edit_default_replace_all_false() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "foo bar foo").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "foo",
            "newString": "hello"
        });

        let result = tool.execute(args).await.unwrap();
        // Should only replace first occurrence (default behavior)
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({"occurrencesReplaced": 1}))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "hello bar foo\n");
    }

    #[tokio::test]
    async fn test_edit_empty_replacement() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "remove this").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "remove this",
            "newString": ""
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({"occurrencesReplaced": 1}))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "\n");
    }

    #[tokio::test]
    async fn test_edit_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "oldString": "anything",
            "newString": "replacement"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_not_found());
    }
}
