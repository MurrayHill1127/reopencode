use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::info;

use crate::server::AppState;

/// Abort a session's active operation
///
/// POST /session/{id}/abort
/// Returns true on success (even if session was already idle)
/// Returns 404 if session doesn't exist
pub async fn abort(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<bool>, StatusCode> {
    info!("Aborting session: {}", id);

    match state.session_manager.abort_session(&id).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to abort session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}