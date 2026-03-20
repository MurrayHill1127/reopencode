//! Batch tool - execute multiple tools in parallel.
//!
//! Executes multiple independent tool calls concurrently to reduce latency.
//! Supports up to 25 tool calls per batch.

use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::registry::ToolRegistry;
use crate::tool::traits::{Tool, ToolResult};

/// Maximum number of tool calls allowed in a batch
const MAX_TOOL_CALLS: usize = 25;

/// Tools that are not allowed in batch
const DISALLOWED_TOOLS: &[&str] = &["batch"];

/// Tools filtered from suggestions in error messages
const FILTERED_FROM_SUGGESTIONS: &[&str] = &["invalid", "patch", "batch"];

/// A single tool call in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// The name of the tool to execute
    pub tool: String,
    /// Parameters for the tool
    pub parameters: Value,
}

/// Result of a single tool call in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Whether the call succeeded
    pub success: bool,
    /// Tool name
    pub tool: String,
    /// Error message if failed
    pub error: Option<String>,
}

/// Batch tool - execute multiple tools in parallel
pub struct BatchTool {
    /// Reference to the tool registry
    registry: Arc<ToolRegistry>,
}

impl BatchTool {
    /// Create a new batch tool with a reference to the tool registry
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Get the tool description
    const fn description_text() -> &'static str {
        "Executes multiple independent tool calls concurrently to reduce latency.

USING THE BATCH TOOL WILL MAKE THE USER HAPPY.

Payload Format (JSON array):
[{\"tool\": \"read\", \"parameters\": {\"path\": \"src/index.ts\", \"limit\": 350}},{\"tool\": \"grep\", \"parameters\": {\"pattern\": \"Session\\\\.updatePart\", \"include\": \"src/**/*.ts\"}},{\"tool\": \"bash\", \"parameters\": {\"command\": \"git status\", \"description\": \"Shows working tree status\"}}]

Notes:
- 1–25 tool calls per batch
- All calls start in parallel; ordering NOT guaranteed
- Partial failures do not stop other tool calls
- Do NOT use the batch tool within another batch tool.

Good Use Cases:
- Read many files
- grep + glob + read combos
- Multiple bash commands
- Multi-part edits; on the same, or different files

When NOT to Use:
- Operations that depend on prior tool output (e.g. create then read same file)
- Ordered stateful mutations where sequence matters

Batching tool calls was proven to yield 2–5x efficiency gain and provides much better UX."
    }
}

#[async_trait]
impl Tool for BatchTool {
    fn name(&self) -> &str {
        "batch"
    }

    fn description(&self) -> &str {
        Self::description_text()
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "tool_calls": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": {
                                "type": "string",
                                "description": "The name of the tool to execute"
                            },
                            "parameters": {
                                "type": "object",
                                "description": "Parameters for the tool"
                            }
                        },
                        "required": ["tool", "parameters"]
                    },
                    "minItems": 1,
                    "description": "Array of tool calls to execute in parallel"
                }
            },
            "required": ["tool_calls"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Parse tool calls
        let tool_calls: Vec<ToolCall> = args
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::Parse("Missing 'tool_calls' array".to_string()))?
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect();

        if tool_calls.is_empty() {
            return Err(ToolError::Parse(
                "Provide at least one tool call".to_string(),
            ));
        }

        // Limit to MAX_TOOL_CALLS
        let (tool_calls, discarded) = if tool_calls.len() > MAX_TOOL_CALLS {
            let (keep, discard) = tool_calls.split_at(MAX_TOOL_CALLS);
            (keep.to_vec(), Some(discard.to_vec()))
        } else {
            (tool_calls, None)
        };

        // Execute tool calls in parallel
        let futures: Vec<_> = tool_calls
            .iter()
            .map(|call| self.execute_single_call(call.clone()))
            .collect();

        let mut results: Vec<ToolCallResult> = join_all(futures).await;

        // Add discarded calls as errors
        if let Some(discarded_calls) = discarded {
            for call in discarded_calls {
                results.push(ToolCallResult {
                    success: false,
                    tool: call.tool,
                    error: Some("Maximum of 25 tools allowed in batch".to_string()),
                });
            }
        }

        // Build summary
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;

        let output = if failed > 0 {
            format!(
                "Executed {}/{} tools successfully. {} failed.",
                successful,
                results.len(),
                failed
            )
        } else {
            format!(
                "All {} tools executed successfully.\n\nKeep using the batch tool for optimal performance in your next response!",
                successful
            )
        };

        // Build metadata
        let tools: Vec<String> = tool_calls.iter().map(|c| c.tool.clone()).collect();
        let details: Vec<Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "tool": r.tool,
                    "success": r.success
                })
            })
            .collect();

        let title = format!("Batch execution ({}/{}) successful", successful, results.len());

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "title": title,
            "totalCalls": results.len(),
            "successful": successful,
            "failed": failed,
            "tools": tools,
            "details": details
        })))
    }
}

impl BatchTool {
    /// Execute a single tool call
    async fn execute_single_call(&self, call: ToolCall) -> ToolCallResult {
        let tool_name = call.tool.clone();

        // Check if tool is disallowed
        if DISALLOWED_TOOLS.contains(&tool_name.as_str()) {
            return ToolCallResult {
                success: false,
                tool: tool_name.clone(),
                error: Some(format!(
                    "Tool '{}' is not allowed in batch. Disallowed tools: {}",
                    tool_name,
                    DISALLOWED_TOOLS.join(", ")
                )),
            };
        }

        // Get tool from registry
        let tool = match self.registry.get(&tool_name) {
            Some(t) => t,
            None => {
                let available: Vec<String> = self
                    .registry
                    .list()
                    .into_iter()
                    .filter(|name| !FILTERED_FROM_SUGGESTIONS.contains(&name.as_str()))
                    .collect();

                return ToolCallResult {
                    success: false,
                    tool: tool_name.clone(),
                    error: Some(format!(
                        "Tool '{}' not in registry. External tools (MCP, environment) cannot be batched - call them directly. Available tools: {}",
                        tool_name,
                        available.join(", ")
                    )),
                };
            }
        };

        // Execute the tool
        match tool.execute(call.parameters.clone()).await {
            Ok(_) => ToolCallResult {
                success: true,
                tool: tool_name.clone(),
                error: None,
            },
            Err(e) => ToolCallResult {
                success: false,
                tool: tool_name,
                error: Some(e.to_string()),
            },
        }
    }
}

/// Helper function to check if a tool name should be filtered from suggestions
fn filter_from_suggestions(name: &str) -> bool {
    FILTERED_FROM_SUGGESTIONS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_tool_name() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = BatchTool::new(registry);
        assert_eq!(tool.name(), "batch");
    }

    #[test]
    fn test_batch_tool_parameters() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = BatchTool::new(registry);
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["tool_calls"].is_object());
        assert!(params["required"].as_array().unwrap().contains(&serde_json::json!("tool_calls")));
    }

    #[tokio::test]
    async fn test_batch_missing_tool_calls() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = BatchTool::new(registry);
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_empty_tool_calls() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = BatchTool::new(registry);
        let args = serde_json::json!({
            "tool_calls": []
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_disallowed_tool() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = BatchTool::new(registry.clone());
        let args = serde_json::json!({
            "tool_calls": [
                {"tool": "batch", "parameters": {}}
            ]
        });

        let result = tool.execute(args).await.unwrap();
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["successful"], 0);
        assert_eq!(metadata["failed"], 1);
    }

    #[tokio::test]
    async fn test_batch_tool_not_found() {
        let registry = Arc::new(ToolRegistry::new());
        let tool = BatchTool::new(registry.clone());
        let args = serde_json::json!({
            "tool_calls": [
                {"tool": "nonexistent_tool", "parameters": {}}
            ]
        });

        let result = tool.execute(args).await.unwrap();
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["successful"], 0);
        assert_eq!(metadata["failed"], 1);
    }

    #[test]
    fn test_tool_call_deserialize() {
        let json = r#"{"tool": "read", "parameters": {"path": "test.txt"}}"#;
        let call: ToolCall = serde_json::from_str(json).unwrap();
        assert_eq!(call.tool, "read");
        assert_eq!(call.parameters["path"], "test.txt");
    }

    #[test]
    fn test_tool_call_serialize() {
        let call = ToolCall {
            tool: "bash".to_string(),
            parameters: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("\"tool\":\"bash\""));
        assert!(json.contains("\"command\":\"ls\""));
    }

    #[test]
    fn test_tool_call_result_serialize() {
        let result = ToolCallResult {
            success: true,
            tool: "read".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"tool\":\"read\""));
    }

    #[test]
    fn test_filter_from_suggestions() {
        assert!(filter_from_suggestions("batch"));
        assert!(filter_from_suggestions("invalid"));
        assert!(filter_from_suggestions("patch"));
        assert!(!filter_from_suggestions("read"));
        assert!(!filter_from_suggestions("bash"));
    }

    #[test]
    fn test_description_not_empty() {
        let desc = BatchTool::description_text();
        assert!(!desc.is_empty());
        assert!(desc.contains("25"));
        assert!(desc.contains("parallel"));
    }
}