//! Session domain types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Session ID type alias
pub type SessionId = String;

/// Message ID type alias
pub type MessageId = String;

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum SessionStatus {
    #[default]
    Active,
    Paused,
    Completed,
}

/// Summary statistics for session changes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionSummary {
    /// Total number of lines added
    pub additions: u32,
    /// Total number of lines deleted
    pub deletions: u32,
    /// Total number of files changed
    pub files: u32,
}

/// File-level diff information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// Path to the changed file
    pub path: String,
    /// Number of lines added in this file
    pub additions: u32,
    /// Number of lines deleted in this file
    pub deletions: u32,
}

/// Tracks reverted message state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionRevert {
    /// ID of the message being reverted
    pub message_id: String,
    /// Optional part ID within the message
    pub part_id: Option<String>,
    /// Optional snapshot identifier
    pub snapshot: Option<String>,
    /// Optional list of file diffs
    pub diff: Option<Vec<FileDiff>>,
    /// Optional summary of changes
    pub summary: Option<SessionSummary>,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub message_count: u32,
    pub metadata: serde_json::Value,
    pub parent_id: Option<SessionId>,
    pub archived_at: Option<i64>,
    pub revert: Option<SessionRevert>,
    pub summary: Option<SessionSummary>,
    pub share_url: Option<String>,
}

impl Session {
    /// Create a new session with default values
    pub fn new(title: Option<String>, parent_id: Option<String>) -> Self {
        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        Self {
            id,
            title: title.unwrap_or_else(|| "New Session".to_string()),
            created_at: now,
            updated_at: now,
            status: SessionStatus::Active,
            message_count: 0,
            metadata: serde_json::json!({}),
            parent_id,
            archived_at: None,
            revert: None,
            summary: None,
            share_url: None,
        }
    }

    /// Update the session's updated_at timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Increment message count
    pub fn add_message(&mut self) {
        self.message_count += 1;
        self.touch();
    }

    /// Set session status
    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.touch();
    }
}

/// Tool state for tool message parts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolState {
    pub status: String, // "pending", "running", "completed", "error"
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Message part types for rich message content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessagePart {
    Text {
        text: String,
    },
    Tool {
        tool: String,
        input: serde_json::Value,
        output: Option<String>,
        state: ToolState,
    },
    Patch {
        path: String,
        content: String,
    },
    Compaction {
        auto: bool,
    },
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: MessageId,
    pub session_id: SessionId,
    pub role: String,
    pub content: String, // Keep for backward compatibility
    pub parts: Vec<MessagePart>,
    pub created_at: DateTime<Utc>,
}

impl SessionMessage {
    /// Create a new message
    pub fn new(session_id: SessionId, role: String, content: String) -> Self {
        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();

        Self {
            id,
            session_id,
            role,
            content,
            parts: Vec::new(),
            created_at: now,
        }
    }

    /// Add a message part to the message
    pub fn add_part(&mut self, part: MessagePart) {
        self.parts.push(part);
    }
}

/// Filter for session queries
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    pub status: Option<SessionStatus>,
    pub title_contains: Option<String>,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl SessionFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by status
    pub fn with_status(mut self, status: SessionStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by title substring
    pub fn with_title_contains(mut self, query: String) -> Self {
        self.title_contains = Some(query);
        self
    }

    /// Set limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new(Some("Test Session".to_string()), None);

        assert!(!session.id.is_empty());
        assert_eq!(session.title, "Test Session");
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.message_count, 0);
        assert_eq!(session.metadata, serde_json::json!({}));
        assert!(session.parent_id.is_none());
        assert!(session.archived_at.is_none());
    }

    #[test]
    fn test_session_default_title() {
        let session = Session::new(None, None);
        assert_eq!(session.title, "New Session");
    }

    #[test]
    fn test_session_with_parent() {
        let parent_id = uuid::Uuid::new_v4().to_string();
        let session = Session::new(Some("Child Session".to_string()), Some(parent_id.clone()));
        assert_eq!(session.parent_id, Some(parent_id));
    }

    #[test]
    fn test_session_touch() {
        let mut session = Session::new(None, None);
        let old_updated = session.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.updated_at > old_updated);
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new(None, None);
        assert_eq!(session.message_count, 0);

        session.add_message();
        assert_eq!(session.message_count, 1);

        session.add_message();
        assert_eq!(session.message_count, 2);
    }

    #[test]
    fn test_session_set_status() {
        let mut session = Session::new(None, None);
        assert_eq!(session.status, SessionStatus::Active);

        session.set_status(SessionStatus::Paused);
        assert_eq!(session.status, SessionStatus::Paused);

        session.set_status(SessionStatus::Completed);
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_message_creation() {
        let session_id = uuid::Uuid::new_v4().to_string();
        let message = SessionMessage::new(
            session_id.clone(),
            "user".to_string(),
            "Hello, world!".to_string(),
        );

        assert!(!message.id.is_empty());
        assert_eq!(message.session_id, session_id);
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello, world!");
    }

    #[test]
    fn test_session_filter_builder() {
        let filter = SessionFilter::new()
            .with_status(SessionStatus::Active)
            .with_title_contains("test".to_string())
            .with_limit(10);

        assert_eq!(filter.status, Some(SessionStatus::Active));
        assert_eq!(filter.title_contains, Some("test".to_string()));
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new(Some("Test".to_string()), None);
        let json = serde_json::to_string(&session).expect("Failed to serialize");

        let deserialized: Session = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(session.id, deserialized.id);
        assert_eq!(session.title, deserialized.title);
        assert_eq!(session.status, deserialized.status);
        assert_eq!(session.parent_id, deserialized.parent_id);
    }

    #[test]
    fn test_message_with_parts() {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut message = SessionMessage::new(
            session_id.clone(),
            "assistant".to_string(),
            "Processing your request".to_string(),
        );

        assert!(message.parts.is_empty());

        let text_part = MessagePart::Text {
            text: "Hello, world!".to_string(),
        };
        message.add_part(text_part);

        let tool_part = MessagePart::Tool {
            tool: "file_read".to_string(),
            input: serde_json::json!({"path": "/test/file.txt"}),
            output: Some("File content".to_string()),
            state: ToolState {
                status: "completed".to_string(),
                output: Some("success".to_string()),
                error: None,
            },
        };
        message.add_part(tool_part);

        assert_eq!(message.parts.len(), 2);
        assert!(matches!(message.parts[0], MessagePart::Text { .. }));
        assert!(matches!(message.parts[1], MessagePart::Tool { .. }));
    }

    #[test]
    fn test_message_part_serialization() {
        let text_part = MessagePart::Text {
            text: "Test text".to_string(),
        };
        let json = serde_json::to_string(&text_part).expect("Failed to serialize Text part");
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Test text\""));

        let patch_part = MessagePart::Patch {
            path: "/src/main.rs".to_string(),
            content: "fn main() {}".to_string(),
        };
        let json = serde_json::to_string(&patch_part).expect("Failed to serialize Patch part");
        assert!(json.contains("\"type\":\"patch\""));
        assert!(json.contains("\"path\":\"/src/main.rs\""));

        let compaction_part = MessagePart::Compaction { auto: true };
        let json =
            serde_json::to_string(&compaction_part).expect("Failed to serialize Compaction part");
        assert!(json.contains("\"type\":\"compaction\""));
        assert!(json.contains("\"auto\":true"));

        let tool_part = MessagePart::Tool {
            tool: "test_tool".to_string(),
            input: serde_json::json!({"key": "value"}),
            output: None,
            state: ToolState::default(),
        };
        let json = serde_json::to_string(&tool_part).expect("Failed to serialize Tool part");
        assert!(json.contains("\"type\":\"tool\""));
        assert!(json.contains("\"tool\":\"test_tool\""));
    }

    #[test]
    fn test_message_serialization_with_parts() {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut message = SessionMessage::new(
            session_id.clone(),
            "user".to_string(),
            "Test message".to_string(),
        );

        message.add_part(MessagePart::Text {
            text: "Additional context".to_string(),
        });

        let json = serde_json::to_string(&message).expect("Failed to serialize message");
        let deserialized: SessionMessage =
            serde_json::from_str(&json).expect("Failed to deserialize message");

        assert_eq!(message.id, deserialized.id);
        assert_eq!(message.session_id, deserialized.session_id);
        assert_eq!(message.role, deserialized.role);
        assert_eq!(message.content, deserialized.content);
        assert_eq!(message.parts.len(), deserialized.parts.len());
    }

    #[test]
    fn test_tool_state_default() {
        let state = ToolState::default();
        assert_eq!(state.status, "");
        assert!(state.output.is_none());
        assert!(state.error.is_none());
    }
}
