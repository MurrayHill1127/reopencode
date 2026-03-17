pub mod config;
pub mod provider;
pub mod session;

use axum::Json;
use serde_json::json;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}