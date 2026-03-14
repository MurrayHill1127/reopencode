//! ReOpenCode (ROC) - Rust rewrite of opencode + oh-my-openagent
//!
//! A high-performance AI coding assistant with plugin support.

pub mod agent;
pub mod category;
pub mod cli;
pub mod command;
pub mod config;
pub mod hook;
pub mod provider;
pub mod session;
pub mod skill;
pub mod storage;
pub mod tool;
pub mod util;

use anyhow::Result;

/// ROC version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize ROC
pub async fn init() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("reopencode=info".parse().unwrap()),
        )
        .init();
    
    tracing::info!("ReOpenCode v{}", VERSION);
    tracing::info!("Initializing...");
    
    Ok(())
}
