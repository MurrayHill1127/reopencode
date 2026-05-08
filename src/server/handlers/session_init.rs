use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::provider::{Message as ProviderMessage, MessageRole as ProviderMessageRole};
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub struct InitRequest {
    pub model_id: String,
    pub provider_id: String,
    pub message_id: String,
}

#[derive(Debug, Serialize)]
pub struct InitResponse {
    pub success: bool,
    pub context: Option<String>,
}

/// Generate project context using AI analysis of the directory structure.
///
/// POST /session/{id}/init
/// Scans the project directory for key files, builds a prompt describing the
/// project structure, and returns an AI-generated context summary suitable for
/// AGENTS.md / CLAUDE.md generation.
pub async fn init(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<InitRequest>,
) -> Result<Json<InitResponse>, StatusCode> {
    info!("Initializing session: {} with model {}/{}", id, body.provider_id, body.model_id);

    let _session = state.session_manager.get_session(&id).await.map_err(|e| {
        if e.is_not_found() { StatusCode::NOT_FOUND } else { StatusCode::INTERNAL_SERVER_ERROR }
    })?;

    // Scan the project directory for key files
    let cwd = &state.cwd;
    let mut project_info = String::new();
    project_info.push_str(&format!("Working directory: {}\n\n", cwd));

    // List key files
    if let Ok(entries) = std::fs::read_dir(cwd) {
        let mut files: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') || name == ".gitignore" || name == ".env.example" {
                let ftype = if entry.path().is_dir() { "/" } else { "" };
                files.push(format!("  {}{}", name, ftype));
            }
        }
        files.sort();
        let top_level = files.iter().take(30).cloned().collect::<Vec<_>>().join("\n");
        project_info.push_str(&format!("Top-level files:\n{}\n\n", top_level));
    }

    // Check for language-specific indicators
    let indicators: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust project"),
        ("package.json", "Node.js/TypeScript project"),
        ("go.mod", "Go project"),
        ("requirements.txt", "Python project"),
        ("pyproject.toml", "Python project"),
        ("Makefile", "Has build system"),
        ("Dockerfile", "Has Docker configuration"),
        (".github/", "Has CI/CD"),
    ];
    for (file, label) in indicators {
        if std::path::Path::new(&format!("{}/{}", cwd, file)).exists() {
            project_info.push_str(&format!("- {}\n", label));
        }
    }

    // Generate context via provider
    let prompt = format!(
        "You are analyzing a software project. Based on the following directory scan, \
         write a concise project context summary (5-10 lines) covering: \
         what the project does, its tech stack, how to build/run it, and key conventions.\n\n{}",
        project_info
    );

    let context = match state.provider.chat(
        vec![ProviderMessage::new(ProviderMessageRole::User, &prompt)],
        &body.model_id,
        0.3,
        Some(512),
        &[],
    ).await {
        Ok(response) => Some(response.content),
        Err(e) => {
            tracing::warn!("Provider error generating init context: {:?}", e);
            Some(project_info)
        }
    };

    Ok(Json(InitResponse { success: true, context }))
}
