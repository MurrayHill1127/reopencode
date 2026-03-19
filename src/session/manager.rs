//! Session manager - high-level service layer

use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tracing::{debug, info};

use crate::session::{
    error::{Result, SessionError},
    query::SessionQuery,
    store::SessionStore,
    types::{MessageId, Session, SessionFilter, SessionId, SessionMessage, SessionStatus},
};

/// Session manager with SQLite persistence
#[derive(Clone)]
pub struct SessionManager {
    store: Arc<SessionStore>,
    query: Arc<SessionQuery>,
    pool: sqlx::SqlitePool,
}

impl SessionManager {
    /// Create new session manager with database connection
    pub async fn new(database_url: &str) -> Result<Self> {
        info!("Creating session manager: {}", database_url);

        let pool = Self::create_pool(database_url).await?;
        let store = Arc::new(SessionStore::new(pool.clone()));
        let query = Arc::new(SessionQuery::new(pool.clone()));

        store.initialize().await?;

        Ok(Self { store, query, pool })
    }

    /// Create in-memory session manager for testing
    pub async fn in_memory() -> Result<Self> {
        Self::new("sqlite::memory:").await
    }

    /// Create connection pool with optimized SQLite settings
    async fn create_pool(database_url: &str) -> Result<sqlx::SqlitePool> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .min_connections(1)
            .connect_with(options)
            .await?;

        Ok(pool)
    }

    // ========== Session Management ==========

    /// Create a new session
    pub async fn create_session(&self, title: Option<String>) -> Result<SessionId> {
        debug!("Creating new session");

        let session = Session::new(title);
        self.store.create_session(&session).await?;

        info!("Created session: {}", session.id);
        Ok(session.id)
    }

    /// Get session by ID
    pub async fn get_session(&self, id: &str) -> Result<Session> {
        self.store
            .get_session(id)
            .await?
            .ok_or_else(|| SessionError::NotFound {
                session_id: id.to_string(),
            })
    }

    /// Update session
    pub async fn update_session(&self, session: &Session) -> Result<()> {
        debug!("Updating session: {}", session.id);
        self.store.update_session(session).await
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<bool> {
        info!("Deleting session: {}", session_id);

        let messages = self.store.delete_messages(session_id).await?;
        debug!("Deleted {} messages", messages);

        self.store.delete_session(session_id).await
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        self.store.list_sessions().await
    }

    /// List sessions with filter
    pub async fn list_sessions_filtered(&self, filter: SessionFilter) -> Result<Vec<Session>> {
        self.query.list_sessions(filter).await
    }

    /// Search sessions by title
    pub async fn search_sessions(&self, query: &str) -> Result<Vec<Session>> {
        self.query.search_sessions(query).await
    }

    /// Get session history
    pub async fn get_session_history(&self, session_id: &str) -> Result<Vec<Session>> {
        self.query.get_session_history(session_id).await
    }

    /// Set session title
    pub async fn set_title(&self, session_id: &str, title: String) -> Result<Session> {
        let mut session = self.get_session(session_id).await?;
        session.title = title;
        session.touch();
        self.store.update_session(&session).await?;
        Ok(session)
    }

    /// Set session status
    pub async fn set_status(&self, session_id: &str, status: SessionStatus) -> Result<Session> {
        let mut session = self.get_session(session_id).await?;
        session.set_status(status);
        self.store.update_session(&session).await?;
        Ok(session)
    }

    // ========== Message Management ==========

    /// Add a message to a session
    pub async fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
    ) -> Result<MessageId> {
        debug!("Adding message to session: {}", session_id);

        let mut session = self.get_session(session_id).await?;
        session.add_message();
        self.store.update_session(&session).await?;

        let message = SessionMessage::new(
            session_id.to_string(),
            role.to_string(),
            content.to_string(),
        );
        self.store.create_message(&message).await?;

        Ok(message.id)
    }

    /// Get messages from a session
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>> {
        self.query.get_messages(session_id, None).await
    }

    /// Get messages with limit
    pub async fn get_messages_with_limit(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionMessage>> {
        self.query.get_messages(session_id, Some(limit)).await
    }

    /// Get messages since a timestamp
    pub async fn get_messages_since(
        &self,
        session_id: &str,
        since: i64,
    ) -> Result<Vec<SessionMessage>> {
        let all_messages = self.get_messages(session_id).await?;
        Ok(all_messages
            .into_iter()
            .filter(|m| m.created_at.timestamp() >= since)
            .collect())
    }

    /// Count messages in a session
    pub async fn count_messages(&self, session_id: &str) -> Result<i64> {
        self.query.count_messages(session_id).await
    }

    /// Count total sessions
    pub async fn count_sessions(&self) -> Result<i64> {
        self.query.count_sessions().await
    }

    // ========== Utility ==========

    /// Get the underlying pool for advanced operations
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_manager() -> (SessionManager, TempDir) {
        let tmp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = tmp_dir.path().join("test.db");
        let manager = SessionManager::new(&format!("sqlite:{}", db_path.display()))
            .await
            .expect("Failed to create manager");

        (manager, tmp_dir)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager
            .create_session(Some("Test".to_string()))
            .await
            .unwrap();

        let session = manager.get_session(&id).await.unwrap();
        assert_eq!(session.title, "Test");
        assert_eq!(session.status, SessionStatus::Active);
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() {
        let (manager, _tmp) = setup_test_manager().await;
        let result = manager.get_session("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().is_not_found());
    }

    #[tokio::test]
    async fn test_add_message() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None).await.unwrap();

        manager.add_message(&id, "user", "Hello").await.unwrap();
        manager
            .add_message(&id, "assistant", "Hi there!")
            .await
            .unwrap();

        let session = manager.get_session(&id).await.unwrap();
        assert_eq!(session.message_count, 2);

        let messages = manager.get_messages(&id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi there!");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let (manager, _tmp) = setup_test_manager().await;

        for i in 0..3 {
            manager
                .create_session(Some(format!("Session {}", i)))
                .await
                .unwrap();
        }

        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None).await.unwrap();

        let deleted = manager.delete_session(&id).await.unwrap();
        assert!(deleted);

        let result = manager.get_session(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_sessions() {
        let (manager, _tmp) = setup_test_manager().await;

        manager
            .create_session(Some("Project Alpha".to_string()))
            .await
            .unwrap();
        manager
            .create_session(Some("Project Beta".to_string()))
            .await
            .unwrap();
        manager
            .create_session(Some("Other".to_string()))
            .await
            .unwrap();

        let results = manager.search_sessions("Project").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_set_title() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager
            .create_session(Some("Original".to_string()))
            .await
            .unwrap();

        let updated = manager
            .set_title(&id, "New Title".to_string())
            .await
            .unwrap();
        assert_eq!(updated.title, "New Title");

        let session = manager.get_session(&id).await.unwrap();
        assert_eq!(session.title, "New Title");
    }

    #[tokio::test]
    async fn test_set_status() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None).await.unwrap();

        let updated = manager
            .set_status(&id, SessionStatus::Paused)
            .await
            .unwrap();
        assert_eq!(updated.status, SessionStatus::Paused);

        let session = manager.get_session(&id).await.unwrap();
        assert_eq!(session.status, SessionStatus::Paused);
    }

    #[tokio::test]
    async fn test_count_sessions() {
        let (manager, _tmp) = setup_test_manager().await;

        assert_eq!(manager.count_sessions().await.unwrap(), 0);

        for _ in 0..5 {
            manager.create_session(None).await.unwrap();
        }

        assert_eq!(manager.count_sessions().await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_count_messages() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None).await.unwrap();

        assert_eq!(manager.count_messages(&id).await.unwrap(), 0);

        for i in 0..3 {
            manager
                .add_message(&id, "user", &format!("Msg {}", i))
                .await
                .unwrap();
        }

        assert_eq!(manager.count_messages(&id).await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_in_memory() {
        let manager = SessionManager::in_memory().await.unwrap();

        let id = manager
            .create_session(Some("In-Memory Test".to_string()))
            .await
            .unwrap();
        let session = manager.get_session(&id).await.unwrap();

        assert_eq!(session.title, "In-Memory Test");
    }
}
