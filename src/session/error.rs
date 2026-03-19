//! Session management errors

use thiserror::Error;

/// Session-related errors
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {session_id}")]
    NotFound { session_id: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("DateTime parse error: {0}")]
    DateTimeParse(String),
}

/// Result type alias for session operations
pub type Result<T> = std::result::Result<T, SessionError>;

impl SessionError {
    /// Check if error is NotFound
    pub fn is_not_found(&self) -> bool {
        matches!(self, SessionError::NotFound { .. })
    }

    /// Check if error is Database error
    pub fn is_database(&self) -> bool {
        matches!(self, SessionError::Database(_))
    }

    /// Extract session_id if NotFound
    pub fn session_id(&self) -> Option<&String> {
        match self {
            SessionError::NotFound { session_id } => Some(session_id),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let not_found = SessionError::NotFound {
            session_id: "test-123".to_string(),
        };
        assert!(not_found.is_not_found());
        assert!(!not_found.is_database());
        assert_eq!(not_found.session_id(), Some(&"test-123".to_string()));

        let db_err = SessionError::Database(sqlx::Error::RowNotFound);
        assert!(!db_err.is_not_found());
        assert!(db_err.is_database());
        assert_eq!(db_err.session_id(), None);
    }

    #[test]
    fn test_error_display() {
        let not_found = SessionError::NotFound {
            session_id: "abc-456".to_string(),
        };
        let display = format!("{}", not_found);
        assert_eq!(display, "Session not found: abc-456");

        let io_err = SessionError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let session_err: SessionError = io_err.into();
        assert!(!session_err.is_database());
        assert!(session_err.to_string().contains("Access denied"));
    }

    #[test]
    fn test_from_serialization_error() {
        let invalid_json = "not valid json";
        let result: std::result::Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }
}
