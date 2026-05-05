//! HTTP Server module - RESTful API using axum

pub mod handlers;
pub mod openapi;
pub mod routes;

use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::agent::Sisyphus;
use crate::bus::Bus;
use crate::mcp::McpManager;
use crate::permission::PermissionStore;
use crate::provider::{OpenAiProvider, Provider, ProviderConfig};
use crate::session::SessionManager;
use crate::storage::{MessageStore, Storage};
use crate::tool::registry::ToolRegistry;

pub use handlers::permission::PermissionRequest;
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
    pub message_store: MessageStore,
    pub bus: Arc<Bus>,
    pub mcp_manager: Arc<McpManager>,
    pub permission_store: PermissionStore,
    pub tools: Arc<ToolRegistry>,
    pub cwd: String,
}

impl AppState {
    pub fn new_kimi(
        api_key: impl Into<String>,
        session_manager: SessionManager,
        message_store: MessageStore,
        directory: impl Into<String>,
    ) -> Self {
        let cwd = directory.into();
        let config =
            ProviderConfig::new("kimi", api_key.into()).with_base_url("https://api.moonshot.cn/v1");

        let provider = Arc::new(OpenAiProvider::new(config));
        let agent = Arc::new(
            Sisyphus::new(Arc::clone(&provider) as Arc<dyn Provider>)
                .with_model("moonshot-v1-8k")
                .with_temperature(0.7),
        );

        let bus = Arc::new(Bus::new(cwd.clone()));
        let mcp_manager = Arc::new(McpManager::new());
        let tools = Arc::new(build_tool_registry());

        Self {
            provider,
            agent,
            session_manager,
            message_store,
            bus,
            mcp_manager,
            permission_store: PermissionStore::new(),
            tools,
            cwd,
        }
    }
}

fn build_tool_registry() -> ToolRegistry {
    use crate::tool::*;
    let registry = ToolRegistry::new();
    registry.register(Box::new(bash::BashTool::new()));
    registry.register(Box::new(read::ReadTool::new()));
    registry.register(Box::new(write::WriteTool::new()));
    registry.register(Box::new(edit::EditTool::new()));
    registry.register(Box::new(glob::GlobTool::new()));
    registry.register(Box::new(grep::GrepTool::new()));
    registry.register(Box::new(ls::ListTool::new()));
    registry.register(Box::new(todo::TodoWriteTool::new()));
    registry.register(Box::new(todo::TodoReadTool::new()));
    registry.register(Box::new(task::TaskTool::new()));
    registry.register(Box::new(question::QuestionTool::new()));
    registry.register(Box::new(webfetch::WebFetchTool::new()));
    registry
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

    let storage = Storage::new(crate::storage::BackendType::Json).await?;
    let message_store = storage.messages();

    let global_path = crate::storage::path::GlobalPath::get();
    global_path.init().await?;
    let db_path = global_path.database_path("latest");
    let database_url = format!("sqlite:{}", db_path.display());

    let session_manager = SessionManager::new(&database_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize session manager: {}", e))?;

    let directory = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "/".to_string());

    let state = AppState::new_kimi(api_key, session_manager, message_store, directory);
    let app = build_app(state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
