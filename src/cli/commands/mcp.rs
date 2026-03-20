//! MCP server management commands
//!
//! Provides CLI commands for managing MCP servers:
//! - `mcp list` / `mcp ls` - List all MCP servers with status
//! - `mcp add` - Interactive wizard to add new server
//! - `mcp remove <name>` - Remove a server
//! - `mcp auth` - OAuth authentication
//! - `mcp debug <name>` - Debug connection

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use inquire::{Confirm, Select, Text, validator::Validation};
use std::path::PathBuf;
use toml_edit::{Array, Item, Table, value};

use crate::config::{Config, ConfigPaths, McpConfig};
use crate::mcp::{McpManager, McpStatus};

/// MCP subcommands
#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// List all MCP servers
    #[command(visible_alias = "ls")]
    List,

    /// Add a new MCP server (interactive)
    Add,

    /// Remove an MCP server
    Remove {
        /// Server name to remove
        name: String,
    },

    /// OAuth authentication
    Auth {
        #[command(subcommand)]
        command: Option<AuthCommands>,
    },

    /// Debug MCP server connection
    Debug {
        /// Server name to debug
        name: String,
    },
}

/// Auth subcommands
#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// List OAuth-capable servers
    #[command(visible_alias = "ls")]
    List,

    /// Authenticate with a server
    Login {
        /// Server name
        name: String,
    },

    /// Remove OAuth credentials
    Logout {
        /// Server name
        name: String,
    },
}

/// Run MCP command
pub async fn run(command: McpCommands) -> Result<()> {
    match command {
        McpCommands::List => cmd_list().await,
        McpCommands::Add => cmd_add().await,
        McpCommands::Remove { name } => cmd_remove(&name).await,
        McpCommands::Auth { command } => cmd_auth(command).await,
        McpCommands::Debug { name } => cmd_debug(&name).await,
    }
}

/// List all MCP servers
async fn cmd_list() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;

    if config.mcp.is_empty() {
        println!("No MCP servers configured.");
        println!("Use 'roc mcp add' to add a server.");
        return Ok(());
    }

    println!("MCP Servers:\n");

    // Create manager to get actual status
    let manager = McpManager::new();
    manager.initialize_from_config(&config).await;

    let statuses = manager.status().await;

    // Display table
    println!("{:<20} {:<12} {:<12} Details", "Name", "Type", "Status");
    println!("{}", "-".repeat(60));

    for (name, mcp_config) in &config.mcp {
        let server_type = match mcp_config {
            McpConfig::Local(_) => "local",
            McpConfig::Remote(_) => "remote",
        };

        let status = statuses.get(name).unwrap_or(&McpStatus::Disabled);
        let status_str = match status {
            McpStatus::Connected => "connected",
            McpStatus::Disabled => "disabled",
            McpStatus::Failed { .. } => "failed",
            McpStatus::NeedsAuth => "needs_auth",
            McpStatus::NeedsClientRegistration { .. } => "needs_reg",
        };

        let details = match status {
            McpStatus::Failed { error } | McpStatus::NeedsClientRegistration { error } => {
                if error.len() > 30 {
                    format!("{}...", &error[..27])
                } else {
                    error.clone()
                }
            }
            _ => match mcp_config {
                McpConfig::Local(l) => l.command.clone(),
                McpConfig::Remote(r) => r.url.clone(),
            },
        };

        println!(
            "{:<20} {:<12} {:<12} {}",
            name, server_type, status_str, details
        );
    }

    println!("\n{} server(s)", config.mcp.len());
    Ok(())
}

/// Add new MCP server
async fn cmd_add() -> Result<()> {
    println!("MCP Server Configuration Wizard\n");

    // Step 1: Choose location
    let location = Select::new(
        "Where should this server be configured?",
        vec!["Global (~/.config/roc/roc.toml)", "Project (./roc.toml)"],
    )
    .prompt()?;

    let config_path = match location {
        "Global (~/.config/roc/roc.toml)" => {
            let config_dir = ConfigPaths::global_config_dir();
            std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
            ConfigPaths::global_config_file()
        }
        "Project (./roc.toml)" => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            cwd.join("roc.toml")
        }
        _ => unreachable!(),
    };

    // Step 2: Enter server name
    let name = Text::new("Server name:")
        .with_validator(|input: &str| {
            if input.is_empty() {
                Ok(Validation::Invalid("Name cannot be empty".into()))
            } else if input.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
                Ok(Validation::Invalid(
                    "Name can only contain letters, numbers, -, and _".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;

    // Check if already exists
    let config = Config::load().context("Failed to load config")?;
    if config.mcp.contains_key(&name) {
        let overwrite = Confirm::new(&format!("Server '{}' already exists. Overwrite?", name))
            .with_default(false)
            .prompt()?;
        if !overwrite {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Step 3: Choose server type
    let server_type =
        Select::new("Server type:", vec!["Local (stdio)", "Remote (HTTP)"]).prompt()?;

    // Step 4: Configure based on type
    let config_entry = match server_type {
        "Local (stdio)" => {
            let command = Text::new("Command (e.g., 'node', 'python'):")
                .with_validator(|input: &str| {
                    if input.is_empty() {
                        Ok(Validation::Invalid("Command is required".into()))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()?;

            let args_input = Text::new("Arguments (space-separated, optional):")
                .prompt()
                .unwrap_or_default();

            let args: Vec<String> = args_input
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            // Build TOML table for local config
            let mut table = Table::new();
            table["command"] = value(command);
            if !args.is_empty() {
                let mut arr = Array::new();
                for arg in args {
                    arr.push(arg);
                }
                table["args"] = value(arr);
            }
            table
        }
        "Remote (HTTP)" => {
            let url = Text::new("Server URL:")
                .with_validator(|input: &str| {
                    if input.is_empty() {
                        Ok(Validation::Invalid("URL is required".into()))
                    } else if !input.starts_with("http://") && !input.starts_with("https://") {
                        Ok(Validation::Invalid(
                            "URL must start with http:// or https://".into(),
                        ))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()?;

            let use_custom_timeout = Confirm::new("Set custom timeout?")
                .with_default(false)
                .prompt()?;

            let timeout: u64 = if use_custom_timeout {
                inquire::CustomType::new("Timeout (ms):")
                    .with_default(30000)
                    .prompt()?
            } else {
                30000
            };

            // Build TOML table for remote config
            let mut table = Table::new();
            table["url"] = value(url);
            if timeout != 30000 {
                table["timeout"] = value(timeout as i64);
            }
            table
        }
        _ => unreachable!(),
    };

    // Step 5: Write to config file
    write_mcp_to_config(&config_path, &name, config_entry)?;

    println!("\n✓ MCP server '{}' added successfully!", name);
    println!("  Config file: {}", config_path.display());
    println!("  Run 'roc mcp list' to see status.");

    Ok(())
}

/// Remove MCP server
async fn cmd_remove(name: &str) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;

    if !config.mcp.contains_key(name) {
        bail!("MCP server '{}' not found in config", name);
    }

    let confirmed = Confirm::new(&format!("Remove MCP server '{}'?", name))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    // Find and remove from config file
    // Try project config first, then global
    let config_files = vec![
        std::env::current_dir()?.join("roc.toml"),
        ConfigPaths::global_config_file(),
    ];

    for config_file in &config_files {
        if config_file.exists() && remove_mcp_from_config(config_file, name)? {
            println!("✓ MCP server '{}' removed.", name);
            return Ok(());
        }
    }

    bail!("Could not find server '{}' in any config file", name)
}

/// Handle auth commands
async fn cmd_auth(command: Option<AuthCommands>) -> Result<()> {
    match command {
        Some(AuthCommands::List) => cmd_auth_list().await,
        Some(AuthCommands::Login { name }) => cmd_auth_login(&name).await,
        Some(AuthCommands::Logout { name }) => cmd_auth_logout(&name).await,
        None => {
            println!("Usage: roc mcp auth <subcommand>");
            println!("\nSubcommands:");
            println!("  list    - List OAuth-capable servers");
            println!("  login   - Authenticate with a server");
            println!("  logout  - Remove OAuth credentials");
            Ok(())
        }
    }
}

/// List OAuth-capable servers
async fn cmd_auth_list() -> Result<()> {
    let config = Config::load().context("Failed to load config")?;

    println!("OAuth Authentication Status:\n");

    // Show all remote servers as potential OAuth candidates
    let remote_servers: Vec<_> = config
        .mcp
        .iter()
        .filter(|(_, c)| matches!(c, McpConfig::Remote(_)))
        .collect();

    if remote_servers.is_empty() {
        println!("No remote MCP servers configured.");
        return Ok(());
    }

    println!("{:<20} {:<15}", "Server", "Auth Status");
    println!("{}", "-".repeat(40));

    for (name, _) in remote_servers {
        // TODO: Check actual auth status when OAuth is implemented
        println!("{:<20} {:<15}", name, "unknown");
    }

    println!("\nNote: OAuth authentication is not yet implemented.");
    println!("This feature will be added in a future release.");

    Ok(())
}

/// Login to OAuth server
async fn cmd_auth_login(name: &str) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;

    if !config.mcp.contains_key(name) {
        bail!("MCP server '{}' not found", name);
    }

    // TODO: Implement OAuth flow
    bail!(
        "OAuth authentication is not yet implemented for server '{}'.\n\
         This feature will be added in a future release.",
        name
    );
}

/// Logout from OAuth server
async fn cmd_auth_logout(name: &str) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;

    if !config.mcp.contains_key(name) {
        bail!("MCP server '{}' not found", name);
    }

    // TODO: Implement OAuth logout
    bail!(
        "OAuth logout is not yet implemented for server '{}'.\n\
         This feature will be added in a future release.",
        name
    );
}

/// Debug MCP server connection
async fn cmd_debug(name: &str) -> Result<()> {
    let config = Config::load().context("Failed to load config")?;

    let mcp_config = config
        .mcp
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("MCP server '{}' not found", name))?;

    println!("Debugging MCP server '{}':\n", name);

    // Show config
    println!("Configuration:");
    match mcp_config {
        McpConfig::Local(local) => {
            println!("  Type: local");
            println!("  Command: {} {:?}", local.command, local.args);
            if let Some(env) = &local.env {
                println!("  Environment: {} variables", env.len());
            }
            if let Some(cwd) = &local.cwd {
                println!("  Working Dir: {}", cwd);
            }
        }
        McpConfig::Remote(remote) => {
            println!("  Type: remote");
            println!("  URL: {}", remote.url);
            println!("  Timeout: {}ms", remote.timeout);
        }
    }

    // Try to connect
    println!("\nTesting connection...");
    let manager = McpManager::new();

    match manager.add(name, mcp_config.clone()).await {
        Ok(()) => {
            println!("✓ Connection successful!");

            // Try to get server info
            if let Some(client) = manager.get_client(name).await {
                let client = client.read().await;
                if let Ok(Some(info)) = client.get_server_info().await {
                    println!("\nServer Info:");
                    println!("  Name: {}", info.name);
                    println!("  Version: {}", info.version);
                    println!("  Protocol: {}", info.protocol_version);
                }

                // Try to list tools
                match client.list_tools().await {
                    Ok(tools) => {
                        println!("\nAvailable Tools ({}):", tools.len());
                        for tool in tools.iter().take(5) {
                            println!("  - {}", tool.name);
                        }
                        if tools.len() > 5 {
                            println!("  ... and {} more", tools.len() - 5);
                        }
                    }
                    Err(e) => {
                        println!("  Failed to list tools: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Connection failed: {}", e);
        }
    }

    Ok(())
}

/// Write MCP config to file
fn write_mcp_to_config(path: &PathBuf, name: &str, config_entry: Table) -> Result<()> {
    let content = if path.exists() {
        std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?
    } else {
        String::new()
    };

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .context("Failed to parse config file")?;

    // Get or create [mcp] table
    if doc.get("mcp").is_none() {
        let mut mcp_table = Table::new();
        mcp_table.set_implicit(true);
        doc["mcp"] = Item::Table(mcp_table);
    }

    // Add server config
    doc["mcp"][name] = Item::Table(config_entry);

    // Write back
    std::fs::write(path, doc.to_string())
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

/// Remove MCP server from config file
fn remove_mcp_from_config(path: &PathBuf, name: &str) -> Result<bool> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .context("Failed to parse config file")?;

    if let Some(mcp) = doc.get_mut("mcp").and_then(|item| item.as_table_mut())
        && mcp.remove(name).is_some()
    {
        std::fs::write(path, doc.to_string())
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_commands_variants() {
        // Test that all command variants can be created
        let _ = McpCommands::List;
        let _ = McpCommands::Add;
        let _ = McpCommands::Remove {
            name: String::from("test"),
        };
        let _ = McpCommands::Debug {
            name: String::from("test"),
        };
    }

    #[test]
    fn test_auth_commands_variants() {
        let _ = AuthCommands::List;
        let _ = AuthCommands::Login {
            name: String::from("test"),
        };
        let _ = AuthCommands::Logout {
            name: String::from("test"),
        };
    }

    #[test]
    fn test_write_mcp_config_local() {
        let mut table = Table::new();
        table["command"] = value("node");
        let mut arr = Array::new();
        arr.push("server.js");
        table["args"] = value(arr);

        assert!(table.get("command").is_some());
        assert!(table.get("args").is_some());
    }

    #[test]
    fn test_write_mcp_config_remote() {
        let mut table = Table::new();
        table["url"] = value("http://localhost:8080");
        table["timeout"] = value(30000i64);

        assert!(table.get("url").is_some());
        assert!(table.get("timeout").is_some());
    }

    #[test]
    fn test_toml_edit_basic() {
        let content = "";
        let doc = content.parse::<toml_edit::DocumentMut>().unwrap();

        // Verify we can create a new document
        assert!(doc.is_empty());
    }
}
