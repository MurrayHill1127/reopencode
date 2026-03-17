use axum::Json;
use serde_json::json;

use crate::config::Config;

pub async fn get() -> Json<serde_json::Value> {
    match Config::load() {
        Ok(config) => Json(serde_json::to_value(config).unwrap_or_else(|_| json!({}))),
        Err(_) => Json(json!({})),
    }
}