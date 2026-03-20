use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::error;

use crate::server::AppState;
use crate::session::TodoInfo;

pub async fn get_todos(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<TodoInfo>>, StatusCode> {
    match state.session_manager.get_session(&id).await {
        Ok(_) => match state.session_manager.get_todos(&id).await {
            Ok(todos) => Ok(Json(todos)),
            Err(e) => {
                error!("Failed to get todos for session {}: {}", id, e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
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