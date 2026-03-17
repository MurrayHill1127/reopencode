use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::pty::{self, PtyInfo, CreatePtyRequest, UpdatePtyRequest, PtySize};
use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PtyInfoResponse {
    pub id: String,
    pub title: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: String,
    pub pid: u32,
}

impl From<PtyInfo> for PtyInfoResponse {
    fn from(info: PtyInfo) -> Self {
        Self {
            id: info.id,
            title: info.title,
            command: info.command,
            args: info.args,
            cwd: info.cwd,
            status: info.status.to_string(),
            pid: info.pid,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePtyBody {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
    #[serde(default)]
    pub rows: Option<u16>,
    #[serde(default)]
    pub cols: Option<u16>,
}

impl From<CreatePtyBody> for CreatePtyRequest {
    fn from(body: CreatePtyBody) -> Self {
        Self {
            command: body.command,
            args: body.args,
            cwd: body.cwd,
            title: body.title,
            env: body.env,
            rows: body.rows,
            cols: body.cols,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePtyBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub size: Option<PtySizeBody>,
}

#[derive(Debug, Deserialize)]
pub struct PtySizeBody {
    pub rows: u16,
    pub cols: u16,
}

impl From<UpdatePtyBody> for UpdatePtyRequest {
    fn from(body: UpdatePtyBody) -> Self {
        Self {
            title: body.title,
            size: body.size.map(|s| PtySize {
                rows: s.rows,
                cols: s.cols,
            }),
        }
    }
}

/// GET /pty - List PTY sessions
pub async fn list() -> Json<Vec<PtyInfoResponse>> {
    let manager = pty::global();
    let sessions = manager.list();
    Json(sessions.into_iter().map(PtyInfoResponse::from).collect())
}

/// POST /pty - Create PTY session
pub async fn create(
    State(_state): State<AppState>,
    Json(body): Json<CreatePtyBody>,
) -> Result<Json<PtyInfoResponse>, StatusCode> {
    let manager = pty::global();
    let req: CreatePtyRequest = body.into();
    
    match manager.create(req).await {
        Ok(info) => Ok(Json(PtyInfoResponse::from(info))),
        Err(e) => {
            tracing::error!("Failed to create PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /pty/:id - Get PTY session
pub async fn get(Path(id): Path<String>) -> Result<Json<PtyInfoResponse>, StatusCode> {
    let manager = pty::global();
    
    match manager.get(&id).await {
        Some(info) => Ok(Json(PtyInfoResponse::from(info))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// PUT /pty/:id - Update PTY session
pub async fn update(
    Path(id): Path<String>,
    Json(body): Json<UpdatePtyBody>,
) -> Result<Json<PtyInfoResponse>, StatusCode> {
    let manager = pty::global();
    let req: UpdatePtyRequest = body.into();
    
    match manager.update(&id, req).await {
        Ok(info) => Ok(Json(PtyInfoResponse::from(info))),
        Err(pty::PtyError::NotFound(_)) => Err(StatusCode::NOT_FOUND),
        Err(pty::PtyError::AlreadyExited(_)) => Err(StatusCode::GONE),
        Err(e) => {
            tracing::error!("Failed to update PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /pty/:id - Remove PTY session
pub async fn remove(Path(id): Path<String>) -> Result<Json<bool>, StatusCode> {
    let manager = pty::global();
    
    match manager.remove(&id).await {
        Ok(result) => Ok(Json(result)),
        Err(pty::PtyError::NotFound(_)) => Ok(Json(true)),
        Err(e) => {
            tracing::error!("Failed to remove PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /pty/:id/connect - WebSocket connection (stub)
pub async fn connect(Path(id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    let manager = pty::global();
    
    if manager.get(&id).await.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    Err(StatusCode::UPGRADE_REQUIRED)
}

/// POST /pty/:id/resize - Resize PTY session
pub async fn resize(
    Path(id): Path<String>,
    Json(size): Json<PtySizeBody>,
) -> Result<Json<bool>, StatusCode> {
    let manager = pty::global();
    
    match manager.resize(&id, size.rows, size.cols).await {
        Ok(()) => Ok(Json(true)),
        Err(pty::PtyError::NotFound(_)) => Err(StatusCode::NOT_FOUND),
        Err(pty::PtyError::AlreadyExited(_)) => Err(StatusCode::GONE),
        Err(e) => {
            tracing::error!("Failed to resize PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /pty/:id/write - Write to PTY session
pub async fn write(
    Path(id): Path<String>,
    Json(body): Json<WriteBody>,
) -> Result<Json<bool>, StatusCode> {
    let manager = pty::global();
    
    match manager.write(&id, body.data.as_bytes()).await {
        Ok(()) => Ok(Json(true)),
        Err(pty::PtyError::NotFound(_)) => Err(StatusCode::NOT_FOUND),
        Err(pty::PtyError::AlreadyExited(_)) => Err(StatusCode::GONE),
        Err(e) => {
            tracing::error!("Failed to write to PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WriteBody {
    pub data: String,
}

/// GET /pty/:id/read - Read PTY output
pub async fn read(
    Path(id): Path<String>,
) -> Result<Json<pty::PtyOutput>, StatusCode> {
    let manager = pty::global();
    
    match manager.read(&id, None).await {
        Ok(output) => Ok(Json(output)),
        Err(pty::PtyError::NotFound(_)) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to read PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /pty/:id/kill - Kill PTY session
pub async fn kill(Path(id): Path<String>) -> Result<Json<bool>, StatusCode> {
    let manager = pty::global();
    
    match manager.kill(&id).await {
        Ok(()) => Ok(Json(true)),
        Err(pty::PtyError::NotFound(_)) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to kill PTY: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}