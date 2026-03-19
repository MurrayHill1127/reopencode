use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PtyStatus {
    Running,
    Exited,
}

impl Default for PtyStatus {
    fn default() -> Self {
        Self::Running
    }
}

impl std::fmt::Display for PtyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PtyStatus::Running => write!(f, "running"),
            PtyStatus::Exited => write!(f, "exited"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtyInfo {
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: PtyStatus,
    pub pid: u32,
    pub exit_code: Option<i32>,
}

impl PtyInfo {
    pub fn new(id: String, command: String, args: Vec<String>, cwd: String, pid: u32) -> Self {
        let title = format!("Terminal {}", &id[id.len().saturating_sub(4)..]);
        Self {
            id,
            title,
            command,
            args,
            cwd,
            status: PtyStatus::Running,
            pid,
            exit_code: None,
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.title = title;
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreatePtyRequest {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub rows: Option<u16>,
    #[serde(default)]
    pub cols: Option<u16>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdatePtyRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub size: Option<PtySize>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct PtyOutput {
    pub data: String,
    pub cursor: usize,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PtyError {
    #[error("PTY session not found: {0}")]
    NotFound(String),

    #[error("PTY spawn failed: {0}")]
    SpawnFailed(String),

    #[error("PTY resize failed: {0}")]
    ResizeFailed(String),

    #[error("PTY write failed: {0}")]
    WriteFailed(String),

    #[error("PTY read failed: {0}")]
    ReadFailed(String),

    #[error("PTY already exited: {0}")]
    AlreadyExited(String),

    #[error("Invalid size: rows={rows}, cols={cols}")]
    InvalidSize { rows: u16, cols: u16 },
}

pub const BUFFER_LIMIT: usize = 2 * 1024 * 1024;
pub const DEFAULT_ROWS: u16 = 24;
pub const DEFAULT_COLS: u16 = 80;

pub fn generate_pty_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("pty_{:04}", id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pty_status_serialization() {
        assert_eq!(
            serde_json::to_string(&PtyStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&PtyStatus::Exited).unwrap(),
            "\"exited\""
        );
    }

    #[test]
    fn test_pty_status_deserialization() {
        let running: PtyStatus = serde_json::from_str("\"running\"").unwrap();
        assert_eq!(running, PtyStatus::Running);

        let exited: PtyStatus = serde_json::from_str("\"exited\"").unwrap();
        assert_eq!(exited, PtyStatus::Exited);
    }

    #[test]
    fn test_pty_info_new() {
        let info = PtyInfo::new(
            "pty_0001".to_string(),
            "bash".to_string(),
            vec!["-l".to_string()],
            "/home/user".to_string(),
            12345,
        );

        assert_eq!(info.id, "pty_0001");
        assert_eq!(info.title, "Terminal 0001");
        assert_eq!(info.command, "bash");
        assert_eq!(info.args, vec!["-l"]);
        assert_eq!(info.cwd, "/home/user");
        assert_eq!(info.status, PtyStatus::Running);
        assert_eq!(info.pid, 12345);
        assert!(info.exit_code.is_none());
    }

    #[test]
    fn test_pty_info_with_title() {
        let info = PtyInfo::new(
            "pty_0001".to_string(),
            "bash".to_string(),
            vec![],
            "/".to_string(),
            1,
        )
        .with_title("My Terminal".to_string());

        assert_eq!(info.title, "My Terminal");
    }

    #[test]
    fn test_create_pty_request_default() {
        let req = CreatePtyRequest::default();
        assert!(req.command.is_none());
        assert!(req.args.is_none());
        assert!(req.cwd.is_none());
        assert!(req.title.is_none());
        assert!(req.env.is_none());
        assert!(req.rows.is_none());
        assert!(req.cols.is_none());
    }

    #[test]
    fn test_update_pty_request_default() {
        let req = UpdatePtyRequest::default();
        assert!(req.title.is_none());
        assert!(req.size.is_none());
    }

    #[test]
    fn test_pty_size() {
        let size = PtySize { rows: 24, cols: 80 };
        assert_eq!(size.rows, 24);
        assert_eq!(size.cols, 80);
    }

    #[test]
    fn test_generate_pty_id() {
        let id1 = generate_pty_id();
        let id2 = generate_pty_id();

        assert!(id1.starts_with("pty_"));
        assert!(id2.starts_with("pty_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_pty_error_display() {
        let err = PtyError::NotFound("test_id".to_string());
        assert_eq!(err.to_string(), "PTY session not found: test_id");

        let err = PtyError::SpawnFailed("permission denied".to_string());
        assert_eq!(err.to_string(), "PTY spawn failed: permission denied");
    }
}
