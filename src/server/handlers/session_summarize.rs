use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::info;

use crate::server::AppState;

/// Request body for summarizing a session
#[derive(Debug, Deserialize)]
pub struct SummarizeRequest {
    /// Provider ID for the model to use
    pub provider_id: String,
    /// Model ID for summarization
    pub model_id: String,
    /// Whether this is an auto-summarization
    #[serde(default)]
    pub auto: bool,
}

/// Summarize a session using AI compaction
///
/// POST /session/{id}/summarize
/// Generates a concise summary of the session using AI to preserve key information.
///
/// Note: This endpoint currently returns success but the actual AI compaction
/// logic needs to be implemented when the infrastructure is ready.
pub async fn summarize(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SummarizeRequest>,
) -> Result<Json<bool>, StatusCode> {
    info!(
        "Summarizing session: {} with model {}/{} (auto: {})",
        id, body.provider_id, body.model_id, body.auto
    );

    // Verify session exists
    match state.session_manager.get_session(&id).await {
        Ok(session) => {
            // Clean up any existing revert state
            if session.revert.is_some() {
                if let Err(e) = state.session_manager.clear_revert(&id).await {
                    tracing::warn!("Failed to clear revert state during summarization: {}", e);
                }
            }

            // TODO: Implement actual AI summarization
            // This would involve:
            // 1. Getting messages from the session
            // 2. Finding the last user message's agent
            // 3. Creating a compaction request
            // 4. Starting the prompt loop with the compaction

            // For now, return success to indicate the endpoint is working
            // The actual summarization logic can be implemented when
            // the AI compaction infrastructure is ready
            Ok(Json(true))
        }
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                tracing::error!("Failed to get session for summarization: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}