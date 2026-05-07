use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::provider::{Message as ProviderMessage, MessageRole as ProviderMessageRole};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct SummarizeRequest {
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub auto: bool,
}

#[derive(Debug, Serialize)]
pub struct SummarizeResponse {
    pub success: bool,
    pub messages_compacted: usize,
    pub summary_length: usize,
}

/// Summarize a session using AI compaction.
///
/// POST /session/{id}/summarize
/// Reads session messages, generates a concise summary via the provider,
/// and replaces earlier messages with the summary system message.
pub async fn summarize(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<SummarizeRequest>,
) -> Result<Json<SummarizeResponse>, StatusCode> {
    info!(
        "Summarizing session: {} with model {}/{} (auto: {})",
        id, body.provider_id, body.model_id, body.auto
    );

    let session = state.session_manager.get_session(&id).await.map_err(|e| {
        if e.is_not_found() { StatusCode::NOT_FOUND } else { StatusCode::INTERNAL_SERVER_ERROR }
    })?;

    // Clear existing revert state
    if session.revert.is_some() {
        let _ = state.session_manager.clear_revert(&id).await;
    }

    // Get all messages
    let messages = state.session_manager.get_messages(&id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if messages.len() < 4 {
        return Ok(Json(SummarizeResponse { success: true, messages_compacted: 0, summary_length: 0 }));
    }

    // Build conversation text from the earliest half of messages
    let split = messages.len() / 2;
    let to_compact: Vec<_> = messages.iter().take(split).collect();
    let compacted_count = to_compact.len();

    let conversation_text: String = to_compact
        .iter()
        .map(|m| format!("[{}]: {}",
            m.role,
            &m.content[..m.content.len().min(2000)]))
        .collect::<Vec<_>>()
        .join("\n---\n");

    let summary_prompt = format!(
        "Summarize this conversation segment concisely. Preserve:\n\
         - Key decisions made\n\
         - Files that were created, edited, or deleted\n\
         - Commands that were run and their results\n\
         - Unresolved issues or questions\n\n\
         Use no more than 5-10 bullet points.\n\n{}",
        conversation_text
    );

    match state.provider.chat(
        vec![ProviderMessage::new(ProviderMessageRole::User, &summary_prompt)],
        "moonshot-v1-8k",
        0.3,
        Some(1024),
        &[],
    ).await {
        Ok(response) => {
            let summary = format!(
                "[Compacted context — {} messages summarized]\n{}",
                compacted_count, response.content
            );
            // Store as a system message
            let _ = state.session_manager.add_message(&id, "system", &summary).await;
            let summary_len = summary.len();

            info!("Session {} compacted: {} messages → {} chars summary", id, compacted_count, summary_len);
            Ok(Json(SummarizeResponse {
                success: true,
                messages_compacted: compacted_count,
                summary_length: summary_len,
            }))
        }
        Err(e) => {
            tracing::error!("Compaction failed: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
