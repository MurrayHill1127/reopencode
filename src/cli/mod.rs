//! CLI module

pub mod commands;

use clap::{Parser, Subcommand};
use commands::{DbCommands, McpCommands};

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
        #[arg(short, long, default_value = "4096")]
        port: u16,
    },

    /// Show version
    Version,

    /// Manage MCP servers
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Import session from JSON file or share URL
    Import {
        /// Path to JSON file or share URL
        file: String,
    },

    /// Export session as JSON
    Export {
        /// Session ID to export (optional, shows picker if not provided)
        session_id: Option<String>,
    },

    /// Database tools
    Db {
        /// SQL query to execute (opens sqlite3 shell if not provided)
        sql: Option<String>,

        #[command(subcommand)]
        command: Option<DbCommands>,
    },

    /// Generate OpenAPI specification
    Generate,
}
