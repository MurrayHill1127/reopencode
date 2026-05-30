//! Session undo/redo handlers.
//!
//! - POST /session/{id}/undo  — removes the last user+assistant message pair
//! - POST /session/{id}/redo  — not yet supported (requires message stash)

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Serialize;
use tracing::info;

use crate::server::AppState;

#[derive(Debug, Serialize)]
pub struct UndoResponse {
    pub success: bool,
    pub messages_removed: usize,
}

/// POST /session/{id}/undo
///
/// Removes the last user message and its corresponding assistant reply.
/// Uses SnapshotManager to restore files if a snapshot exists.
pub async fn undo(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<UndoResponse>, StatusCode> {
    info!("Undo for session: {}", session_id);

    if state.session_manager.get_session(&session_id).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get messages to find the last user message
    let messages = state.session_manager.get_messages(&session_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if messages.is_empty() {
        return Ok(Json(UndoResponse { success: false, messages_removed: 0 }));
    }

    // Count how many messages to remove: last assistant + last user (up to 2)
    let to_remove = messages.iter().rev()
        .take_while(|m| m.role == "assistant" || m.role == "tool")
        .count()
        .max(1)
        + 1; // +1 for the user message

    let removed = state.session_manager
        .delete_last_messages(&session_id, to_remove.min(messages.len()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Attempt to restore files from the most recent snapshot
    if let Ok(Some(hash)) = state.snapshot.track().await {
        let _ = state.snapshot.restore(&hash).await;
    }

    Ok(Json(UndoResponse { success: true, messages_removed: removed }))
}

/// POST /session/{id}/redo
///
/// Redo is not yet supported — requires a message stash mechanism.
pub async fn redo(
    State(_state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<UndoResponse>, StatusCode> {
    info!("Redo requested for session: {} (not yet supported)", session_id);
    Ok(Json(UndoResponse { success: false, messages_removed: 0 }))
}
