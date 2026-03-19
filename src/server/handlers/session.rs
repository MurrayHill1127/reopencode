use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::sse::{Event, Sse},
};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tracing::{error, info};

use crate::agent::{Agent, Message, Role};
use crate::provider::Message as ProviderMessage;
use crate::provider::MessageRole as ProviderMessageRole;
use crate::server::AppState;
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

pub async fn list(State(state): State<AppState>) -> Json<Vec<SessionInfo>> {
    match state.session_manager.list_sessions().await {
        Ok(sessions) => Json(sessions.into_iter().map(SessionInfo::from).collect()),
        Err(e) => {
            error!("Failed to list sessions: {}", e);
            Json(vec![])
        }
    }
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<Json<SessionInfo>, StatusCode> {
    match state.session_manager.create_session(body.title).await {
        Ok(id) => match state.session_manager.get_session(&id).await {
            Ok(session) => Ok(Json(SessionInfo::from(session))),
            Err(e) => {
                error!("Failed to fetch created session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        },
        Err(e) => {
            error!("Failed to create session: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, StatusCode> {
    match state.session_manager.get_session(&id).await {
        Ok(session) => Ok(Json(SessionInfo::from(session))),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                error!("Failed to get session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    match state.session_manager.delete_session(&id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            if e.is_not_found() {
                Err(StatusCode::NOT_FOUND)
            } else {
                error!("Failed to delete session: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

pub async fn send_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, StatusCode> {
    info!(
        "Processing message for session {}: {}",
        session_id, body.content
    );

    if state
        .session_manager
        .get_session(&session_id)
        .await
        .is_err()
    {
        return Err(StatusCode::NOT_FOUND);
    }

    let user_msg_id = match state
        .session_manager
        .add_message(&session_id, "user", &body.content)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to store user message: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let messages = match state.session_manager.get_messages(&session_id).await {
        Ok(msgs) => msgs,
        Err(e) => {
            error!("Failed to get session messages: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let agent_messages: Vec<Message> = messages
        .iter()
        .map(|m| Message {
            role: if m.role == "user" {
                Role::User
            } else {
                Role::Assistant
            },
            content: m.content.clone(),
        })
        .collect();

    match state.agent.execute(agent_messages, vec![]).await {
        Ok(response) => {
            info!(
                "Agent response: {} tokens used",
                response.usage.total_tokens
            );

            match state
                .session_manager
                .add_message(&session_id, "assistant", &response.content)
                .await
            {
                Ok(_) => Ok(Json(SendMessageResponse {
                    message_id: user_msg_id,
                    response: response.content,
                })),
                Err(e) => {
                    error!("Failed to store assistant response: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            error!("Agent execution failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn stream_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    info!(
        "Streaming message for session {}: {}",
        session_id, body.content
    );

    let provider_messages = vec![ProviderMessage::new(
        ProviderMessageRole::User,
        body.content,
    )];

    let config = state.agent.config();
    let stream = state.provider.chat_stream(
        provider_messages,
        &config.model,
        config.temperature,
        config.max_tokens,
        &[],
    );

    let sse_stream = futures::stream::StreamExt::map(stream, |result| match result {
        Ok(content) => Ok(Event::default().data(content)),
        Err(e) => {
            error!("Stream error: {}", e);
            Ok(Event::default().data(format!("[ERROR] {}", e)))
        }
    });

    Sse::new(sse_stream)
}
