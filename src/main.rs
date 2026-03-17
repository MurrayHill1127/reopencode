//! ReOpenCode (ROC) - Rust rewrite of opencode + oh-my-openagent

mod agent;
mod bus;
mod category;
mod cli;
mod command;
mod config;
mod hook;
mod provider;
mod server;
mod session;
mod skill;
mod storage;
mod tool;
mod util;

use anyhow::Result;
use clap::Parser;

/// ROC version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("reopencode=info".parse().unwrap()),
        )
        .init();
    
    tracing::info!("ReOpenCode v{}", VERSION);
    
    // Parse CLI arguments
    let args = cli::Args::parse();
    
    // Execute command
    match args.command {
        Some(cli::Commands::Run { cwd }) => {
            cli::commands::run(cwd).await?;
        }
        Some(cli::Commands::Serve { port }) => {
            cli::commands::serve(port).await?;
        }
        Some(cli::Commands::Version) => {
            cli::commands::version();
        }
        None => {
            // Default: start interactive session
            cli::commands::run(None).await?;
        }
    }
    
    Ok(())
}
