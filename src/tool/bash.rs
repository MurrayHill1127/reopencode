 async_trait::async_trait;
use serde_json::Value;
use std::process::Command;

use crate::tool::error::{ToolError, Result};
use crate::tool::traits::{Tool, ToolResult};

/// Bash tool - execute shell commands
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "cwd": {
                    "type": "string",
                    "description": "Working directory"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'command' argument".to_string()))?;

        let cwd = args["cwd"].as_str();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }

        let output = cmd.output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = format!("{}{}", stdout, stderr);

        Ok(ToolResult {
            output: result,
            metadata: Some(serde_json::json!({
                "exit_code": output.status.code(),
                "stdout_len": stdout.len(),
                "stderr_len": stderr.len(),
            })),
        })
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_echo_command() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "echo hello"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_bash_failing_command() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "exit 1"
        });

        let result = tool.execute(args).await.unwrap();
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["exit_code"], 1);
    }

    #[tokio::test]
    async fn test_bash_with_cwd() {
        let tool = BashTool::new();
        let args = serde_json::json!({
            "command": "pwd",
            "cwd": "/tmp"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("/tmp"));
    }

    #[tokio::test]
    async fn test_bash_missing_command_arg() {
        let tool = BashTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ToolError::Parse(_)));
    }
}