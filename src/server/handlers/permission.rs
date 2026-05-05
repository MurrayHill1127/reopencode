use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::permission::Reply;
use crate::server::AppState;

pub use crate::permission::Request as PermissionRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionTool {
    pub message_id: String,
    pub call_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PermissionReplyBody {
    pub reply: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// GET /permission — list pending permission requests
pub async fn list(State(state): State<AppState>) -> Json<Vec<PermissionRequest>> {
    info!("Listing pending permission requests");
    Json(state.permission_store.list_pending().await)
}

/// POST /permission/:id/reply — resolve a pending permission request
pub async fn reply(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PermissionReplyBody>,
) -> Result<Json<bool>, StatusCode> {
    info!("Permission reply for {}: {}", id, body.reply);

    let reply = match body.reply.as_str() {
        "once" => Reply::Once,
        "always" => Reply::Always,
        "reject" => Reply::Reject,
        _ => {
            info!("Invalid permission reply value: {}", body.reply);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    match state.permission_store.reply(&id, reply, body.message).await {
        Some(_) => Ok(Json(true)),
        None => {
            info!("Permission request {} not found", id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}
