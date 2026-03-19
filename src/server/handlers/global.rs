use crate::bus::GlobalBus;
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
    let mut rx = GlobalBus::subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(global_event) => {
                    yield Ok::<_, Infallible>(Event::default()
                        .data(serde_json::to_string(&SseEvent {
                            directory: global_event.directory,
                            payload: SsePayload {
                                event_type: global_event.payload.event_type,
                                properties: serde_json::json!({}),
                            },
                        }).unwrap()));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new().interval(std::time::Duration::from_secs(15)),
    )
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
