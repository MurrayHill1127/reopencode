//! Todo tools - manage task lists for coding sessions

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Todo status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl Default for TodoStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Todo priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TodoPriority {
    High,
    Medium,
    Low,
}

impl Default for TodoPriority {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for TodoPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

/// A single todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Brief description of the task
    pub content: String,
    /// Current status: pending, in_progress, completed, cancelled
    pub status: TodoStatus,
    /// Priority level: high, medium, low
    pub priority: TodoPriority,
}

impl TodoItem {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            status: TodoStatus::default(),
            priority: TodoPriority::default(),
        }
    }

    pub fn with_status(mut self, status: TodoStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_priority(mut self, priority: TodoPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Session ID type placeholder - will be replaced with actual session module type
pub type SessionID = String;

/// In-memory todo storage (will be replaced with session integration)
static TODO_STORE: LazyLock<RwLock<HashMap<SessionID, Vec<TodoItem>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Get todos for a session
fn get_todos(session_id: &str) -> Vec<TodoItem> {
    let store = TODO_STORE.read().unwrap();
    store.get(session_id).cloned().unwrap_or_default()
}

/// Update todos for a session
fn update_todos(session_id: &str, todos: Vec<TodoItem>) {
    let mut store = TODO_STORE.write().unwrap();
    if todos.is_empty() {
        store.remove(session_id);
    } else {
        store.insert(session_id.to_string(), todos);
    }
}

/// Default session ID for tools without session context
const DEFAULT_SESSION: &str = "default";

/// TodoWrite tool - update todo list
pub struct TodoWriteTool;

impl TodoWriteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "todowrite"
    }

    fn description(&self) -> &str {
        r#"Use this tool to create and manage a structured task list for your current coding session. This helps you track progress, organize complex tasks, and demonstrate thoroughness to the user.
It also helps the user understand the progress of the task and overall progress of their requests.

## When to Use This Tool
Use this tool proactively in these scenarios:

1. Complex multistep tasks - When a task requires 3 or more distinct steps or actions
2. Non-trivial and complex tasks - Tasks that require careful planning or multiple operations
3. User explicitly requests todo list - When the user directly asks you to use the todo list
4. User provides multiple tasks - When users provide a list of things to be done (numbered or comma-separated)
5. After receiving new instructions - Immediately capture user requirements as todos. Feel free to edit the todo list based on new information.
6. After completing a task - Mark it complete and add any new follow-up tasks
7. When you start working on a new task, mark the todo as in_progress. Ideally you should only have one todo as in_progress at a time. Complete existing tasks before starting new ones.

## When NOT to Use This Tool
Skip using this tool when:
1. There is only a single, straightforward task
2. The task is trivial and tracking it provides no organizational benefit
3. The task can be completed in less than 3 trivial steps
4. The task is purely conversational or informational

## Task States
- pending: Task not yet started
- in_progress: Currently working on (limit to ONE task at a time)
- completed: Task finished successfully
- cancelled: Task no longer needed"#
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "The updated todo list",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "Brief description of the task"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed", "cancelled"],
                                "description": "Current status of the task"
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["high", "medium", "low"],
                                "description": "Priority level of the task"
                            }
                        },
                        "required": ["content", "status", "priority"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let todos: Vec<TodoItem> = serde_json::from_value(
            args.get("todos")
                .ok_or_else(|| ToolError::Parse("Missing 'todos' argument".to_string()))?
                .clone(),
        )
        .map_err(|e| ToolError::Parse(format!("Invalid todos format: {}", e)))?;

        // Validate: only one in_progress allowed
        let in_progress_count = todos
            .iter()
            .filter(|t| t.status == TodoStatus::InProgress)
            .count();
        if in_progress_count > 1 {
            return Err(ToolError::Execution(
                "Only one todo can be in_progress at a time".to_string(),
            ));
        }

        // TODO: Integrate with permission system
        // await ctx.ask({ permission: "todowrite", patterns: ["*"], always: ["*"], metadata: {} })

        update_todos(DEFAULT_SESSION, todos.clone());

        let active_count = todos
            .iter()
            .filter(|t| t.status != TodoStatus::Completed)
            .count();

        let output = serde_json::to_string_pretty(&todos)
            .map_err(|e| ToolError::Execution(format!("Failed to serialize todos: {}", e)))?;

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "todos": todos,
            "active_count": active_count
        })))
    }
}

/// TodoRead tool - read current todo list
pub struct TodoReadTool;

impl TodoReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TodoReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for TodoReadTool {
    fn name(&self) -> &str {
        "todoread"
    }

    fn description(&self) -> &str {
        "Use this tool to read your todo list"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult> {
        // TODO: Integrate with permission system
        // await ctx.ask({ permission: "todoread", patterns: ["*"], always: ["*"], metadata: {} })

        let todos = get_todos(DEFAULT_SESSION);

        let active_count = todos
            .iter()
            .filter(|t| t.status != TodoStatus::Completed)
            .count();

        let output = serde_json::to_string_pretty(&todos)
            .map_err(|e| ToolError::Execution(format!("Failed to serialize todos: {}", e)))?;

        Ok(ToolResult::new(output).with_metadata(serde_json::json!({
            "todos": todos,
            "active_count": active_count
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn unique_session_id() -> String {
        format!("test_{}", TEST_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    #[test]
    fn test_todowrite_tool_name() {
        let tool = TodoWriteTool::new();
        assert_eq!(tool.name(), "todowrite");
    }

    #[test]
    fn test_todoread_tool_name() {
        let tool = TodoReadTool::new();
        assert_eq!(tool.name(), "todoread");
    }

    #[test]
    fn test_todo_item_new() {
        let item = TodoItem::new("Test task");
        assert_eq!(item.content, "Test task");
        assert_eq!(item.status, TodoStatus::Pending);
        assert_eq!(item.priority, TodoPriority::Medium);
    }

    #[test]
    fn test_todo_item_builders() {
        let item = TodoItem::new("Test task")
            .with_status(TodoStatus::InProgress)
            .with_priority(TodoPriority::High);
        assert_eq!(item.status, TodoStatus::InProgress);
        assert_eq!(item.priority, TodoPriority::High);
    }

    #[tokio::test]
    async fn test_todowrite_single_todo() {
        let session = unique_session_id();
        let tool = TodoWriteTool::new();
        let args = serde_json::json!({
            "todos": [{
                "content": "Implement feature X",
                "status": "pending",
                "priority": "high"
            }]
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Implement feature X"));

        update_todos(&session, serde_json::from_value(serde_json::json!([{
            "content": "Implement feature X",
            "status": "pending",
            "priority": "high"
        }])).unwrap());

        let stored = get_todos(&session);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].content, "Implement feature X");
    }

    #[tokio::test]
    async fn test_todowrite_multiple_todos() {
        let tool = TodoWriteTool::new();
        let args = serde_json::json!({
            "todos": [
                {"content": "Task 1", "status": "completed", "priority": "high"},
                {"content": "Task 2", "status": "in_progress", "priority": "medium"},
                {"content": "Task 3", "status": "pending", "priority": "low"}
            ]
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.output.contains("Task 1"));
        assert!(result.output.contains("Task 2"));
        assert!(result.output.contains("Task 3"));

        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["active_count"], 2);
    }

    #[tokio::test]
    async fn test_todowrite_only_one_in_progress() {
        let tool = TodoWriteTool::new();
        let args = serde_json::json!({
            "todos": [
                {"content": "Task 1", "status": "in_progress", "priority": "high"},
                {"content": "Task 2", "status": "in_progress", "priority": "medium"}
            ]
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.to_lowercase().contains("only one"),
            "Error message was: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_todowrite_missing_todos() {
        let tool = TodoWriteTool::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_todoread_empty() {
        let session = unique_session_id();
        let stored = get_todos(&session);
        assert!(stored.is_empty());
    }

    #[tokio::test]
    async fn test_todoread_after_write() {
        let session = unique_session_id();

        update_todos(&session, vec![
            TodoItem {
                content: "Task A".to_string(),
                status: TodoStatus::Pending,
                priority: TodoPriority::High,
            },
            TodoItem {
                content: "Task B".to_string(),
                status: TodoStatus::Completed,
                priority: TodoPriority::Low,
            },
        ]);

        let stored = get_todos(&session);
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].content, "Task A");
        assert_eq!(stored[1].content, "Task B");
    }

    #[tokio::test]
    async fn test_todowrite_empty_clears() {
        let session = unique_session_id();

        update_todos(&session, vec![TodoItem::new("Task")]);
        assert!(!get_todos(&session).is_empty());

        update_todos(&session, vec![]);
        assert!(get_todos(&session).is_empty());
    }

    #[test]
    fn test_todo_status_display() {
        assert_eq!(TodoStatus::Pending.to_string(), "pending");
        assert_eq!(TodoStatus::InProgress.to_string(), "in_progress");
        assert_eq!(TodoStatus::Completed.to_string(), "completed");
        assert_eq!(TodoStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn test_todo_priority_display() {
        assert_eq!(TodoPriority::High.to_string(), "high");
        assert_eq!(TodoPriority::Medium.to_string(), "medium");
        assert_eq!(TodoPriority::Low.to_string(), "low");
    }
}