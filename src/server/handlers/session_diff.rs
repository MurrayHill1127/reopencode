use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::server::AppState;
use crate::session::types::FileDiff;

#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub message_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffResponse {
    pub diffs: Vec<FileDiff>,
}

pub async fn get_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DiffQuery>,
) -> Result<Json<DiffResponse>, StatusCode> {
    info!("Getting diff for session: {} (message: {:?})", id, query.message_id);

    match state
        .session_manager
        .get_session_diff(&id, query.message_id.as_deref())
        .await
    {
        Ok(diffs) => Ok(Json(DiffResponse { diffs })),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to get diff: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}