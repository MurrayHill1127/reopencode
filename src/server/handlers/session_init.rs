use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct InitRequest {
    pub model_id: String,
    pub provider_id: String,
    pub message_id: String,
}

#[derive(Debug, Serialize)]
pub struct InitResponse {
    pub success: bool,
}

pub async fn init(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<InitRequest>,
) -> Result<Json<InitResponse>, StatusCode> {
    info!("Initializing session: {} with model {}/{}", id, body.provider_id, body.model_id);

    match state.session_manager.get_session(&id).await {
        Ok(_) => {
            // In a full implementation, this would trigger the /init command
            // to generate AGENTS.md. For now, return success.
            Ok(Json(InitResponse { success: true }))
        }
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to init session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}