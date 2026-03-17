//! CLI commands

pub mod tui;

use anyhow::Result;
use crate::server::{self, ServerConfig};

/// Run command - start server and TUI
pub async fn run(cwd: Option<String>) -> Result<()> {
    if let Some(cwd) = cwd {
        println!("Working directory: {}", cwd);
    }
    
    let server_config = ServerConfig::default();
    let server_addr = format!("{}:{}", server_config.host, server_config.port);
    
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server::start(server_config).await {
            eprintln!("Server error: {}", e);
        }
    });
    
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    println!("Server started at http://{}", server_addr);
    println!("Starting TUI...");
    
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
