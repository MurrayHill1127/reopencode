//! LSP tool - Language Server Protocol operations (placeholder)
//!
//! This is a placeholder implementation. Full LSP integration will be added later.

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Supported LSP operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspOperation {
    GoToDefinition,
    FindReferences,
    Hover,
    DocumentSymbol,
    WorkspaceSymbol,
    GoToImplementation,
    PrepareCallHierarchy,
    IncomingCalls,
    OutgoingCalls,
}

impl LspOperation {
    /// Parse operation from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "goToDefinition" => Some(Self::GoToDefinition),
            "findReferences" => Some(Self::FindReferences),
            "hover" => Some(Self::Hover),
            "documentSymbol" => Some(Self::DocumentSymbol),
            "workspaceSymbol" => Some(Self::WorkspaceSymbol),
            "goToImplementation" => Some(Self::GoToImplementation),
            "prepareCallHierarchy" => Some(Self::PrepareCallHierarchy),
            "incomingCalls" => Some(Self::IncomingCalls),
            "outgoingCalls" => Some(Self::OutgoingCalls),
            _ => None,
        }
    }

    /// Get all valid operation names
    pub fn all_names() -> &'static [&'static str] {
        &[
            "goToDefinition",
            "findReferences",
            "hover",
            "documentSymbol",
            "workspaceSymbol",
            "goToImplementation",
            "prepareCallHierarchy",
            "incomingCalls",
            "outgoingCalls",
        ]
    }
}

/// LSP tool parameters
#[derive(Debug, Clone)]
pub struct LspParams {
    pub operation: LspOperation,
    pub file_path: String,
    pub line: u32,
    pub character: u32,
}

impl LspParams {
    /// Parse parameters from JSON value
    pub fn from_value(args: &Value) -> Result<Self> {
        let operation_str = args["operation"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'operation' argument".to_string()))?;

        let operation = LspOperation::from_str(operation_str)
            .ok_or_else(|| ToolError::Parse(format!("Invalid operation: {}", operation_str)))?;

        let file_path = args["filePath"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'filePath' argument".to_string()))?
            .to_string();

        let line = args["line"]
            .as_u64()
            .ok_or_else(|| ToolError::Parse("Missing 'line' argument".to_string()))?
            as u32;

        if line < 1 {
            return Err(ToolError::Parse("Line must be >= 1 (1-based)".to_string()));
        }

        let character = args["character"]
            .as_u64()
            .ok_or_else(|| ToolError::Parse("Missing 'character' argument".to_string()))?
            as u32;

        if character < 1 {
            return Err(ToolError::Parse(
                "Character must be >= 1 (1-based)".to_string(),
            ));
        }

        Ok(Self {
            operation,
            file_path,
            line,
            character,
        })
    }
}

/// LSP tool - Language Server Protocol operations
pub struct LspTool;

impl LspTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LspTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    fn description(&self) -> &str {
        "Interact with Language Server Protocol (LSP) servers to get code intelligence features. \
         Supported operations: goToDefinition, findReferences, hover, documentSymbol, workspaceSymbol, \
         goToImplementation, prepareCallHierarchy, incomingCalls, outgoingCalls. \
         All operations require filePath, line (1-based), and character (1-based). \
         Note: LSP servers must be configured for the file type."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": LspOperation::all_names(),
                    "description": "The LSP operation to perform"
                },
                "filePath": {
                    "type": "string",
                    "description": "The absolute or relative path to the file"
                },
                "line": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "The line number (1-based, as shown in editors)"
                },
                "character": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "The character offset (1-based, as shown in editors)"
                }
            },
            "required": ["operation", "filePath", "line", "character"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let params = LspParams::from_value(&args)?;

        // Placeholder implementation - full LSP integration coming soon
        let op_name = match params.operation {
            LspOperation::GoToDefinition => "goToDefinition",
            LspOperation::FindReferences => "findReferences",
            LspOperation::Hover => "hover",
            LspOperation::DocumentSymbol => "documentSymbol",
            LspOperation::WorkspaceSymbol => "workspaceSymbol",
            LspOperation::GoToImplementation => "goToImplementation",
            LspOperation::PrepareCallHierarchy => "prepareCallHierarchy",
            LspOperation::IncomingCalls => "incomingCalls",
            LspOperation::OutgoingCalls => "outgoingCalls",
        };

        let output = format!(
            "LSP integration coming soon. \
             Operation: {} on {}:{}:{}",
            op_name, params.file_path, params.line, params.character
        );

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "operation": op_name,
            "filePath": params.file_path,
            "line": params.line,
            "character": params.character,
            "placeholder": true
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_operation_from_str() {
        assert_eq!(
            LspOperation::from_str("goToDefinition"),
            Some(LspOperation::GoToDefinition)
        );
        assert_eq!(
            LspOperation::from_str("findReferences"),
            Some(LspOperation::FindReferences)
        );
        assert_eq!(
            LspOperation::from_str("hover"),
            Some(LspOperation::Hover)
        );
        assert_eq!(
            LspOperation::from_str("documentSymbol"),
            Some(LspOperation::DocumentSymbol)
        );
        assert_eq!(
            LspOperation::from_str("workspaceSymbol"),
            Some(LspOperation::WorkspaceSymbol)
        );
        assert_eq!(
            LspOperation::from_str("goToImplementation"),
            Some(LspOperation::GoToImplementation)
        );
        assert_eq!(
            LspOperation::from_str("prepareCallHierarchy"),
            Some(LspOperation::PrepareCallHierarchy)
        );
        assert_eq!(
            LspOperation::from_str("incomingCalls"),
            Some(LspOperation::IncomingCalls)
        );
        assert_eq!(
            LspOperation::from_str("outgoingCalls"),
            Some(LspOperation::OutgoingCalls)
        );
        assert_eq!(LspOperation::from_str("invalid"), None);
    }

    #[test]
    fn test_lsp_operation_all_names() {
        let names = LspOperation::all_names();
        assert_eq!(names.len(), 9);
        assert!(names.contains(&"goToDefinition"));
        assert!(names.contains(&"findReferences"));
        assert!(names.contains(&"hover"));
    }

    #[test]
    fn test_lsp_params_valid() {
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "filePath": "/path/to/file.rs",
            "line": 10,
            "character": 5
        });

        let params = LspParams::from_value(&args).unwrap();
        assert_eq!(params.operation, LspOperation::GoToDefinition);
        assert_eq!(params.file_path, "/path/to/file.rs");
        assert_eq!(params.line, 10);
        assert_eq!(params.character, 5);
    }

    #[test]
    fn test_lsp_params_missing_operation() {
        let args = serde_json::json!({
            "filePath": "/path/to/file.rs",
            "line": 10,
            "character": 5
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Parse(_)));
    }

    #[test]
    fn test_lsp_params_invalid_operation() {
        let args = serde_json::json!({
            "operation": "invalidOp",
            "filePath": "/path/to/file.rs",
            "line": 10,
            "character": 5
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Parse(_)));
    }

    #[test]
    fn test_lsp_params_missing_file_path() {
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "line": 10,
            "character": 5
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_lsp_params_missing_line() {
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "filePath": "/path/to/file.rs",
            "character": 5
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_lsp_params_missing_character() {
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "filePath": "/path/to/file.rs",
            "line": 10
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_lsp_params_line_zero() {
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "filePath": "/path/to/file.rs",
            "line": 0,
            "character": 5
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Parse(_)));
    }

    #[test]
    fn test_lsp_params_character_zero() {
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "filePath": "/path/to/file.rs",
            "line": 10,
            "character": 0
        });

        let result = LspParams::from_value(&args);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ToolError::Parse(_)));
    }

    #[tokio::test]
    async fn test_lsp_tool_execute_placeholder() {
        let tool = LspTool::new();
        let args = serde_json::json!({
            "operation": "goToDefinition",
            "filePath": "src/main.rs",
            "line": 42,
            "character": 10
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("LSP integration coming soon"));
        assert!(result.output.contains("goToDefinition"));
        assert!(result.output.contains("src/main.rs:42:10"));

        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["operation"], "goToDefinition");
        assert_eq!(metadata["filePath"], "src/main.rs");
        assert_eq!(metadata["line"], 42);
        assert_eq!(metadata["character"], 10);
        assert_eq!(metadata["placeholder"], true);
    }

    #[test]
    fn test_lsp_tool_name() {
        let tool = LspTool::new();
        assert_eq!(tool.name(), "lsp");
    }

    #[test]
    fn test_lsp_tool_default() {
        let tool: LspTool = Default::default();
        assert_eq!(tool.name(), "lsp");
    }

    #[test]
    fn test_lsp_tool_parameters() {
        let tool = LspTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["operation"].is_object());
        assert!(params["properties"]["filePath"].is_object());
        assert!(params["properties"]["line"].is_object());
        assert!(params["properties"]["character"].is_object());

        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("operation")));
        assert!(required.contains(&serde_json::json!("filePath")));
        assert!(required.contains(&serde_json::json!("line")));
        assert!(required.contains(&serde_json::json!("character")));

        let enum_vals = params["properties"]["operation"]["enum"].as_array().unwrap();
        assert_eq!(enum_vals.len(), 9);
    }

    #[tokio::test]
    async fn test_all_operations() {
        let tool = LspTool::new();

        for op_name in LspOperation::all_names() {
            let args = serde_json::json!({
                "operation": op_name,
                "filePath": "test.rs",
                "line": 1,
                "character": 1
            });

            let result = tool.execute(args).await.unwrap();
            assert!(
                result.output.contains(op_name),
                "Output should contain operation name {}",
                op_name
            );
        }
    }
}