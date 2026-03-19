//! Database schema definitions
//!
//! Defines all record types for sessions, messages, parts, todos, and projects.
//! Corresponds to TypeScript schemas in session/session.sql.ts and project/project.sql.ts

use serde::{Deserialize, Serialize};

// ==================== Session ====================

/// Session record
/// Corresponds to TypeScript: SessionTable (session/session.sql.ts:14-44)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRecord {
    pub id: String,
    pub project_id: String,
    pub workspace_id: Option<String>,
    pub parent_id: Option<String>,
    pub slug: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub share_url: Option<String>,
    pub summary_additions: Option<i32>,
    pub summary_deletions: Option<i32>,
    pub summary_files: Option<i32>,
    pub summary_diffs: Option<serde_json::Value>,
    pub revert: Option<serde_json::Value>,
    pub permission: Option<serde_json::Value>,
    pub time_created: i64,
    pub time_updated: i64,
    pub time_compacting: Option<i64>,
    pub time_archived: Option<i64>,
}

impl SessionRecord {
    pub fn new(id: String, project_id: String, directory: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id,
            project_id,
            directory,
            title: "New Session".to_string(),
            version: "0.1.0".to_string(),
            time_created: now,
            time_updated: now,
            slug: crate::util::generate_uuid()[..8].to_string(),
            ..Default::default()
        }
    }
}

// ==================== Message ====================

/// Message record
/// Corresponds to TypeScript: MessageTable (session/session.sql.ts:46-58)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: serde_json::Value,
}

/// Message part record
/// Corresponds to TypeScript: PartTable (session/session.sql.ts:60-76)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartRecord {
    pub id: String,
    pub message_id: String,
    pub session_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: serde_json::Value,
}

/// Message role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    #[default]
    User,
    Assistant,
}

/// Message time info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTime {
    pub created: i64,
    pub updated: Option<i64>,
}

/// Message info (without parts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub agent: Option<String>,
    pub time: MessageTime,
}

/// Image source for message content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Message part types
/// Corresponds to TypeScript: MessageV2.Part
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePart {
    Text {
        id: String,
        text: String,
    },
    Thinking {
        id: String,
        thinking: String,
    },
    ToolUse {
        id: String,
        tool: String,
        call_id: String,
        input: serde_json::Value,
    },
    ToolResult {
        id: String,
        tool_call_id: String,
        content: String,
        is_error: bool,
    },
    Image {
        id: String,
        source: ImageSource,
    },
}

impl MessagePart {
    pub fn id(&self) -> &str {
        match self {
            MessagePart::Text { id, .. } => id,
            MessagePart::Thinking { id, .. } => id,
            MessagePart::ToolUse { id, .. } => id,
            MessagePart::ToolResult { id, .. } => id,
            MessagePart::Image { id, .. } => id,
        }
    }
}

/// Message with parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageWithParts {
    pub info: MessageInfo,
    pub parts: Vec<MessagePart>,
}

// ==================== Todo ====================

/// Todo record
/// Corresponds to TypeScript: TodoTable (session/session.sql.ts:78-95)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoRecord {
    pub session_id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
    pub position: i32,
    pub time_created: i64,
    pub time_updated: i64,
}

/// Todo status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TodoStatus {
    #[default]
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "pending"),
            TodoStatus::InProgress => write!(f, "in_progress"),
            TodoStatus::Completed => write!(f, "completed"),
            TodoStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Todo priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TodoPriority {
    #[default]
    Medium,
    High,
    Low,
}

impl std::fmt::Display for TodoPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoPriority::High => write!(f, "high"),
            TodoPriority::Medium => write!(f, "medium"),
            TodoPriority::Low => write!(f, "low"),
        }
    }
}

/// Todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub position: i32,
}

impl TodoItem {
    pub fn new(content: String, position: i32) -> Self {
        Self {
            id: crate::util::generate_uuid(),
            content,
            status: TodoStatus::Pending,
            priority: TodoPriority::Medium,
            position,
        }
    }
}

// ==================== Project ====================

/// Project record
/// Corresponds to TypeScript: ProjectTable (project/project.sql.ts)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectRecord {
    pub id: String,
    pub vcs: Option<String>,
    pub worktree: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub time_initialized: Option<i64>,
}

impl ProjectRecord {
    pub fn new(worktree: String) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: format!(
                "proj_{}",
                &crate::util::generate_uuid().replace('-', "")[..12]
            ),
            worktree,
            time_created: now,
            time_updated: now,
            ..Default::default()
        }
    }
}

/// Permission record
/// Corresponds to TypeScript: PermissionTable (session/session.sql.ts:97-103)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRecord {
    pub project_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: serde_json::Value,
}

/// Workspace record
/// Corresponds to TypeScript: WorkspaceTable (control-plane/workspace.sql.ts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRecord {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub directory: String,
    pub time_created: i64,
    pub time_updated: i64,
}

// ==================== Timestamps ====================

/// Timestamp fields mixin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    pub time_created: i64,
    pub time_updated: i64,
}

impl Default for Timestamps {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            time_created: now,
            time_updated: now,
        }
    }
}

impl Timestamps {
    pub fn touch(&mut self) {
        self.time_updated = chrono::Utc::now().timestamp_millis();
    }
}

// ==================== Pagination ====================

/// Paginated result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub cursor: Option<String>,
}

impl<T> Page<T> {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            cursor: None,
        }
    }

    pub fn new(items: Vec<T>, cursor: Option<String>) -> Self {
        Self { items, cursor }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_record_new() {
        let session = SessionRecord::new(
            "ses_123".to_string(),
            "proj_abc".to_string(),
            "/home/user/project".to_string(),
        );
        assert_eq!(session.id, "ses_123");
        assert_eq!(session.project_id, "proj_abc");
        assert_eq!(session.title, "New Session");
        assert!(session.time_created > 0);
    }

    #[test]
    fn test_message_part_id() {
        let part = MessagePart::Text {
            id: "part_123".to_string(),
            text: "Hello".to_string(),
        };
        assert_eq!(part.id(), "part_123");
    }

    #[test]
    fn test_todo_status_display() {
        assert_eq!(TodoStatus::Pending.to_string(), "pending");
        assert_eq!(TodoStatus::InProgress.to_string(), "in_progress");
        assert_eq!(TodoStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn test_todo_item_new() {
        let todo = TodoItem::new("Test task".to_string(), 1);
        assert_eq!(todo.content, "Test task");
        assert_eq!(todo.position, 1);
        assert_eq!(todo.status, TodoStatus::Pending);
        assert!(todo.id.starts_with("part_") || todo.id.contains('-'));
    }

    #[test]
    fn test_project_record_new() {
        let project = ProjectRecord::new("/home/user/project".to_string());
        assert!(project.id.starts_with("proj_"));
        assert_eq!(project.worktree, "/home/user/project");
        assert!(project.time_created > 0);
    }

    #[test]
    fn test_timestamps_default() {
        let ts = Timestamps::default();
        assert!(ts.time_created > 0);
        assert!(ts.time_updated > 0);
    }

    #[test]
    fn test_page_empty() {
        let page: Page<String> = Page::empty();
        assert!(page.items.is_empty());
        assert!(page.cursor.is_none());
    }

    #[test]
    fn test_message_role_serialization() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");

        let role: MessageRole = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, MessageRole::Assistant);
    }

    #[test]
    fn test_message_part_serialization() {
        let part = MessagePart::Text {
            id: "p1".to_string(),
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"id\":\"p1\""));
    }
}
