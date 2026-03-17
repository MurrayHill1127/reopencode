use axum::{
    extract::Json,
    response::sse::{Event, Sse},
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SseEvent {
    pub directory: String,
    pub payload: SsePayload,
}

#[derive(Debug, Serialize)]
pub struct SsePayload {
    #[serde(rename = "type")]
    pub event_type: String,
    pub properties: serde_json::Value,
}

pub async fn health() -> Json<HealthStatus> {
    Json(HealthStatus {
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn event() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = futures::stream::iter(vec![Ok(Event::default().data(
        serde_json::to_string(&SseEvent {
            directory: "/".to_string(),
            payload: SsePayload {
                event_type: "server.connected".to_string(),
                properties: serde_json::json!({}),
            },
        })
        .unwrap(),
    ))]);
    Sse::new(stream)
}

pub async fn config_get() -> Json<GlobalConfig> {
    Json(GlobalConfig::default())
}

pub async fn config_patch(Json(body): Json<GlobalConfig>) -> Json<GlobalConfig> {
    Json(body)
}

pub async fn dispose() -> Json<bool> {
    Json(true)
}