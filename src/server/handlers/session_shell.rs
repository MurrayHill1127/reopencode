use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct ShellInput {
    pub agent: String,
    pub model: Option<ModelRef>,
    pub command: String,
}

#[derive(Debug, Deserialize)]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

#[derive(Debug, Serialize)]
pub struct AssistantMessage {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub role: String,
    pub time: MessageTime,
}

#[derive(Debug, Serialize)]
pub struct MessageTime {
    pub created: i64,
}

/// POST /session/{id}/shell - Execute shell command
///
/// Executes a shell command within the session context and returns the AI's response.
pub async fn shell(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<ShellInput>,
) -> Result<Json<AssistantMessage>, StatusCode> {
    info!(
        "Shell command for session {} (agent: {}): {}",
        session_id, body.agent, body.command
    );

    // Verify session exists
    if state.session_manager.get_session(&session_id).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Stub response - actual shell execution logic needed
    let now = chrono::Utc::now().timestamp_millis();
    Ok(Json(AssistantMessage {
        id: uuid::Uuid::new_v4().to_string(),
        session_id,
        role: "assistant".to_string(),
        time: MessageTime { created: now },
    }))
}