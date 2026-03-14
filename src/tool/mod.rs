//! Tool system - tools that agents can use

use async_trait::async_trait;
use serde_json::Value;

/// Tool result
#[derive(Debug)]
pub struct ToolResult {
    pub output: String,
    pub metadata: Option<Value>,
}

/// Tool error
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Execution error: {0}")]
    Execution(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {0}")]
    Parse(String),
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
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError>;
}

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
    
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'command' argument".to_string()))?;
        
        let cwd = args["cwd"].as_str();
        
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);
        
        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }
        
        let output = cmd.output().await?;
        
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
        
        let content = tokio::fs::read_to_string(path).await?;
        
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
    
    async fn execute(&self, args: Value) -> Result<ToolResult, ToolError> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'path' argument".to_string()))?;
        
        let content = args["content"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'content' argument".to_string()))?;
        
        tokio::fs::write(path, content).await?;
        
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
