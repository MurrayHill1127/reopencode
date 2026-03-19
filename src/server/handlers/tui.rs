//! TUI control routes for terminal user interface interactions.

use axum::{extract::Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiRequest {
    pub path: String,
    pub body: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TuiPromptAppend {
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TuiCommandExecute {
    pub command: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TuiToastShow {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TuiSessionSelect {
    pub session_id: String,
}

static TUI_REQUEST_QUEUE: once_cell::sync::Lazy<Arc<Mutex<Vec<TuiRequest>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));
static TUI_RESPONSE_QUEUE: once_cell::sync::Lazy<Arc<Mutex<Vec<serde_json::Value>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// GET /tui/control/next - Get next TUI request
pub async fn control_next() -> Json<TuiRequest> {
    let queue = TUI_REQUEST_QUEUE.lock().await;
    if let Some(req) = queue.last() {
        Json(req.clone())
    } else {
        Json(TuiRequest {
            path: "/tui/empty".to_string(),
            body: serde_json::json!({}),
        })
    }
}

/// POST /tui/control/response - Submit TUI response
pub async fn control_response(Json(body): Json<serde_json::Value>) -> Json<bool> {
    let mut queue = TUI_RESPONSE_QUEUE.lock().await;
    queue.push(body);
    Json(true)
}

/// POST /tui/append-prompt - Append prompt to TUI
pub async fn append_prompt(Json(body): Json<TuiPromptAppend>) -> Json<bool> {
    tracing::info!("TUI append_prompt: {}", body.prompt);
    Json(true)
}

/// POST /tui/open-help - Open help dialog
pub async fn open_help() -> Json<bool> {
    tracing::info!("TUI open_help");
    Json(true)
}

/// POST /tui/open-sessions - Open sessions dialog
pub async fn open_sessions() -> Json<bool> {
    tracing::info!("TUI open_sessions");
    Json(true)
}

/// POST /tui/open-themes - Open themes dialog
pub async fn open_themes() -> Json<bool> {
    tracing::info!("TUI open_themes");
    Json(true)
}

/// POST /tui/open-models - Open models dialog
pub async fn open_models() -> Json<bool> {
    tracing::info!("TUI open_models");
    Json(true)
}

/// POST /tui/submit-prompt - Submit prompt
pub async fn submit_prompt() -> Json<bool> {
    tracing::info!("TUI submit_prompt");
    Json(true)
}

/// POST /tui/clear-prompt - Clear prompt
pub async fn clear_prompt() -> Json<bool> {
    tracing::info!("TUI clear_prompt");
    Json(true)
}

/// POST /tui/execute-command - Execute TUI command
pub async fn execute_command(
    Json(body): Json<TuiCommandExecute>,
) -> Result<Json<bool>, StatusCode> {
    tracing::info!("TUI execute_command: {}", body.command);

    let valid_commands = [
        "session_new",
        "session_share",
        "session_interrupt",
        "session_compact",
        "messages_page_up",
        "messages_page_down",
        "messages_line_up",
        "messages_line_down",
        "messages_half_page_up",
        "messages_half_page_down",
        "messages_first",
        "messages_last",
        "agent_cycle",
    ];

    if !valid_commands.contains(&body.command.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(Json(true))
}

/// POST /tui/show-toast - Show toast notification
pub async fn show_toast(Json(body): Json<TuiToastShow>) -> Json<bool> {
    tracing::info!("TUI show_toast: {} (level: {:?})", body.message, body.level);
    Json(true)
}

/// POST /tui/publish - Publish TUI event
pub async fn publish(Json(body): Json<serde_json::Value>) -> Result<Json<bool>, StatusCode> {
    tracing::info!("TUI publish: {:?}", body);

    if !body.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(Json(true))
}

/// POST /tui/select-session - Select session
pub async fn select_session(Json(body): Json<TuiSessionSelect>) -> Result<Json<bool>, StatusCode> {
    tracing::info!("TUI select_session: {}", body.session_id);

    if body.session_id.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(Json(true))
}
