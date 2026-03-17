use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    id: String,
    name: String,
    models: Vec<String>,
}

pub async fn list() -> Json<Vec<ProviderInfo>> {
    Json(vec![
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
        },
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            models: vec!["claude-3-opus".to_string(), "claude-3-sonnet".to_string()],
        },
    ])
}