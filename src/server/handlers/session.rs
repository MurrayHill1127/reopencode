use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::session::{Session, SessionStatus};

#[derive(Debug, Serialize)]
pub struct SessionInfo {
    id: String,
    title: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    status: String,
    message_count: u32,
}

impl From<Session> for SessionInfo {
    fn from(s: Session) -> Self {
        Self {
            id: s.id,
            title: s.title,
            created_at: s.created_at,
            updated_at: s.updated_at,
            status: match s.status {
                SessionStatus::Active => "active".to_string(),
                SessionStatus::Paused => "paused".to_string(),
                SessionStatus::Completed => "completed".to_string(),
            },
            message_count: s.message_count,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub response: String,
}

pub async fn list() -> Json<Vec<SessionInfo>> {
    Json(vec![])
}

pub async fn create(Json(body): Json<CreateSessionRequest>) -> Json<SessionInfo> {
    let session = Session::new(body.title);
    Json(SessionInfo::from(session))
}

pub async fn get(Path(_id): Path<String>) -> StatusCode {
    StatusCode::NOT_FOUND
}

pub async fn send_message(
    Path(session_id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Json<SendMessageResponse> {
    let message_id = uuid::Uuid::new_v4().to_string();
    
    Json(SendMessageResponse {
        message_id,
        response: format!("Echo: {} (session: {})", body.content, session_id),
    })
}