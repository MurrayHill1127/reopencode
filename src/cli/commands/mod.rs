//! CLI commands

pub mod clipboard;
pub mod db;
pub mod export;
pub mod generate;
pub mod import;
pub mod mcp;
pub mod tui;

use crate::server::{self, ServerConfig};
use anyhow::Result;

pub use db::{DbCommands, run as db_run};
pub use export::run as export_run;
pub use generate::run as generate_run;
pub use import::run as import_run;
pub use mcp::{McpCommands, run as mcp_run};

/// Run command - start server and ratatui TUI
pub async fn run(_cwd: Option<String>) -> Result<()> {
    let server_config = ServerConfig::default();
    let server_addr = format!("{}:{}", server_config.host, server_config.port);

    let server_handle = tokio::spawn(async move {
        if let Err(e) = server::start(server_config).await {
            eprintln!("Server error: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    tracing::info!("Server started at http://{}", server_addr);
    tui::run().await?;
    server_handle.abort();
    Ok(())
}

/// Serve command - start HTTP server
pub async fn serve(port: u16) -> Result<()> {
    let config = ServerConfig::new(port, "127.0.0.1".to_string());
    server::start(config).await
}

/// Version command
pub fn version() {
    println!("ReOpenCode v{}", env!("CARGO_PKG_VERSION"));
}
