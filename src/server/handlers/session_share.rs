use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Serialize;
use tracing::info;

use crate::server::AppState;
use crate::session::types::Session;

#[derive(Debug, Serialize)]
pub struct ShareResponse {
    pub id: String,
    pub title: String,
    pub parent_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub message_count: u32,
    pub share_url: Option<String>,
}

impl From<Session> for ShareResponse {
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
            share_url: s.share_url,
        }
    }
}

pub async fn share(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ShareResponse>, StatusCode> {
    info!("Sharing session: {}", id);

    match state.session_manager.share_session(&id).await {
        Ok(session) => Ok(Json(ShareResponse::from(session))),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to share session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn unshare(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ShareResponse>, StatusCode> {
    info!("Unsharing session: {}", id);

    match state.session_manager.unshare_session(&id).await {
        Ok(session) => Ok(Json(ShareResponse::from(session))),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to unshare session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}