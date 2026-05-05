//! ReOpenCode (ROC) - Rust rewrite of opencode + oh-my-openagent
#![allow(dead_code, unused_variables, unused_imports)]

mod agent;
mod bus;
mod git;
mod snapshot;
mod category;
mod cli;
mod command;
mod config;
mod hook;
mod mcp;
mod provider;
mod pty;
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
    // Parse CLI arguments
    let args = cli::Args::parse();

    // Initialize logging (skip for generate command since it outputs to stdout)
    if !matches!(args.command, Some(cli::Commands::Generate)) {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("reopencode=info".parse().unwrap()),
            )
            .init();

        tracing::info!("ReOpenCode v{}", VERSION);
    }

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
        Some(cli::Commands::Mcp { command }) => {
            cli::commands::mcp_run(command).await?;
        }
        Some(cli::Commands::Import { file }) => {
            cli::commands::import_run(file).await?;
        }
        Some(cli::Commands::Export { session_id }) => {
            cli::commands::export_run(session_id).await?;
        }
        Some(cli::Commands::Db { sql, command }) => {
            cli::commands::db_run(sql, command).await?;
        }
        Some(cli::Commands::Generate) => {
            cli::commands::generate_run().await?;
        }
        None => {
            // Default: start interactive session
            cli::commands::run(None).await?;
        }
    }

    Ok(())
}
