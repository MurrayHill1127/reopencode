//! LSP status handler — reports active language server connections.

use axum::Json;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct LspStatus {
    pub id: String,
    pub name: String,
    pub root: String,
    pub status: String,
}

/// List active LSP connections.
pub async fn list() -> Json<Vec<LspStatus>> {
    // LSP connections are tracked via LspManager.
    // Currently returns empty until LspManager is integrated into AppState.
    Json(Vec::new())
}
