use axum::{
    extract::{Json, Path},
    http::StatusCode,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static PTY_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(1));

fn generate_pty_id() -> String {
    format!("pty_{:04}", PTY_COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PtyInfo {
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: String,
    pub pid: u32,
}

#[derive(Debug, Deserialize, Default)]
pub struct CreatePtyRequest {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdatePtyRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub size: Option<PtySize>,
}

#[derive(Debug, Deserialize)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

/// GET /pty - List PTY sessions
pub async fn list() -> Json<Vec<PtyInfo>> {
    Json(vec![])
}

/// POST /pty - Create PTY session
pub async fn create(Json(body): Json<CreatePtyRequest>) -> Result<Json<PtyInfo>, StatusCode> {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string());

    Ok(Json(PtyInfo {
        id: generate_pty_id(),
        title: body.title.unwrap_or_else(|| "Terminal".to_string()),
        command: body.command.unwrap_or_else(|| "bash".to_string()),
        args: body.args.unwrap_or_default(),
        cwd: body.cwd.unwrap_or(cwd),
        status: "running".to_string(),
        pid: 12345,
    }))
}

/// GET /pty/:id - Get PTY session
pub async fn get(Path(_id): Path<String>) -> Result<Json<PtyInfo>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

/// PUT /pty/:id - Update PTY session
pub async fn update(
    Path(_id): Path<String>,
    Json(_body): Json<UpdatePtyRequest>,
) -> Result<Json<PtyInfo>, StatusCode> {
    Err(StatusCode::NOT_FOUND)
}

/// DELETE /pty/:id - Remove PTY session
pub async fn remove(Path(_id): Path<String>) -> Result<Json<bool>, StatusCode> {
    Ok(Json(true))
}

/// GET /pty/:id/connect - WebSocket connection (stub)
pub async fn connect(Path(_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    // WebSocket stub - return error indicating WebSocket upgrade needed
    Err(StatusCode::UPGRADE_REQUIRED)
}