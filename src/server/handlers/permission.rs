use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    pub metadata: serde_json::Value,
    pub always: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<PermissionTool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionTool {
    pub message_id: String,
    pub call_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PermissionReply {
    pub reply: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// GET /permission - List pending permission requests
pub async fn list() -> Json<Vec<PermissionRequest>> {
    info!("Listing pending permission requests");
    Json(vec![])
}

/// POST /permission/:id/reply - Reply to a permission request
pub async fn reply(
    Path(id): Path<String>,
    Json(body): Json<PermissionReply>,
) -> Result<Json<bool>, StatusCode> {
    info!("Processing permission reply for request {}: {:?}", id, body.reply);
    
    let valid_replies = ["once", "always", "reject"];
    if !valid_replies.contains(&body.reply.as_str()) {
        info!("Invalid permission reply: {}", body.reply);
        return Err(StatusCode::BAD_REQUEST);
    }
    
    info!("Permission request {} replied with: {}", id, body.reply);
    Ok(Json(true))
}