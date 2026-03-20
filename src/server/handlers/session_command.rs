use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct CommandInput {
    #[serde(rename = "messageID")]
    pub message_id: Option<String>,
    pub agent: Option<String>,
    pub model: Option<String>,
    pub arguments: String,
    pub command: String,
    pub variant: Option<String>,
    pub parts: Option<Vec<FilePartInput>>,
}

#[derive(Debug, Deserialize)]
pub struct FilePartInput {
    #[serde(rename = "type")]
    pub part_type: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct CommandResponse {
    pub info: AssistantMessage,
    pub parts: Vec<serde_json::Value>,
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

/// POST /session/{id}/command - Execute slash command
///
/// Sends a command to a session for execution by the AI assistant.
pub async fn command(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<CommandInput>,
) -> Result<Json<CommandResponse>, StatusCode> {
    info!(
        "Command '{}' with args '{}' for session: {}",
        body.command, body.arguments, session_id
    );

    // Verify session exists
    if state.session_manager.get_session(&session_id).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Stub response - actual command execution logic needed
    let now = chrono::Utc::now().timestamp_millis();
    Ok(Json(CommandResponse {
        info: AssistantMessage {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            role: "assistant".to_string(),
            time: MessageTime { created: now },
        },
        parts: vec![],
    }))
}