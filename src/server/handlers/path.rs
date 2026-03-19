use axum::{Json, http::StatusCode};
use serde::Serialize;
use tracing::error;

#[derive(Debug, Serialize)]
pub struct PathInfo {
    pub home: String,
    pub state: String,
    pub config: String,
    pub directory: String,
}

/// GET /path - Return path information
pub async fn get() -> Result<Json<PathInfo>, StatusCode> {
    let home = dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "/".to_string());

    let cwd = match std::env::current_dir() {
        Ok(path) => path.display().to_string(),
        Err(e) => {
            error!("Failed to get current directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let state = format!("{}/.local/share/opencode", home);
    let config = format!("{}/.config/opencode", home);

    Ok(Json(PathInfo {
        home,
        state,
        config,
        directory: cwd,
    }))
}
