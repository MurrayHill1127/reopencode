//! CLI commands

use anyhow::Result;

/// Run command - start interactive session
pub async fn run(cwd: Option<String>) -> Result<()> {
    println!("Starting interactive session...");
    if let Some(cwd) = cwd {
        println!("Working directory: {}", cwd);
    }
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
