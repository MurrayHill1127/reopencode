pub mod command;
pub mod config;
pub mod file;
pub mod global;
pub mod mcp;
pub mod path;
pub mod permission;
pub mod project;
pub mod provider;
pub mod pty;
pub mod question;
pub mod session;
pub mod tui;
pub mod vcs;

use axum::Json;
use serde_json::json;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}