use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::info;

use crate::permission::Reply;
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct PermissionReplyBody {
    pub response: String, // "once" | "always" | "reject"
    pub message: Option<String>,
}

/// POST /session/{id}/permissions/{permissionID} - Respond to permission request
pub async fn permission_reply(
    State(state): State<AppState>,
    Path((session_id, permission_id)): Path<(String, String)>,
    Json(body): Json<PermissionReplyBody>,
) -> Result<Json<bool>, StatusCode> {
    info!(
        "Permission reply for session {}, permission {}: {}",
        session_id, permission_id, body.response
    );

    let reply = match body.response.as_str() {
        "once" => Reply::Once,
        "always" => Reply::Always,
        "reject" => Reply::Reject,
        _ => {
            info!("Invalid permission response: {}", body.response);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    match state.permission_store.reply(&permission_id, reply, body.message).await {
        Some(()) => Ok(Json(true)),
        None => {
            info!("Permission request {} not found", permission_id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}