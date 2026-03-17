//! HTTP Server module - RESTful API using axum

pub mod handlers;
pub mod routes;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::agent::Sisyphus;
use crate::bus::Bus;
use crate::provider::{OpenAiProvider, Provider, ProviderConfig};
use crate::session::SessionManager;
use crate::storage::path::GlobalPath;

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

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub provider: Arc<dyn Provider>,
    pub agent: Arc<Sisyphus>,
    pub session_manager: SessionManager,
    pub bus: Arc<Bus>,
}

impl AppState {
    /// Create a new AppState with Kimi provider configuration
    pub fn new_kimi(api_key: impl Into<String>, session_manager: SessionManager, directory: impl Into<String>) -> Self {
        let config = ProviderConfig::new("kimi", api_key.into())
            .with_base_url("https://api.moonshot.cn/v1");
        
        let provider = Arc::new(OpenAiProvider::new(config));
        let agent = Arc::new(
            Sisyphus::new(Arc::clone(&provider) as Arc<dyn Provider>)
                .with_model("moonshot-v1-8k")
                .with_temperature(0.7),
        );
        
        let bus = Arc::new(Bus::new(directory));

        Self { provider, agent, session_manager, bus }
    }
}

pub fn build_app(state: AppState) -> Router {
    create_router()
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn start(config: ServerConfig) -> anyhow::Result<()> {
    let api_key = std::env::var("KIMI_API_KEY")
        .map_err(|_| anyhow::anyhow!("KIMI_API_KEY environment variable not set"))?;
    
    let global_path = GlobalPath::get();
    global_path.init().await?;
    let db_path = global_path.database_path("latest");
    let database_url = format!("sqlite:{}", db_path.display());
    
    let session_manager = SessionManager::new(&database_url).await
        .map_err(|e| anyhow::anyhow!("Failed to initialize session manager: {}", e))?;
    
    let directory = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string());
    
    let state = AppState::new_kimi(api_key, session_manager, directory);
    let app = build_app(state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}