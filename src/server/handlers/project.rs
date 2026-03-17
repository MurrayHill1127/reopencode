use axum::{
    http::StatusCode,
    Json,
};
use serde::Serialize;
use tracing::{error, info};

#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub git_branch: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list() -> Json<Vec<ProjectInfo>> {
    Json(vec![])
}

/// GET /project/current - Return current project info
pub async fn current() -> Result<Json<ProjectInfo>, StatusCode> {
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to get current directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let path_str = cwd.display().to_string();
    let name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git_branch = get_current_git_branch(&cwd);

    info!("Current project: {} at {}", name, path_str);

    Ok(Json(ProjectInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        path: path_str,
        git_branch,
        created_at: chrono::Utc::now(),
    }))
}

/// POST /project/git/init - Initialize git repo in current directory
pub async fn git_init() -> Result<Json<serde_json::Value>, StatusCode> {
    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to get current directory: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if cwd.join(".git").exists() {
        return Ok(Json(serde_json::json!({
            "success": false,
            "message": "Already a git repository"
        })));
    }

    let output = match std::process::Command::new("git")
        .arg("init")
        .current_dir(&cwd)
        .output()
    {
        Ok(output) => output,
        Err(e) => {
            error!("Failed to run git init: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if output.status.success() {
        info!("Initialized git repository in {:?}", cwd);
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "Git repository initialized"
        })))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("git init failed: {}", stderr);
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

fn get_current_git_branch(dir: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Some(branch);
        }
    }
    None
}