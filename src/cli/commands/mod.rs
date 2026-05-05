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

/// Run command - start server and TUI (JS frontend preferred, ratatui fallback)
pub async fn run(cwd: Option<String>) -> Result<()> {
    let working_dir = cwd.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let server_config = ServerConfig::default();
    let server_addr = format!("{}:{}", server_config.host, server_config.port);

    let server_handle = tokio::spawn(async move {
        if let Err(e) = server::start(server_config).await {
            eprintln!("Server error: {}", e);
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    // Find tui/index.tsx relative to the binary or current dir
    let tui_entry = {
        let exe = std::env::current_exe().ok();
        let candidates = [
            std::path::PathBuf::from("tui/index.tsx"),
            exe.as_ref()
                .and_then(|p| p.parent())
                .map(|p| p.join("../tui/index.tsx"))
                .unwrap_or_default(),
        ];
        candidates.into_iter().find(|p| p.exists())
    };

    let bun_available = std::process::Command::new("bun")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if bun_available {
        if let Some(entry) = tui_entry {
            let status = std::process::Command::new("bun")
                .arg("run")
                .arg(entry)
                .env("ROC_DIR", &working_dir)
                .status();
            server_handle.abort();
            return match status {
                Ok(s) if s.success() => Ok(()),
                Ok(s) => Err(anyhow::anyhow!("TUI exited with status {}", s)),
                Err(e) => Err(anyhow::anyhow!("Failed to launch TUI: {}", e)),
            };
        }
    }

    // Fallback: ratatui TUI
    eprintln!("Note: bun not found or tui/index.tsx missing — falling back to ratatui TUI");
    eprintln!("      Install bun and run from the reopencode directory for the full experience.");
    println!("Server started at http://{}", server_addr);
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
