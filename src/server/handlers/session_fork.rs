use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::server::AppState;
use crate::session::types::Session;

/// Request body for forking a session
#[derive(Debug, Deserialize)]
pub struct ForkRequest {
    /// Optional message ID to fork at (messages after this will not be copied)
    pub message_id: Option<String>,
}

/// Response for fork operation
#[derive(Debug, Serialize)]
pub struct ForkResponse {
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub message_count: u32,
}

impl From<Session> for ForkResponse {
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
        }
    }
}

/// Fork a session at a specific message point
///
/// POST /session/{id}/fork
/// Creates a new session that is a fork of the original, optionally up to a specific message.
pub async fn fork(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ForkRequest>,
) -> Result<Json<ForkResponse>, StatusCode> {
    info!("Forking session: {} (up to message: {:?})", id, body.message_id);

    match state
        .session_manager
        .fork_session(&id, body.message_id.as_deref())
        .await
    {
        Ok(session) => Ok(Json(ForkResponse::from(session))),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to fork session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}