use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::sse::{Event, Sse},
};
use futures::stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::agent::{Agent, Message, Role};
use crate::provider::{Message as ProviderMessage, MessageRole as ProviderMessageRole, ToolDefinition};
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
    parent_id: Option<String>,
    archived_at: Option<i64>,
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
            parent_id: s.parent_id,
            archived_at: s.archived_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    title: Option<String>,
    parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSessionRequest {
    pub title: Option<String>,
    pub time: Option<UpdateTime>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTime {
    pub archived: Option<i64>,
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
    match state.session_manager.create_session(body.title, body.parent_id).await {
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

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSessionRequest>,
) -> Result<Json<SessionInfo>, StatusCode> {
    let mut session = match state.session_manager.get_session(&id).await {
        Ok(s) => s,
        Err(e) => {
            if e.is_not_found() {
                return Err(StatusCode::NOT_FOUND);
            } else {
                error!("Failed to get session: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    if let Some(title) = body.title {
        session = state
            .session_manager
            .set_title(&id, title)
            .await
            .map_err(|e| {
                error!("Failed to update title: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    if let Some(time) = body.time {
        if let Some(archived) = time.archived {
            session = state
                .session_manager
                .set_archived(&id, Some(archived))
                .await
                .map_err(|e| {
                    error!("Failed to update archived: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
    }

    Ok(Json(SessionInfo::from(session)))
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
    info!("Streaming message for session {}", session_id);

    let (tx, rx) = mpsc::unbounded_channel::<String>();

    // Clone what the spawned task needs.
    let provider = state.provider.clone();
    let session_manager = state.session_manager.clone();
    let tools_registry = state.tools.clone();
    let cwd = state.cwd.clone();
    let config = state.agent.config().clone();
    let user_content = body.content.clone();

    tokio::spawn(async move {
        // --- store user message ---
        if session_manager.add_message(&session_id, "user", &user_content).await.is_err() {
            let _ = tx.send("[ERROR] Failed to store user message".to_string());
            return;
        }

        // --- build tool definitions from registry ---
        let tool_names = tools_registry.list();
        let tool_defs: Vec<ToolDefinition> = tool_names.iter()
            .filter_map(|name| tools_registry.get(name))
            .map(|t| ToolDefinition::from(&t.to_agent_definition()))
            .collect();

        // --- build system prompt ---
        let system_prompt = format!(
            "You are an expert software engineering assistant running in the directory: {cwd}\n\n\
             You have access to a set of tools to read files, write code, search the codebase, \
             and run shell commands. Use them to complete the user's request thoroughly.\n\n\
             Guidelines:\n\
             - Read files before editing them\n\
             - Prefer targeted edits over full rewrites\n\
             - Run tests after making changes when possible\n\
             - Explain your reasoning briefly before acting\n\
             - If uncertain about a file's contents, read it first",
            cwd = cwd
        );

        // --- load conversation history ---
        let history = match session_manager.get_messages(&session_id).await {
            Ok(h) => h,
            Err(e) => {
                let _ = tx.send(format!("[ERROR] Failed to load history: {e}"));
                return;
            }
        };

        // Build message list: system + history (history already includes the user message we just stored)
        let mut messages: Vec<ProviderMessage> = Vec::new();
        messages.push(ProviderMessage::new(ProviderMessageRole::System, &system_prompt));
        for msg in &history {
            let role = if msg.role == "user" {
                ProviderMessageRole::User
            } else {
                ProviderMessageRole::Assistant
            };
            messages.push(ProviderMessage::new(role, &msg.content));
        }

        // --- agent loop ---
        const MAX_STEPS: usize = 20;
        let mut full_response = String::new();

        for step in 0..MAX_STEPS {
            let response = match provider.chat(
                messages.clone(),
                &config.model,
                config.temperature,
                config.max_tokens,
                &tool_defs,
            ).await {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("[ERROR] Provider error: {e}");
                    let _ = tx.send(msg);
                    return;
                }
            };

            // Stream any text content immediately
            if !response.content.is_empty() {
                full_response.push_str(&response.content);
                // Send in small chunks so the TUI renders progressively
                for chunk in response.content.split_inclusive(' ') {
                    let _ = tx.send(chunk.to_string());
                }
            }

            if response.tool_calls.is_empty() {
                // Final response — done
                break;
            }

            info!("Step {}: executing {} tool calls", step + 1, response.tool_calls.len());

            // Append the assistant message with tool_calls to history
            let assistant_msg = ProviderMessage {
                role: ProviderMessageRole::Assistant,
                content: response.content.clone(),
                tool_call_id: None,
                tool_calls: response.tool_calls.clone(),
            };
            messages.push(assistant_msg);

            // Execute each tool call and append results
            for call in &response.tool_calls {
                let tool_name = &call.function.name;
                let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));

                let result_text = match tools_registry.get(tool_name) {
                    None => {
                        warn!("Tool not found: {}", tool_name);
                        format!("Error: tool '{}' not found", tool_name)
                    }
                    Some(tool) => {
                        // Announce the tool call to the stream
                        let announce = format!("\n\n**Tool: {}**\n```\n{}\n```\n\n", tool_name, args);
                        let _ = tx.send(announce);

                        match tool.execute(args).await {
                            Ok(r) => r.output,
                            Err(e) => format!("Error: {e}"),
                        }
                    }
                };

                messages.push(ProviderMessage {
                    role: ProviderMessageRole::Tool,
                    content: result_text,
                    tool_call_id: Some(call.id.clone()),
                    tool_calls: vec![],
                });
            }
        }

        // --- store assistant response ---
        if !full_response.is_empty() {
            if let Err(e) = session_manager.add_message(&session_id, "assistant", &full_response).await {
                error!("Failed to store assistant response: {}", e);
            }
        }
    });

    // Convert mpsc channel to SSE stream
    let sse_stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|chunk| {
            (Ok::<Event, Infallible>(Event::default().data(chunk)), rx)
        })
    });

    Sse::new(sse_stream)
}
