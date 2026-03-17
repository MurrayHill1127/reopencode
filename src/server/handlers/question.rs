use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestionRequest {
    pub id: String,
    pub session_id: String,
    pub questions: Vec<QuestionInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<QuestionTool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestionInfo {
    pub question: String,
    pub header: String,
    pub options: Vec<QuestionOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuestionTool {
    pub message_id: String,
    pub call_id: String,
}

#[derive(Debug, Deserialize)]
pub struct QuestionReply {
    pub answers: Vec<Vec<String>>,
}

/// GET /question - List pending questions
pub async fn list() -> Json<Vec<QuestionRequest>> {
    info!("Listing pending questions");
    Json(vec![])
}

/// POST /question/:id/reply - Reply to a question
pub async fn reply(
    Path(id): Path<String>,
    Json(body): Json<QuestionReply>,
) -> Result<Json<bool>, StatusCode> {
    info!("Replying to question {}: {:?}", id, body.answers);
    Ok(Json(true))
}

/// POST /question/:id/reject - Reject a question
pub async fn reject(Path(id): Path<String>) -> Result<Json<bool>, StatusCode> {
    info!("Rejecting question {}", id);
    Ok(Json(true))
}