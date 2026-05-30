pub mod agent;
pub mod auth;
pub mod command;
pub mod config;
pub mod file;
pub mod global;
pub mod skill;
pub mod lsp;
pub mod mcp;
pub mod path;
pub mod permission;
pub mod project;
pub mod provider;
pub mod pty;
pub mod question;
pub mod session;
pub mod session_abort;
pub mod session_children;
pub mod session_diff;
pub mod session_fork;
pub mod session_init;
pub mod session_message;
pub mod session_revert;
pub mod session_share;
pub mod session_status;
pub mod session_summarize;
pub mod session_todo;
pub mod session_command;
pub mod session_permission_reply;
pub mod session_prompt_async;
pub mod session_shell;
pub mod session_unrevert;
pub mod session_undo;
pub mod tui;
pub mod vcs;
pub mod worktree;

use axum::Json;
use serde_json::json;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
