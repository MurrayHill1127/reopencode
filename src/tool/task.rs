//! Task tool - delegate tasks to specialized agents
//!
//! This is a placeholder implementation. Full implementation requires
//! session module integration for agent spawning and communication.

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Task tool - delegate tasks to specialized agents
pub struct TaskTool;

impl TaskTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &str {
        "task"
    }

    fn description(&self) -> &str {
        r#"Launch a new agent to handle complex, multistep tasks autonomously.

When using the Task tool, you must specify a subagent_type parameter to select which agent type to use.

When to use the Task tool:
- When you are instructed to execute custom slash commands
- For complex multi-step tasks that can be handled autonomously
- When you need specialized agent capabilities

When NOT to use the Task tool:
- If you want to read a specific file path, use the Read or Glob tool instead
- If you are searching for a specific class definition, use the Glob tool instead
- If you are searching for code within a specific file, use the Read tool instead

Usage notes:
1. Launch multiple agents concurrently whenever possible, to maximize performance
2. Each agent invocation starts with a fresh context unless you provide task_id to resume
3. The agent's outputs should generally be trusted
4. Clearly tell the agent whether you expect it to write code or just to do research"#
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "A short (3-5 words) description of the task"
                },
                "prompt": {
                    "type": "string",
                    "description": "The task for the agent to perform"
                },
                "subagent_type": {
                    "type": "string",
                    "description": "The type of specialized agent to use for this task"
                },
                "task_id": {
                    "type": "string",
                    "description": "This should only be set if you mean to resume a previous task (you can pass a prior task_id and the task will continue the same subagent session as before instead of creating a fresh one)"
                },
                "command": {
                    "type": "string",
                    "description": "The command that triggered this task"
                }
            },
            "required": ["description", "prompt", "subagent_type"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        // Parse required parameters
        let description = args["description"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'description' argument".to_string()))?;

        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'prompt' argument".to_string()))?;

        let subagent_type = args["subagent_type"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'subagent_type' argument".to_string()))?;

        // Optional parameters
        let task_id = args["task_id"].as_str();
        let command = args["command"].as_str();

        // Placeholder implementation - session module integration required
        let output = if task_id.is_some() {
            format!(
                "Task delegation requires session module integration.\n\
                 \n\
                 Task ID: {} (for resuming)\n\
                 Description: {}\n\
                 Subagent Type: {}\n\
                 Command: {}\n\
                 Prompt: {}",
                task_id.unwrap(),
                description,
                subagent_type,
                command.unwrap_or("N/A"),
                prompt
            )
        } else {
            format!(
                "Task delegation requires session module integration.\n\
                 \n\
                 Description: {}\n\
                 Subagent Type: {}\n\
                 Command: {}\n\
                 Prompt: {}",
                description,
                subagent_type,
                command.unwrap_or("N/A"),
                prompt
            )
        };

        Ok(ToolResult {
            output,
            metadata: Some(serde_json::json!({
                "description": description,
                "subagent_type": subagent_type,
                "task_id": task_id,
                "command": command,
                "placeholder": true
            })),
        })
    }
}

impl Default for TaskTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_tool_new() {
        let tool = TaskTool::new();
        assert_eq!(tool.name(), "task");
    }

    #[test]
    fn test_task_tool_default() {
        let tool: TaskTool = Default::default();
        assert_eq!(tool.name(), "task");
    }

    #[test]
    fn test_task_tool_parameters() {
        let tool = TaskTool::new();
        let params = tool.parameters();

        assert_eq!(params["type"], "object");
        assert!(params["properties"]["description"].is_object());
        assert!(params["properties"]["prompt"].is_object());
        assert!(params["properties"]["subagent_type"].is_object());
        assert!(params["properties"]["task_id"].is_object());
        assert!(params["properties"]["command"].is_object());

        let required = params["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("description")));
        assert!(required.contains(&serde_json::json!("prompt")));
        assert!(required.contains(&serde_json::json!("subagent_type")));
    }

    #[tokio::test]
    async fn test_task_execute_with_required_params() {
        let tool = TaskTool::new();
        let args = serde_json::json!({
            "description": "Test task",
            "prompt": "Do something",
            "subagent_type": "build"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Test task"));
        assert!(result.output.contains("Do something"));
        assert!(result.output.contains("build"));
        assert!(result.metadata.is_some());
    }

    #[tokio::test]
    async fn test_task_execute_with_all_params() {
        let tool = TaskTool::new();
        let args = serde_json::json!({
            "description": "Test task",
            "prompt": "Do something",
            "subagent_type": "build",
            "task_id": "task_123",
            "command": "/test"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("task_123"));
        assert!(result.output.contains("/test"));
    }

    #[tokio::test]
    async fn test_task_execute_missing_description() {
        let tool = TaskTool::new();
        let args = serde_json::json!({
            "prompt": "Do something",
            "subagent_type": "build"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }

    #[tokio::test]
    async fn test_task_execute_missing_prompt() {
        let tool = TaskTool::new();
        let args = serde_json::json!({
            "description": "Test task",
            "subagent_type": "build"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }

    #[tokio::test]
    async fn test_task_execute_missing_subagent_type() {
        let tool = TaskTool::new();
        let args = serde_json::json!({
            "description": "Test task",
            "prompt": "Do something"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }
}