//! Todo management for sessions

use serde::{Deserialize, Serialize};

/// Todo item information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoInfo {
    /// Brief description of the task
    pub content: String,
    /// Current status: pending, in_progress, completed, cancelled
    pub status: String,
    /// Priority level: high, medium, low
    pub priority: String,
}

/// Status constants for todo items
pub mod status {
    pub const PENDING: &str = "pending";
    pub const IN_PROGRESS: &str = "in_progress";
    pub const COMPLETED: &str = "completed";
    pub const CANCELLED: &str = "cancelled";
}

/// Priority constants for todo items
pub mod priority {
    pub const HIGH: &str = "high";
    pub const MEDIUM: &str = "medium";
    pub const LOW: &str = "low";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_info_serialization() {
        let todo = TodoInfo {
            content: "Implement feature X".to_string(),
            status: status::IN_PROGRESS.to_string(),
            priority: priority::HIGH.to_string(),
        };

        let json = serde_json::to_string(&todo).unwrap();
        assert!(json.contains("Implement feature X"));
        assert!(json.contains("in_progress"));
        assert!(json.contains("high"));

        let parsed: TodoInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.content, todo.content);
        assert_eq!(parsed.status, todo.status);
        assert_eq!(parsed.priority, todo.priority);
    }

    #[test]
    fn test_status_constants() {
        assert_eq!(status::PENDING, "pending");
        assert_eq!(status::IN_PROGRESS, "in_progress");
        assert_eq!(status::COMPLETED, "completed");
        assert_eq!(status::CANCELLED, "cancelled");
    }

    #[test]
    fn test_priority_constants() {
        assert_eq!(priority::HIGH, "high");
        assert_eq!(priority::MEDIUM, "medium");
        assert_eq!(priority::LOW, "low");
    }
}
