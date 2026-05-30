use async_trait::async_trait;
use serde_json::Value;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Bash tool - execute shell commands with a 30-second timeout
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
        "Execute a shell command. Has a 30-second timeout. Use for running tests, building, searching, or any shell operation."
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
                    "description": "Working directory (optional)"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Timeout in seconds (default 30, max 120)"
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
        let timeout_secs = args["timeout_secs"]
            .as_u64()
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(120);

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);
        cmd.kill_on_drop(true);

        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }

        let run = async {
            let output = cmd.output().await?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Ok::<_, std::io::Error>((stdout, stderr, output.status.code()))
        };

        match timeout(Duration::from_secs(timeout_secs), run).await {
            Ok(Ok((stdout, stderr, exit_code))) => {
                let result = format!("{}{}", stdout, stderr);
                Ok(ToolResult {
                    output: result,
                    metadata: Some(serde_json::json!({
                        "exit_code": exit_code,
                        "stdout_len": stdout.len(),
                        "stderr_len": stderr.len(),
                    })),
                })
            }
            Ok(Err(e)) => Err(ToolError::Execution(e.to_string())),
            Err(_) => Err(ToolError::Execution(format!(
                "Command timed out after {}s: {}",
                timeout_secs, command
            ))),
        }
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
