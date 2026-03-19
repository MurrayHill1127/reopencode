use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommandInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// GET /command - List available commands
pub async fn list() -> Json<Vec<CommandInfo>> {
    Json(vec![
        CommandInfo {
            id: "run".to_string(),
            name: "run".to_string(),
            description: "Run opencode in the current directory".to_string(),
        },
        CommandInfo {
            id: "serve".to_string(),
            name: "serve".to_string(),
            description: "Start the HTTP server".to_string(),
        },
        CommandInfo {
            id: "config".to_string(),
            name: "config".to_string(),
            description: "Manage configuration".to_string(),
        },
        CommandInfo {
            id: "session".to_string(),
            name: "session".to_string(),
            description: "Manage sessions".to_string(),
        },
        CommandInfo {
            id: "help".to_string(),
            name: "help".to_string(),
            description: "Show help information".to_string(),
        },
    ])
}
