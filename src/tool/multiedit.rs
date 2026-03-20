//! MultiEdit tool - apply multiple edits to a single file in one operation

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// MultiEdit tool - apply multiple edits to a single file in one operation
pub struct MultiEditTool;

impl MultiEditTool {
    /// Create a new MultiEditTool instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for MultiEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MultiEditTool {
    fn name(&self) -> &str {
        "multiedit"
    }

    fn description(&self) -> &str {
        "Apply multiple edits to a single file in one operation"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "filePath": {
                    "type": "string",
                    "description": "The absolute path to the file to modify"
                },
                "edits": {
                    "type": "array",
                    "description": "Array of edit operations to perform sequentially on the file",
                    "items": {
                        "type": "object",
                        "properties": {
                            "oldString": {
                                "type": "string",
                                "description": "The text to replace"
                            },
                            "newString": {
                                "type": "string",
                                "description": "The text to replace it with (must be different from oldString)"
                            },
                            "replaceAll": {
                                "type": "boolean",
                                "description": "Replace all occurrences of oldString (default false)"
                            }
                        },
                        "required": ["oldString", "newString"]
                    }
                }
            },
            "required": ["filePath", "edits"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Extract required filePath argument
        let file_path = args["filePath"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'filePath' argument".to_string()))?;

        // Extract required edits array
        let edits = args["edits"]
            .as_array()
            .ok_or_else(|| ToolError::Parse("Missing 'edits' argument or not an array".to_string()))?;

        if edits.is_empty() {
            return Err(ToolError::Parse("'edits' array cannot be empty".to_string()));
        }

        let mut validated_edits = Vec::with_capacity(edits.len());
        for (index, edit) in edits.iter().enumerate() {
            let old_string = edit["oldString"]
                .as_str()
                .ok_or_else(|| {
                    ToolError::Parse(format!("Missing 'oldString' in edit at index {}", index))
                })?;

            let new_string = edit["newString"]
                .as_str()
                .ok_or_else(|| {
                    ToolError::Parse(format!("Missing 'newString' in edit at index {}", index))
                })?;

            let replace_all = edit["replaceAll"].as_bool().unwrap_or(false);
            validated_edits.push((old_string.to_string(), new_string.to_string(), replace_all));
        }

        // Read file content once at start
        let mut content = tokio::fs::read_to_string(file_path).await?;

        let total_edits = edits.len();
        let mut total_occurrences = 0;

        // Apply each edit sequentially to the content
        for (index, (old_string, new_string, replace_all)) in validated_edits.into_iter().enumerate() {
            // Check if old_string exists in current content
            if !content.contains(&old_string) {
                return Err(ToolError::NotFound(format!(
                    "String not found in edit {}: {}",
                    index,
                    old_string
                )));
            }

            // Perform replacement
            let occurrences = if replace_all {
                let count = content.matches(&old_string).count();
                content = content.replace(&old_string, &new_string);
                count
            } else {
                content = content.replacen(&old_string, &new_string, 1);
                1
            };

            total_occurrences += occurrences;
        }

        // Write final content to file once at end
        tokio::fs::write(file_path, content).await?;

        // Return result with metadata
        Ok(ToolResult::new(format!(
            "Successfully applied {} edits to {}",
            total_edits, file_path
        ))
        .with_metadata(serde_json::json!({
            "totalEdits": total_edits,
            "occurrencesReplaced": total_occurrences
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_multiedit_single_edit() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world!").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "world",
                    "newString": "Rust"
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.output,
            format!("Successfully applied 1 edits to {}", path)
        );
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 1,
                "occurrencesReplaced": 1
            }))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "Hello Rust!\n");
    }

    #[tokio::test]
    async fn test_multiedit_multiple_edits_sequential() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world! Hello universe!").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "Hello",
                    "newString": "Hi"
                },
                {
                    "oldString": "Hi world!",
                    "newString": "Greetings world!"
                },
                {
                    "oldString": "Hello universe!",
                    "newString": "Goodbye universe!"
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 3,
                "occurrencesReplaced": 3
            }))
        );

        // Verify file content - each edit operates on result of previous
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "Greetings world! Goodbye universe!\n");
    }

    #[tokio::test]
    async fn test_multiedit_with_replace_all() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "foo bar foo baz foo").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "foo",
                    "newString": "hello",
                    "replaceAll": true
                },
                {
                    "oldString": "hello bar hello",
                    "newString": "replaced section"
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 2,
                "occurrencesReplaced": 4
            }))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "replaced section baz hello\n");
    }

    #[tokio::test]
    async fn test_multiedit_file_not_found() {
        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": "/nonexistent/path/file.txt",
            "edits": [
                {
                    "oldString": "old",
                    "newString": "new"
                }
            ]
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_io());
    }

    #[tokio::test]
    async fn test_multiedit_string_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Hello world!").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "nonexistent",
                    "newString": "replacement"
                }
            ]
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_not_found());
        assert!(err.to_string().contains("String not found"));
    }

    #[tokio::test]
    async fn test_multiedit_missing_required_args() {
        let tool = MultiEditTool::new();

        // Missing filePath
        let args = serde_json::json!({
            "edits": [
                {
                    "oldString": "old",
                    "newString": "new"
                }
            ]
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'filePath'")
        );

        // Missing edits
        let args = serde_json::json!({
            "filePath": "/tmp/test.txt"
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'edits'")
        );

        // Empty edits array
        let args = serde_json::json!({
            "filePath": "/tmp/test.txt",
            "edits": []
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'edits' array cannot be empty")
        );

        // Missing oldString in edit
        let args = serde_json::json!({
            "filePath": "/tmp/test.txt",
            "edits": [
                {
                    "newString": "new"
                }
            ]
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'oldString'")
        );

        // Missing newString in edit
        let args = serde_json::json!({
            "filePath": "/tmp/test.txt",
            "edits": [
                {
                    "oldString": "old"
                }
            ]
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
    async fn test_multiedit_combined_metadata() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line1\nline2\nline3\nline2\nline4").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "line1",
                    "newString": "first"
                },
                {
                    "oldString": "line2",
                    "newString": "second",
                    "replaceAll": true
                },
                {
                    "oldString": "line3",
                    "newString": "third"
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 3,
                "occurrencesReplaced": 4  // 1 + 2 + 1
            }))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "first\nsecond\nthird\nsecond\nline4\n");
    }

    #[tokio::test]
    async fn test_multiedit_empty_replacement() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "remove this and that").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "remove this ",
                    "newString": ""
                },
                {
                    "oldString": "and that",
                    "newString": ""
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 2,
                "occurrencesReplaced": 2
            }))
        );

        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "\n");
    }

    #[tokio::test]
    async fn test_multiedit_multiline_content() {
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(
            temp_file,
            "line1\nline2\nline3\nline4\nline5"
        )
        .unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "line2\nline3",
                    "newString": "replaced block"
                },
                {
                    "oldString": "line4\nline5",
                    "newString": "another block"
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 2,
                "occurrencesReplaced": 2
            }))
        );

        // Verify file content
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "line1\nreplaced block\nanother block");
    }

    #[tokio::test]
    async fn test_multiedit_chain_edits() {
        // Test that each edit operates on the result of the previous edit
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "aaa").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "aaa",
                    "newString": "bbb"
                },
                {
                    "oldString": "bbb",
                    "newString": "ccc"
                },
                {
                    "oldString": "ccc",
                    "newString": "ddd"
                }
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(
            result.metadata,
            Some(serde_json::json!({
                "totalEdits": 3,
                "occurrencesReplaced": 3
            }))
        );

        // Verify file content - each edit operates on result of previous
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "ddd\n");
    }

    #[tokio::test]
    async fn test_multiedit_second_edit_fails() {
        // Test that if a later edit fails, the earlier edits were still applied
        // (in-memory, so if string not found in second edit, it returns error)
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "original content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "original",
                    "newString": "modified"
                },
                {
                    "oldString": "nonexistent",
                    "newString": "replacement"
                }
            ]
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_not_found());
        assert!(err.to_string().contains("String not found in edit 1"));

        // Since the edit failed, the file should NOT be written (no partial application)
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "original content\n");
    }

    #[tokio::test]
    async fn test_multiedit_empty_file() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = MultiEditTool::new();
        let args = serde_json::json!({
            "filePath": path,
            "edits": [
                {
                    "oldString": "anything",
                    "newString": "replacement"
                }
            ]
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_not_found());
    }
}