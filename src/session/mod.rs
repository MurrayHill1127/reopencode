//! Session management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Session ID
pub type SessionId = String;

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub status: SessionStatus,
    pub message_count: u32,
}

/// Message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: String,
    pub session_id: SessionId,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

/// Session manager
pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    messages: HashMap<SessionId, Vec<SessionMessage>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            messages: HashMap::new(),
        }
    }
    
    /// Create a new session
    pub fn create_session(&mut self, title: Option<String>) -> SessionId {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        
        let session = Session {
            id: id.clone(),
            title: title.unwrap_or_else(|| "New Session".to_string()),
            created_at: now,
            updated_at: now,
            status: SessionStatus::Active,
            message_count: 0,
        };
        
        self.sessions.insert(id.clone(), session);
        self.messages.insert(id.clone(), Vec::new());
        
        id
    }
    
    /// Get session info
    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }
    
    /// Add a message to a session
    pub fn add_message(&mut self, session_id: &str, role: &str, content: &str) -> Option<String> {
        let session = self.sessions.get_mut(session_id)?;
        session.message_count += 1;
        session.updated_at = chrono::Utc::now().timestamp();
        
        let message = SessionMessage {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        
        let message_id = message.id.clone();
        self.messages
            .entry(session_id.to_string())
            .or_insert_with(Vec::new)
            .push(message);
        
        Some(message_id)
    }
    
    /// Get messages from a session
    pub fn get_messages(&self, session_id: &str) -> Option<&Vec<SessionMessage>> {
        self.messages.get(session_id)
    }
    
    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }
    
    /// Delete a session
    pub fn delete_session(&mut self, session_id: &str) -> bool {
        self.sessions.remove(session_id).is_some() && self.messages.remove(session_id).is_some()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_session() {
        let mut manager = SessionManager::new();
        let id = manager.create_session(Some("Test".to_string()));
        
        let session = manager.get_session(&id).unwrap();
        assert_eq!(session.title, "Test");
        assert_eq!(session.status, SessionStatus::Active);
    }
    
    #[test]
    fn test_add_message() {
        let mut manager = SessionManager::new();
        let id = manager.create_session(None);
        
        manager.add_message(&id, "user", "Hello");
        manager.add_message(&id, "assistant", "Hi there!");
        
        let messages = manager.get_messages(&id).unwrap();
        assert_eq!(messages.len(), 2);
    }
}
