use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct PermissionReplyBody {
    pub response: String, // "once" | "always" | "reject"
}

/// POST /session/{id}/permissions/{permissionID} - Respond to permission request
///
/// Approve or deny a permission request from the AI assistant.
/// Note: This endpoint is deprecated in the TypeScript version but kept for compatibility.
pub async fn permission_reply(
    Path((session_id, permission_id)): Path<(String, String)>,
    Json(body): Json<PermissionReplyBody>,
) -> Result<Json<bool>, StatusCode> {
    info!(
        "Permission reply for session {}, permission {}: {}",
        session_id, permission_id, body.response
    );

    let valid_responses = ["once", "always", "reject"];
    if !valid_responses.contains(&body.response.as_str()) {
        info!("Invalid permission response: {}", body.response);
        return Err(StatusCode::BAD_REQUEST);
    }

    // TODO: actual permission handling via permission store
    // This would call PermissionNext.reply() equivalent
    info!("Permission request {} replied with: {}", permission_id, body.response);
    Ok(Json(true))
}