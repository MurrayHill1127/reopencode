use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::info;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct PromptInput {
    #[serde(rename = "messageID")]
    pub message_id: Option<String>,
    pub model: Option<ModelRef>,
    pub agent: Option<String>,
    #[serde(rename = "noReply")]
    pub no_reply: Option<bool>,
    pub parts: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// POST /session/{id}/prompt_async - Async prompt (returns 204 immediately)
///
/// Sends a message to a session asynchronously, starting the session if needed
/// and returning immediately without waiting for the response.
pub async fn prompt_async(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(_body): Json<PromptInput>,
) -> Result<StatusCode, StatusCode> {
    info!("Async prompt for session: {}", session_id);

    // Verify session exists
    if state.session_manager.get_session(&session_id).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Spawn background task for prompt processing
    let sid = session_id.clone();
    tokio::spawn(async move {
        info!("Background prompt processing for session: {}", sid);
        // TODO: actual prompt processing logic
        // This would call SessionPrompt.prompt() equivalent
    });

    Ok(StatusCode::NO_CONTENT)
}