//! Session status tracking (in-memory, matches TypeScript SessionStatus)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

/// Session status info - matches TypeScript union type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum SessionStatusInfo {
    #[serde(rename = "idle")]
    Idle,
    #[serde(rename = "busy")]
    Busy,
    #[serde(rename = "retry")]
    Retry {
        attempt: i32,
        message: String,
        next: i64,
    },
}

impl Default for SessionStatusInfo {
    fn default() -> Self {
        SessionStatusInfo::Idle
    }
}

/// Event published when status changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEvent {
    pub session_id: String,
    pub status: SessionStatusInfo,
}

/// In-memory session status state
pub struct SessionStatusState {
    data: RwLock<HashMap<String, SessionStatusInfo>>,
    event_tx: broadcast::Sender<StatusEvent>,
}

impl SessionStatusState {
    /// Create new status state
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            data: RwLock::new(HashMap::new()),
            event_tx,
        }
    }

    /// Get status for a session (returns idle if not set)
    pub fn get(&self, session_id: &str) -> SessionStatusInfo {
        self.data
            .read()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// List all statuses
    pub fn list(&self) -> HashMap<String, SessionStatusInfo> {
        self.data.read().unwrap().clone()
    }

    /// Set status for a session
    pub fn set(&self, session_id: &str, status: SessionStatusInfo) {
        // Publish event before updating state
        let _ = self.event_tx.send(StatusEvent {
            session_id: session_id.to_string(),
            status: status.clone(),
        });

        let mut data = self.data.write().unwrap();
        if status == SessionStatusInfo::Idle {
            data.remove(session_id);
        } else {
            data.insert(session_id.to_string(), status);
        }
    }

    /// Subscribe to status change events
    pub fn subscribe(&self) -> broadcast::Receiver<StatusEvent> {
        self.event_tx.subscribe()
    }
}

impl Default for SessionStatusState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global session status state
static STATUS_STATE: std::sync::OnceLock<Arc<SessionStatusState>> = std::sync::OnceLock::new();

/// Get or create the global status state
pub fn global_status() -> Arc<SessionStatusState> {
    STATUS_STATE
        .get_or_init(|| Arc::new(SessionStatusState::new()))
        .clone()
}

/// Convenience functions matching TypeScript API
pub fn get(session_id: &str) -> SessionStatusInfo {
    global_status().get(session_id)
}

pub fn list() -> HashMap<String, SessionStatusInfo> {
    global_status().list()
}

pub fn set(session_id: &str, status: SessionStatusInfo) {
    global_status().set(session_id, status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_default_is_idle() {
        let status: SessionStatusInfo = SessionStatusInfo::default();
        assert_eq!(status, SessionStatusInfo::Idle);
    }

    #[test]
    fn test_status_serialization() {
        // Idle
        let idle = SessionStatusInfo::Idle;
        let json = serde_json::to_string(&idle).unwrap();
        assert_eq!(json, r#"{"type":"idle"}"#);

        // Busy
        let busy = SessionStatusInfo::Busy;
        let json = serde_json::to_string(&busy).unwrap();
        assert_eq!(json, r#"{"type":"busy"}"#);

        // Retry
        let retry = SessionStatusInfo::Retry {
            attempt: 3,
            message: "Connection failed".to_string(),
            next: 1234567890,
        };
        let json = serde_json::to_string(&retry).unwrap();
        assert!(json.contains(r#""type":"retry""#));
        assert!(json.contains(r#""attempt":3"#));
        assert!(json.contains(r#""message":"Connection failed""#));
        assert!(json.contains(r#""next":1234567890"#));
    }

    #[test]
    fn test_status_deserialization() {
        // Idle
        let idle: SessionStatusInfo = serde_json::from_str(r#"{"type":"idle"}"#).unwrap();
        assert_eq!(idle, SessionStatusInfo::Idle);

        // Busy
        let busy: SessionStatusInfo = serde_json::from_str(r#"{"type":"busy"}"#).unwrap();
        assert_eq!(busy, SessionStatusInfo::Busy);

        // Retry
        let retry: SessionStatusInfo =
            serde_json::from_str(r#"{"type":"retry","attempt":2,"message":"Error","next":100}"#)
                .unwrap();
        match retry {
            SessionStatusInfo::Retry {
                attempt,
                message,
                next,
            } => {
                assert_eq!(attempt, 2);
                assert_eq!(message, "Error");
                assert_eq!(next, 100);
            }
            _ => panic!("Expected retry variant"),
        }
    }

    #[test]
    fn test_state_get_returns_idle_for_missing() {
        let state = SessionStatusState::new();
        let status = state.get("nonexistent");
        assert_eq!(status, SessionStatusInfo::Idle);
    }

    #[test]
    fn test_state_set_and_get() {
        let state = SessionStatusState::new();

        state.set("session-1", SessionStatusInfo::Busy);
        assert_eq!(state.get("session-1"), SessionStatusInfo::Busy);

        state.set("session-2", SessionStatusInfo::Idle);
        assert_eq!(state.get("session-2"), SessionStatusInfo::Idle);
    }

    #[test]
    fn test_state_set_idle_removes_entry() {
        let state = SessionStatusState::new();

        state.set("session-1", SessionStatusInfo::Busy);
        assert!(state.list().contains_key("session-1"));

        state.set("session-1", SessionStatusInfo::Idle);
        assert!(!state.list().contains_key("session-1"));
    }

    #[test]
    fn test_state_list() {
        let state = SessionStatusState::new();

        state.set("s1", SessionStatusInfo::Busy);
        state.set(
            "s2",
            SessionStatusInfo::Retry {
                attempt: 1,
                message: "retrying".to_string(),
                next: 100,
            },
        );

        let list = state.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list.get("s1"), Some(&SessionStatusInfo::Busy));
    }
}
