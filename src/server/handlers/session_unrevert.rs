use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use tracing::info;

use crate::server::AppState;
use crate::session::types::Session;

/// Response for unrevert operation
#[derive(Debug, Serialize)]
pub struct UnrevertResponse {
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub message_count: u32,
    pub revert: Option<serde_json::Value>,
}

impl From<Session> for UnrevertResponse {
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
            revert: s.revert.map(|r| serde_json::to_value(r).unwrap_or_default()),
        }
    }
}

/// Restore reverted messages in a session
///
/// POST /session/{id}/unrevert
/// Clears the session's revert state, restoring the session to its full state.
pub async fn unrevert(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UnrevertResponse>, StatusCode> {
    info!("Unreverting session: {}", id);

    match state.session_manager.clear_revert(&id).await {
        Ok(session) => Ok(Json(UnrevertResponse::from(session))),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to unrevert session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}