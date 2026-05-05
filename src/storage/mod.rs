//! Storage module for ROC (reopencode)
//!
//! Provides data persistence for sessions, messages, projects, and configuration.
//!
//! # Features
//!
//! - Dual backend support: SQLite (high performance) + JSON (compatibility)
//! - Session, message, part, and todo storage
//! - Project and workspace management
//! - In-memory caching with TTL
//! - Schema migrations
//!
//! # Example
//!
//! ```rust,no_run
//! use reopencode::storage::{Storage, BackendType};
//!
//! #[tokio::main]
//! async fn main() {
//!     let storage = Storage::new(BackendType::Json).await.unwrap();
//!     
//!     let sessions = storage.sessions();
//!     let session = sessions.create(Default::default()).await.unwrap();
//!     
//!     println!("Created session: {}", session.id);
//! }
//! ```

pub mod backend;
pub mod cache;
pub mod database;
pub mod error;
pub mod migration;
pub mod path;
pub mod project;
pub mod schema;
pub mod session;

// Re-export error types
pub use error::{DatabaseError, StorageError};

// Re-export path types
#[allow(unused_imports)]
pub use path::{CACHE_VERSION, GlobalPath};

// Re-export cache types
pub use cache::{CacheConfig, CacheStats, MemoryCache};

// Re-export schema types
pub use schema::{
    MessageInfo, MessagePart, MessageRecord, MessageRole, MessageTime, MessageWithParts, Page,
    ProjectRecord, SessionRecord, TodoItem, TodoPriority, TodoStatus, WorkspaceRecord,
};

// Re-export backend types
pub use backend::{BackendType, JsonBackend, SqliteBackend, StorageBackend};

// Re-export database types
pub use database::Database;

// Re-export session types
#[allow(unused_imports)]
pub use session::{MessageStore, SessionCreateInput, SessionStore, TodoStore};

// Re-export project types
#[allow(unused_imports)]
pub use project::{ProjectCreateInput, ProjectStore, WorkspaceCreateInput, WorkspaceStore};

// Re-export migration types
pub use migration::MigrationRunner;

use std::sync::Arc;

/// Main storage manager
///
/// Provides unified access to all storage operations.
pub struct Storage {
    backend: Arc<dyn StorageBackend>,
    cache: MemoryCache,
    path: &'static GlobalPath,
    backend_type: BackendType,
}

impl Storage {
    /// Create a new storage instance with the specified backend type
    pub async fn new(backend_type: BackendType) -> Result<Self, StorageError> {
        let path = GlobalPath::get();
        path.init().await?;

        let backend: Arc<dyn StorageBackend> = match backend_type {
            BackendType::Json => Arc::new(JsonBackend::new(&path.data)?),
            BackendType::Sqlite => {
                let db_path = path.database_path("kv");
                Arc::new(SqliteBackend::open(&db_path).await?)
            }
        };

        Ok(Self {
            backend,
            cache: MemoryCache::new(CacheConfig::default()),
            path,
            backend_type,
        })
    }

    /// Get the session store
    pub fn sessions(&self) -> SessionStore {
        SessionStore::new(self.backend.clone(), &self.cache)
    }

    /// Get the message store
    pub fn messages(&self) -> MessageStore {
        MessageStore::new(self.backend.clone(), &self.cache)
    }

    /// Get the todo store
    pub fn todos(&self) -> TodoStore {
        TodoStore::new(self.backend.clone())
    }

    /// Get the project store
    pub fn projects(&self) -> ProjectStore {
        ProjectStore::new(self.backend.clone())
    }

    /// Get the workspace store
    pub fn workspaces(&self) -> WorkspaceStore {
        WorkspaceStore::new(self.backend.clone())
    }

    /// Get the global path instance
    pub fn path(&self) -> &'static GlobalPath {
        self.path
    }

    /// Get the backend type
    pub fn backend_type(&self) -> BackendType {
        self.backend_type
    }

    /// Get the cache instance
    pub fn cache(&self) -> &MemoryCache {
        &self.cache
    }

    /// Clear all caches
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Run database migrations (SQLite only)
    pub async fn migrate(&self) -> Result<(), StorageError> {
        if self.backend_type == BackendType::Sqlite {
            let db_path = self.path.database_path("latest");
            let db = Database::open(&db_path).await?;
            let runner = MigrationRunner::builtin();
            runner.run(&db).await?;
            db.close().await;
        }
        Ok(())
    }
}

/// Create a new storage instance with default settings
pub async fn init() -> Result<Storage, StorageError> {
    Storage::new(BackendType::Json).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::storage::path::CACHE_VERSION;
    #[allow(unused_imports)]
    use crate::storage::project::ProjectCreateInput;
    #[allow(unused_imports)]
    use crate::storage::session::SessionCreateInput;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_new() {
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("OPENCODE_TEST_HOME", temp.path());
        }

        let storage = Storage::new(BackendType::Json).await.unwrap();

        assert_eq!(storage.backend_type(), BackendType::Json);

        unsafe {
            std::env::remove_var("OPENCODE_TEST_HOME");
        }
    }

    #[tokio::test]
    async fn test_storage_sessions() {
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("OPENCODE_TEST_HOME", temp.path());
        }

        let storage = Storage::new(BackendType::Json).await.unwrap();

        let session = storage
            .sessions()
            .create(SessionCreateInput {
                project_id: "test".to_string(),
                slug: "test".to_string(),
                directory: "/tmp".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(session.id.starts_with("ses_"));

        unsafe {
            std::env::remove_var("OPENCODE_TEST_HOME");
        }
    }

    #[tokio::test]
    async fn test_storage_projects() {
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("OPENCODE_TEST_HOME", temp.path());
        }

        let storage = Storage::new(BackendType::Json).await.unwrap();

        let project = storage
            .projects()
            .create(ProjectCreateInput {
                worktree: "/test/project".to_string(),
                vcs: Some("git".to_string()),
            })
            .await
            .unwrap();

        assert!(project.id.starts_with("proj_"));

        unsafe {
            std::env::remove_var("OPENCODE_TEST_HOME");
        }
    }

    #[test]
    fn test_backend_type_default() {
        assert_eq!(BackendType::default(), BackendType::Sqlite);
    }

    #[test]
    fn test_cache_version() {
        assert!(!CACHE_VERSION.is_empty());
    }
}
