//! HTTP Server module - RESTful API using axum

pub mod handlers;
pub mod routes;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub use routes::create_router;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 4096,
            host: "127.0.0.1".to_string(),
        }
    }
}

impl ServerConfig {
    pub fn new(port: u16, host: String) -> Self {
        Self { port, host }
    }
}

pub fn build_app() -> Router {
    create_router()
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
}

pub async fn start(config: ServerConfig) -> anyhow::Result<()> {
    let app = build_app();
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}