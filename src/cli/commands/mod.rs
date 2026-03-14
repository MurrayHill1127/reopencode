//! CLI commands

pub mod tui;

use anyhow::Result;

/// Run command - start TUI interactive session
pub async fn run(cwd: Option<String>) -> Result<()> {
    if let Some(cwd) = cwd {
        println!("Working directory: {}", cwd);
    }
    println!("Starting TUI session...");
    tui::run()?;
    Ok(())
}

/// Serve command - start HTTP server
pub async fn serve(port: u16) -> Result<()> {
    println!("Starting HTTP server on port {}...", port);
    Ok(())
}

/// Version command
pub fn version() {
    println!("ReOpenCode v{}", env!("CARGO_PKG_VERSION"));
}
