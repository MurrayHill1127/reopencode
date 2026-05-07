use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tracing::info;

use crate::server::AppState;
use crate::session::types::{Session, SessionSummary};

/// In-memory store of message_id → snapshot hash for revert operations.
static SNAPSHOT_MAP: std::sync::LazyLock<Mutex<std::collections::HashMap<String, String>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Register a snapshot hash for a message (called during tool execution).
pub fn register_snapshot(message_id: &str, hash: &str) {
    if let Ok(mut map) = SNAPSHOT_MAP.lock() {
        map.insert(message_id.to_string(), hash.to_string());
    }
}

/// Request body for reverting a session
#[derive(Debug, Deserialize)]
pub struct RevertRequest {
    /// Message ID to revert to
    pub message_id: String,
    /// Optional part ID within the message
    pub part_id: Option<String>,
    /// Optional summary of changes
    pub summary: Option<SummaryInput>,
}

/// Summary input for revert
#[derive(Debug, Deserialize)]
pub struct SummaryInput {
    pub additions: u32,
    pub deletions: u32,
    pub files: u32,
}

impl From<SummaryInput> for SessionSummary {
    fn from(s: SummaryInput) -> Self {
        Self {
            additions: s.additions,
            deletions: s.deletions,
            files: s.files,
        }
    }
}

/// Response for revert operation
#[derive(Debug, Serialize)]
pub struct RevertResponse {
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub message_count: u32,
    pub revert: Option<RevertInfo>,
}

/// Revert info in response
#[derive(Debug, Serialize)]
pub struct RevertInfo {
    pub message_id: String,
    pub part_id: Option<String>,
    pub snapshot: Option<String>,
}

impl From<Session> for RevertResponse {
    fn from(s: Session) -> Self {
        Self {
            id: s.id,
            title: s.title,
            parent_id: s.parent_id,
            created_at: s.created_at,
            updated_at: s.updated_at,
            status: match s.status {
                crate::session::SessionStatus::Active => "active".to_string(),
                crate::session::SessionStatus::Paused => "paused".to_string(),
                crate::session::SessionStatus::Completed => "completed".to_string(),
            },
            message_count: s.message_count,
            revert: s.revert.map(|r| RevertInfo {
                message_id: r.message_id,
                part_id: r.part_id,
                snapshot: r.snapshot,
            }),
        }
    }
}

/// Revert a session to a specific message
///
/// POST /session/{id}/revert
/// Sets the session's revert state to track the reverted messages.
pub async fn revert(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RevertRequest>,
) -> Result<Json<RevertResponse>, StatusCode> {
    info!(
        "Reverting session: {} to message: {} (part: {:?})",
        id, body.message_id, body.part_id
    );

    // Try to restore files from snapshot
    let snapshot_hash = SNAPSHOT_MAP
        .lock()
        .ok()
        .and_then(|map| map.get(&body.message_id).cloned());

    let mut diff_summary = None;
    if let Some(ref hash) = snapshot_hash {
        match state.snapshot.diff_full(hash, "HEAD").await {
            Ok(diffs) => {
                let additions: u32 = diffs.iter().map(|d| d.additions).sum();
                let deletions: u32 = diffs.iter().map(|d| d.deletions).sum();
                let files = diffs.len() as u32;
                diff_summary = Some(SessionSummary { additions, deletions, files });
                // Actually restore the files
                let _ = state.snapshot.restore(hash).await;
            }
            Err(e) => {
                tracing::warn!("Failed to diff snapshot {}: {}", hash, e);
            }
        }
    }

    let summary = body.summary.map(SessionSummary::from).or(diff_summary);

    match state
        .session_manager
        .set_revert(&id, &body.message_id, body.part_id.as_deref(), summary)
        .await
    {
        Ok(session) => Ok(Json(RevertResponse::from(session))),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to revert session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}