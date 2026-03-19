//! CLI module

pub mod commands;

use clap::{Parser, Subcommand};

/// ReOpenCode (ROC) - AI coding assistant
#[derive(Parser, Debug)]
#[command(name = "roc")]
#[command(author = "ReOpenCode Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "AI coding assistant written in Rust", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start interactive session
    Run {
        /// Working directory
        #[arg(short, long)]
        cwd: Option<String>,
    },

    /// Start HTTP server
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Show version
    Version,
}
