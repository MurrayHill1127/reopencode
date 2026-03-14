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
}

impl Session {
    /// Create a new session with default values
    pub fn new(title: Option<String>) -> Self {
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

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: MessageId,
    pub session_id: SessionId,
    pub role: String,
    pub content: String,
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
            created_at: now,
        }
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
        let session = Session::new(Some("Test Session".to_string()));

        assert!(!session.id.is_empty());
        assert_eq!(session.title, "Test Session");
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.message_count, 0);
        assert_eq!(session.metadata, serde_json::json!({}));
    }

    #[test]
    fn test_session_default_title() {
        let session = Session::new(None);
        assert_eq!(session.title, "New Session");
    }

    #[test]
    fn test_session_touch() {
        let mut session = Session::new(None);
        let old_updated = session.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.updated_at > old_updated);
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new(None);
        assert_eq!(session.message_count, 0);

        session.add_message();
        assert_eq!(session.message_count, 1);

        session.add_message();
        assert_eq!(session.message_count, 2);
    }

    #[test]
    fn test_session_set_status() {
        let mut session = Session::new(None);
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
        let session = Session::new(Some("Test".to_string()));
        let json = serde_json::to_string(&session).expect("Failed to serialize");

        let deserialized: Session = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(session.id, deserialized.id);
        assert_eq!(session.title, deserialized.title);
        assert_eq!(session.status, deserialized.status);
    }
}