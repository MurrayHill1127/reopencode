use axum::Json;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct LspStatus {
    pub id: String,
    pub name: String,
    pub root: String,
    pub status: String, // "connected" | "error"
}

pub async fn list() -> Json<Vec<LspStatus>> {
    // For now, return empty list (stub)
    // Future: track actual LSP connections
    Json(Vec::new())
}
