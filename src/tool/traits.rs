//! Tool traits - Tool trait and ToolResult

use async_trait::async_trait;
use serde_json::Value;

use crate::agent::ToolDefinition;
use crate::tool::error::Result;

/// Tool result
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub metadata: Option<Value>,
}

impl ToolResult {
    pub fn new(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Tool trait - all tools must implement this
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get tool name
    fn name(&self) -> &str;

    /// Get tool description
    fn description(&self) -> &str;

    /// Get tool parameters (JSON Schema)
    fn parameters(&self) -> Value;

    /// Execute the tool with arguments
    async fn execute(&self, args: Value) -> Result<ToolResult>;

    fn to_agent_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_creation() {
        let result = ToolResult::new("test output");
        assert_eq!(result.output, "test output");
        assert!(result.metadata.is_none());
    }

    #[test]
    fn test_tool_result_with_metadata() {
        let metadata = serde_json::json!({
            "exit_code": 0,
            "duration_ms": 100
        });
        let result = ToolResult::new("test output").with_metadata(metadata.clone());
        assert_eq!(result.output, "test output");
        assert_eq!(result.metadata, Some(metadata));
    }
}