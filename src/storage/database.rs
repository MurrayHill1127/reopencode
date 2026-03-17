//! SQLite database management
//!
//! Provides connection pool, transactions, and database initialization.
//! Corresponds to TypeScript: Database namespace (storage/db.ts)

use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use crate::storage::{DatabaseError, StorageError};

/// Database manager
/// 
/// Manages SQLite connection pool with optimized settings for ROC.
pub struct Database {
    pool: SqlitePool,
    path: PathBuf,
}

impl Database {
    /// Open database at the given path
    /// 
    /// Creates the database file if it doesn't exist.
    /// Configures WAL mode and optimized PRAGMA settings.
    pub async fn open(path: &Path) -> Result<Self, DatabaseError> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;
        }
        
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5))
            .foreign_keys(true);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        
        // Set performance PRAGMAs
        sqlx::query("PRAGMA cache_size = -64000")
            .execute(&pool)
            .await?;
        
        sqlx::query("PRAGMA wal_checkpoint(PASSIVE)")
            .execute(&pool)
            .await?;
        
        Ok(Self {
            pool,
            path: path.to_path_buf(),
        })
    }

    /// Get the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Get the database file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Execute a query
    pub async fn execute(&self, sql: &str) -> Result<(), DatabaseError> {
        sqlx::query(sql).execute(&self.pool).await?;
        Ok(())
    }

    /// Check if a table exists
    pub async fn table_exists(&self, name: &str) -> Result<bool, DatabaseError> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result.is_some())
    }

    /// Get current schema version
    pub async fn schema_version(&self) -> Result<i32, DatabaseError> {
        if !self.table_exists("_migrations").await? {
            return Ok(0);
        }
        
        let result: Option<(Option<i32>,)> = sqlx::query_as(
            "SELECT MAX(version) FROM _migrations"
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(result.and_then(|r| r.0).unwrap_or(0))
    }

    /// Record a migration
    pub async fn record_migration(&self, version: i32, name: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO _migrations (version, name, applied_at) VALUES (?, ?, datetime('now'))"
        )
        .bind(version)
        .bind(name)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

impl From<DatabaseError> for StorageError {
    fn from(err: DatabaseError) -> Self {
        StorageError::Database(err)
    }
}

/// SQL migrations for creating tables
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial", include_str!("migrations/001_initial.sql")),
];

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_open() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        
        let db = Database::open(&db_path).await.unwrap();
        assert!(db_path.exists());
        assert_eq!(db.path(), db_path);
        
        db.close().await;
    }

    #[tokio::test]
    async fn test_database_table_exists() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        
        let db = Database::open(&db_path).await.unwrap();
        
        assert!(!db.table_exists("nonexistent").await.unwrap());
        
        db.execute("CREATE TABLE test_table (id INTEGER PRIMARY KEY)").await.unwrap();
        assert!(db.table_exists("test_table").await.unwrap());
        
        db.close().await;
    }

    #[tokio::test]
    async fn test_database_schema_version() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        
        let db = Database::open(&db_path).await.unwrap();
        
        // No migrations table yet
        let version = db.schema_version().await.unwrap();
        assert_eq!(version, 0);
        
        db.close().await;
    }

    #[tokio::test]
    async fn test_database_record_migration() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");
        
        let db = Database::open(&db_path).await.unwrap();
        
        // Create migrations table first
        db.execute(
            "CREATE TABLE _migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL
            )"
        ).await.unwrap();
        
        db.record_migration(1, "initial").await.unwrap();
        
        let version = db.schema_version().await.unwrap();
        assert_eq!(version, 1);
        
        db.close().await;
    }
}