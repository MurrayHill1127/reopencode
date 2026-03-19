//! Schema migration system
//!
//! Provides database migration management with version tracking.

use std::sync::Arc;

use crate::storage::{Database, DatabaseError, MigrationError, StorageError};

/// Migration definition
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i32,
    pub name: String,
    pub sql: String,
}

impl Migration {
    pub fn new(version: i32, name: impl Into<String>, sql: impl Into<String>) -> Self {
        Self {
            version,
            name: name.into(),
            sql: sql.into(),
        }
    }
}

/// Migration runner
pub struct MigrationRunner {
    migrations: Vec<Migration>,
}

impl MigrationRunner {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Add a migration
    pub fn add(&mut self, migration: Migration) {
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.version);
    }

    /// Get built-in migrations
    pub fn builtin() -> Self {
        let mut runner = Self::new();

        runner.add(Migration::new(
            1,
            "initial",
            include_str!("migrations/001_initial.sql"),
        ));

        runner
    }

    /// Run all pending migrations
    pub async fn run(&self, db: &Database) -> Result<(), StorageError> {
        let current_version = db.schema_version().await?;

        for migration in &self.migrations {
            if migration.version > current_version {
                self.run_migration(db, migration).await?;
            }
        }

        Ok(())
    }

    /// Run a single migration
    async fn run_migration(
        &self,
        db: &Database,
        migration: &Migration,
    ) -> Result<(), StorageError> {
        tracing::info!(
            "Running migration {}: {}",
            migration.version,
            migration.name
        );

        // Execute the migration SQL
        db.execute(&migration.sql).await?;

        // Record the migration
        db.record_migration(migration.version, &migration.name)
            .await?;

        tracing::info!("Migration {} completed", migration.version);

        Ok(())
    }

    /// Get pending migrations
    pub async fn pending(&self, db: &Database) -> Result<Vec<&Migration>, StorageError> {
        let current_version = db.schema_version().await?;

        Ok(self
            .migrations
            .iter()
            .filter(|m| m.version > current_version)
            .collect())
    }

    /// Get migration count
    pub fn count(&self) -> usize {
        self.migrations.len()
    }
}

impl Default for MigrationRunner {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migration_new() {
        let migration = Migration::new(1, "test", "CREATE TABLE test (id INTEGER);");

        assert_eq!(migration.version, 1);
        assert_eq!(migration.name, "test");
        assert!(migration.sql.contains("CREATE TABLE"));
    }

    #[test]
    fn test_migration_runner_add() {
        let mut runner = MigrationRunner::new();

        runner.add(Migration::new(2, "second", "SQL2"));
        runner.add(Migration::new(1, "first", "SQL1"));

        assert_eq!(runner.count(), 2);
        assert_eq!(runner.migrations[0].version, 1);
        assert_eq!(runner.migrations[1].version, 2);
    }

    #[test]
    fn test_migration_runner_builtin() {
        let runner = MigrationRunner::builtin();

        assert!(runner.count() >= 1);
        assert_eq!(runner.migrations[0].version, 1);
        assert_eq!(runner.migrations[0].name, "initial");
    }

    #[tokio::test]
    async fn test_migration_run() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");

        let db = Database::open(&db_path).await.unwrap();

        let runner = MigrationRunner::builtin();
        runner.run(&db).await.unwrap();

        // Verify tables exist
        assert!(db.table_exists("session").await.unwrap());
        assert!(db.table_exists("message").await.unwrap());
        assert!(db.table_exists("project").await.unwrap());

        // Verify version
        let version = db.schema_version().await.unwrap();
        assert!(version >= 1);

        db.close().await;
    }

    #[tokio::test]
    async fn test_migration_pending() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.db");

        let db = Database::open(&db_path).await.unwrap();

        let runner = MigrationRunner::builtin();

        // Before running
        let pending = runner.pending(&db).await.unwrap();
        assert!(!pending.is_empty());

        // Run migrations
        runner.run(&db).await.unwrap();

        // After running
        let pending = runner.pending(&db).await.unwrap();
        assert!(pending.is_empty());

        db.close().await;
    }
}
