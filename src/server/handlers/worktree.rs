//! Worktree HTTP handlers — manage isolated git worktrees for sessions.

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::server::AppState;
use crate::worktree as wt;

#[derive(Debug, Serialize)]
pub struct WorktreeResponse {
    pub name: String,
    pub branch: String,
    pub directory: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorktreeRequest {
    pub name: Option<String>,
    pub session_id: Option<String>,
}

/// List all worktrees.
pub async fn list(State(state): State<AppState>) -> Json<Vec<WorktreeResponse>> {
    let repo = std::path::Path::new(&state.cwd);
    let trees = wt::list(repo).await.unwrap_or_default();
    let result: Vec<_> = trees
        .into_iter()
        .map(|info| WorktreeResponse {
            name: info.name,
            branch: info.branch,
            directory: info.directory.display().to_string(),
        })
        .collect();
    Json(result)
}

/// Create a new worktree.
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<CreateWorktreeRequest>,
) -> Result<Json<WorktreeResponse>, StatusCode> {
    let repo = std::path::Path::new(&state.cwd);
    let info = wt::make_info(&wt::worktree_root("reopencode"), body.name.as_deref(), repo)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    wt::create(&info, repo)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(WorktreeResponse {
        name: info.name,
        branch: info.branch,
        directory: info.directory.display().to_string(),
    }))
}

/// Remove a worktree by name.
pub async fn remove(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<bool>, StatusCode> {
    let repo = std::path::Path::new(&state.cwd);
    let dir = wt::worktree_root("reopencode").join(&name);
    let removed = wt::remove(&dir, repo).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(removed))
}
