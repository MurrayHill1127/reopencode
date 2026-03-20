use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::error;

use crate::server::AppState;

use super::session::SessionInfo;

pub async fn get_children(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<SessionInfo>>, StatusCode> {
    match state.session_manager.get_session(&id).await {
        Ok(_) => {
            match state.session_manager.get_children(&id).await {
                Ok(children) => Ok(Json(children.into_iter().map(SessionInfo::from).collect())),
                Err(e) => {
                    error!("Failed to get children for session {}: {}", id, e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                error!("Failed to get session {}: {}", id, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}