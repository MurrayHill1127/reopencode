use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::{error, info};

use crate::agent::Agent;
use crate::provider::{Message as ProviderMessage, MessageRole as ProviderMessageRole, ToolDefinition};
use crate::server::AppState;
use crate::server::handlers::session::load_system_prompt;

#[derive(Debug, Deserialize)]
pub struct PromptInput {
    #[serde(rename = "messageID")]
    pub message_id: Option<String>,
    pub model: Option<ModelRef>,
    pub agent: Option<String>,
    #[serde(rename = "noReply")]
    pub no_reply: Option<bool>,
    pub parts: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// POST /session/{id}/prompt_async - Async prompt (returns 204 immediately)
///
/// Sends a message to a session asynchronously, starting the session if needed
/// and returning immediately without waiting for the response.
pub async fn prompt_async(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<PromptInput>,
) -> Result<StatusCode, StatusCode> {
    info!("Async prompt for session: {}", session_id);

    // Verify session exists
    if state.session_manager.get_session(&session_id).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Skip if no_reply is set
    if body.no_reply == Some(true) {
        return Ok(StatusCode::NO_CONTENT);
    }

    // Extract text content from parts
    let content = body.parts
        .as_ref()
        .and_then(|parts| {
            parts.iter().find_map(|p| {
                p.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
            })
        })
        .unwrap_or_default();

    if content.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }

    // Clone state for background task
    let provider = state.provider.clone();
    let session_manager = state.session_manager.clone();
    let tools = state.tools.clone();
    let config = state.agent.config().clone();
    let cwd = state.cwd.clone();
    let active_sessions = state.active_sessions.clone();

    // Reject if session already processing
    if active_sessions.contains_key(&session_id) {
        return Err(StatusCode::CONFLICT);
    }
    active_sessions.insert(session_id.clone(), ());

    tokio::spawn(async move {
        // Store user message
        if let Err(e) = session_manager.add_message(&session_id, "user", &content).await {
            error!("prompt_async: failed to store user message: {}", e);
            active_sessions.remove(&session_id);
            return;
        }

        // Build tool definitions
        let tool_names = tools.list();
        let tool_defs: Vec<ToolDefinition> = tool_names.iter()
            .filter_map(|name| tools.get(name))
            .map(|t| ToolDefinition::from(&t.to_agent_definition()))
            .collect();

        let system_prompt = load_system_prompt(&cwd);

        // Load history
        let history = match session_manager.get_messages(&session_id).await {
            Ok(h) => h,
            Err(e) => {
                error!("prompt_async: failed to load history: {}", e);
                active_sessions.remove(&session_id);
                return;
            }
        };

        let mut messages: Vec<ProviderMessage> = Vec::new();
        messages.push(ProviderMessage::new(ProviderMessageRole::System, &system_prompt));
        for msg in &history {
            let role = if msg.role == "user" { ProviderMessageRole::User } else { ProviderMessageRole::Assistant };
            messages.push(ProviderMessage::new(role, &msg.content));
        }

        // Agent loop (up to 20 steps)
        let mut full_response = String::new();
        for _step in 0..20usize {
            let response = match provider.chat(
                messages.clone(),
                &config.model,
                config.temperature,
                config.max_tokens,
                &tool_defs,
            ).await {
                Ok(r) => r,
                Err(e) => {
                    error!("prompt_async: provider error: {}", e);
                    break;
                }
            };

            if !response.content.is_empty() {
                full_response.push_str(&response.content);
            }

            if response.tool_calls.is_empty() {
                break;
            }

            let mut asst_msg = ProviderMessage::new(ProviderMessageRole::Assistant, &response.content);
            asst_msg.tool_calls = response.tool_calls.clone();
            messages.push(asst_msg);

            for call in &response.tool_calls {
                let tool_name = &call.function.name;
                let args: serde_json::Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Object(Default::default()));

                let result = match tools.get(tool_name) {
                    None => format!("Error: tool '{}' not found", tool_name),
                    Some(tool) => match tool.execute(args).await {
                        Ok(r) => r.output,
                        Err(e) => format!("Error: {e}"),
                    },
                };
                messages.push(ProviderMessage::tool(result, call.id.clone()));
            }
        }

        if !full_response.is_empty() {
            if let Err(e) = session_manager.add_message(&session_id, "assistant", &full_response).await {
                error!("prompt_async: failed to store response: {}", e);
            }
        }
        active_sessions.remove(&session_id);
    });

    Ok(StatusCode::NO_CONTENT)
}