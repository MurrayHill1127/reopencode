//! Database management commands
//!
//! Provides CLI commands for database operations:
//! - `db [query]` - Execute SQL query or open sqlite3 shell
//! - `db path` - Show database file path
//! - `db migrate` - Run schema migrations

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use std::process::Command;

use crate::storage::{Database, GlobalPath, MigrationRunner};

/// DB subcommands
#[derive(Subcommand, Debug)]
pub enum DbCommands {
    /// Execute SQL query or open sqlite3 shell (default)
    #[command(visible_alias = "query")]
    Query {
        /// SQL query to execute
        query: Option<String>,
    },

    /// Show database file path
    Path,

    /// Run schema migrations
    Migrate,
}

/// Run DB command
pub async fn run(sql: Option<String>, command: Option<DbCommands>) -> Result<()> {
    if let Some(query) = sql {
        return cmd_query(Some(query)).await;
    }

    match command {
        Some(DbCommands::Query { query }) => cmd_query(query).await,
        Some(DbCommands::Path) => cmd_path().await,
        Some(DbCommands::Migrate) => cmd_migrate().await,
        None => cmd_query(None).await,
    }
}

/// Execute SQL query or open sqlite3 shell
async fn cmd_query(query: Option<String>) -> Result<()> {
    let db_path = GlobalPath::get().database_path("latest");

    if let Some(sql) = query {
        // Execute query directly
        execute_query(&db_path, &sql).await
    } else {
        // Open sqlite3 shell
        open_sqlite_shell(&db_path)
    }
}

/// Execute a SQL query and display results
async fn execute_query(db_path: &std::path::Path, sql: &str) -> Result<()> {
    let db = Database::open(db_path)
        .await
        .context("Failed to open database")?;

    // For queries that return data
    if sql.trim().to_uppercase().starts_with("SELECT")
        || sql.trim().to_uppercase().starts_with("PRAGMA")
    {
        // Use raw connection for flexible query
        let pool = db.pool();
        let rows: Vec<(i32,)> = sqlx::query_as(sql)
            .fetch_all(pool)
            .await
            .context("Query execution failed")?;

        for row in rows {
            println!("{}", row.0);
        }
    } else {
        // Execute non-SELECT statements
        db.execute(sql)
            .await
            .context("Statement execution failed")?;
        println!("OK");
    }

    db.close().await;
    Ok(())
}

/// Open sqlite3 CLI with the database
fn open_sqlite_shell(db_path: &std::path::Path) -> Result<()> {
    // Check if sqlite3 is available
    let sqlite3_check = Command::new("sqlite3").arg("--version").output();

    if sqlite3_check.is_err() {
        bail!(
            "sqlite3 command not found.\n\
             Install sqlite3:\n\
             - Debian/Ubuntu: sudo apt install sqlite3\n\
             - macOS: brew install sqlite3\n\
             - Arch: sudo pacman -S sqlite"
        );
    }

    // Spawn sqlite3 with the database
    let status = Command::new("sqlite3")
        .arg(db_path)
        .status()
        .context("Failed to spawn sqlite3")?;

    if !status.success() {
        bail!("sqlite3 exited with error");
    }

    Ok(())
}

/// Show database file path
async fn cmd_path() -> Result<()> {
    let path = GlobalPath::get().database_path("latest");
    println!("{}", path.display());
    Ok(())
}

/// Run schema migrations
async fn cmd_migrate() -> Result<()> {
    let db_path = GlobalPath::get().database_path("latest");

    println!("Opening database at {}...", db_path.display());

    let db = Database::open(&db_path)
        .await
        .context("Failed to open database")?;

    let runner = MigrationRunner::builtin();

    // Show pending migrations
    let pending = runner.pending(&db).await?;
    if pending.is_empty() {
        println!("No pending migrations. Database is up to date.");
        db.close().await;
        return Ok(());
    }

    println!("Found {} pending migration(s):", pending.len());
    for m in &pending {
        println!("  - {}: {}", m.version, m.name);
    }

    // Run migrations
    println!("\nRunning migrations...");
    runner.run(&db).await.context("Migration failed")?;

    println!("\nMigrations completed successfully.");

    db.close().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_commands_variants() {
        let _ = DbCommands::Query { query: None };
        let _ = DbCommands::Query {
            query: Some("SELECT 1".to_string()),
        };
        let _ = DbCommands::Path;
        let _ = DbCommands::Migrate;
    }

    #[test]
    fn test_db_path_returns_path() {
        let path = GlobalPath::get().database_path("latest");
        assert!(path.to_str().unwrap().contains("opencode.db"));
    }
}
