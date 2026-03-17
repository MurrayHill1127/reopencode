use axum::Json;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct VcsInfo {
    pub branch: Option<String>,
    pub is_git_repo: bool,
}

/// GET /vcs - Return git/VCS information
pub async fn get() -> Json<VcsInfo> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    
    let is_git_repo = is_inside_git_repo(&cwd);
    let branch = if is_git_repo {
        get_current_branch(&cwd)
    } else {
        None
    };

    Json(VcsInfo {
        branch,
        is_git_repo,
    })
}

fn is_inside_git_repo(dir: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|output| {
            output.status.success() 
                && String::from_utf8_lossy(&output.stdout).trim() == "true"
        })
        .unwrap_or(false)
}

fn get_current_branch(dir: &Path) -> Option<String> {
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