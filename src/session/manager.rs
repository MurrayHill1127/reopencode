//! Session manager - high-level service layer

use std::str::FromStr;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tracing::{debug, info};

use crate::session::{
    error::{Result, SessionError},
    query::SessionQuery,
    status::SessionStatusInfo,
    store::SessionStore,
    todo::TodoInfo,
    types::{FileDiff, MessageId, Session, SessionFilter, SessionId, SessionMessage, SessionStatus, SessionRevert, SessionSummary},
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
    pub async fn create_session(&self, title: Option<String>, parent_id: Option<String>) -> Result<SessionId> {
        debug!("Creating new session");

        let session = Session::new(title, parent_id);
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

    /// Get child sessions of a parent session
    pub async fn get_children(&self, parent_id: &str) -> Result<Vec<Session>> {
        self.store.get_children(parent_id).await
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

    /// Set session archived timestamp
    pub async fn set_archived(&self, session_id: &str, archived: Option<i64>) -> Result<Session> {
        let mut session = self.get_session(session_id).await?;
        session.archived_at = archived;
        session.touch();
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

    /// Add a message with rich parts (tool call history, etc.)
    pub async fn add_message_with_parts(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        parts: Vec<crate::session::types::MessagePart>,
    ) -> Result<MessageId> {
        debug!("Adding message with parts to session: {}", session_id);

        let mut session = self.get_session(session_id).await?;
        session.add_message();
        self.store.update_session(&session).await?;

        let mut message = SessionMessage::new(
            session_id.to_string(),
            role.to_string(),
            content.to_string(),
        );
        for part in parts {
            message.add_part(part);
        }
        self.store.create_message(&message).await?;

        Ok(message.id)
    }

    /// Delete the last N messages from a session (for undo).
    /// Returns the number of messages deleted.
    pub async fn delete_last_messages(&self, session_id: &str, count: usize) -> Result<usize> {
        let messages = self.get_messages(session_id).await?;
        let to_delete: Vec<_> = messages.iter().rev().take(count).collect();
        let deleted = to_delete.len();
        for msg in &to_delete {
            let _ = sqlx::query("DELETE FROM messages WHERE id = ?")
                .bind(&msg.id)
                .execute(&self.pool)
                .await;
        }
        Ok(deleted)
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

    // ========== Todo Management ==========

    pub async fn get_todos(&self, session_id: &str) -> Result<Vec<TodoInfo>> {
        self.store.get_todos(session_id).await
    }

    pub async fn update_todos(&self, session_id: &str, todos: Vec<TodoInfo>) -> Result<()> {
        self.store.update_todos(session_id, &todos).await
    }

    // ========== Utility ==========

    /// Get the underlying pool for advanced operations
    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    // ========== Status Tracking ==========

    /// Get status for a session (returns idle if not set)
    pub fn get_session_status(&self, session_id: &str) -> SessionStatusInfo {
        crate::session::status::get(session_id)
    }

    /// List all session statuses
    pub fn list_session_statuses(&self) -> std::collections::HashMap<String, SessionStatusInfo> {
        crate::session::status::list()
    }

    /// Set status for a session
    pub fn set_session_status(&self, session_id: &str, status: SessionStatusInfo) {
        crate::session::status::set(session_id, status);
    }

    // ========== Abort Operations ==========

    /// Abort a session's active operation
    ///
    /// Returns Ok(true) if session was aborted (or was already idle).
    /// Returns Err if session does not exist.
    pub async fn abort_session(&self, session_id: &str) -> Result<bool> {
        // Check session exists first
        self.get_session(session_id).await?;

        // Cancel any active prompt
        crate::session::prompt::cancel(session_id).await;

        Ok(true)
    }

    pub async fn fork_session(
        &self,
        session_id: &str,
        message_id: Option<&str>,
    ) -> Result<Session> {
        info!("Forking session: {} (up to message: {:?})", session_id, message_id);

        let original = self.get_session(session_id).await?;
        
        let forked_title = format!("Forked: {}", original.title);
        let new_session_id = self.create_session(Some(forked_title), Some(session_id.to_string())).await?;
        
        let messages = match message_id {
            Some(mid) => self.store.get_messages_until(session_id, mid).await?,
            None => self.store.get_messages(session_id).await?,
        };

        let count = self.copy_messages(session_id, &new_session_id, &messages).await?;
        info!("Copied {} messages to forked session {}", count, new_session_id);

        self.get_session(&new_session_id).await
    }

    pub async fn set_revert(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: Option<&str>,
        summary: Option<SessionSummary>,
    ) -> Result<Session> {
        info!("Setting revert on session {} for message {}", session_id, message_id);

        let mut session = self.get_session(session_id).await?;
        
        session.revert = Some(SessionRevert {
            message_id: message_id.to_string(),
            part_id: part_id.map(String::from),
            snapshot: None,
            diff: None,
            summary,
        });
        session.touch();
        
        self.store.update_session(&session).await?;
        info!("Revert set on session {}", session_id);
        
        Ok(session)
    }

    pub async fn clear_revert(&self, session_id: &str) -> Result<Session> {
        info!("Clearing revert from session: {}", session_id);

        let mut session = self.get_session(session_id).await?;
        session.revert = None;
        session.touch();
        
        self.store.update_session(&session).await?;
        info!("Revert cleared from session {}", session_id);
        
        Ok(session)
    }

    pub async fn get_messages_until(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<Vec<SessionMessage>> {
        debug!("Getting messages until {} in session {}", message_id, session_id);
        self.store.get_messages_until(session_id, message_id).await
    }

    pub async fn copy_messages(
        &self,
        from_session_id: &str,
        to_session_id: &str,
        messages: &[SessionMessage],
    ) -> Result<usize> {
        debug!(
            "Copying {} messages from {} to {}",
            messages.len(),
            from_session_id,
            to_session_id
        );

        let mut count = 0;
        for message in messages {
            self.store.copy_message_to_session(to_session_id, message).await?;
            count += 1;
        }

        Ok(count)
    }

    // ========== Share Operations ==========

    pub async fn share_session(&self, session_id: &str) -> Result<Session> {
        info!("Sharing session: {}", session_id);

        let mut session = self.get_session(session_id).await?;
        let share_url = format!("https://share.opencode.ai/session/{}", session_id);
        session.share_url = Some(share_url);
        session.touch();
        
        self.store.update_session(&session).await?;
        info!("Session {} shared", session_id);
        
        Ok(session)
    }

    pub async fn unshare_session(&self, session_id: &str) -> Result<Session> {
        info!("Unsharing session: {}", session_id);

        let mut session = self.get_session(session_id).await?;
        session.share_url = None;
        session.touch();
        
        self.store.update_session(&session).await?;
        info!("Session {} unshared", session_id);
        
        Ok(session)
    }

    // ========== Diff Operations ==========

    pub async fn get_session_diff(&self, session_id: &str, _message_id: Option<&str>) -> Result<Vec<FileDiff>> {
        debug!("Getting diff for session: {} (message: {:?})", session_id, _message_id);
        
        self.get_session(session_id).await?;
        
        // TODO: Compute actual diffs from message snapshots
        Ok(Vec::new())
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
            .create_session(Some("Test".to_string()), None)
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
        let id = manager.create_session(None, None).await.unwrap();

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
                .create_session(Some(format!("Session {}", i)), None)
                .await
                .unwrap();
        }

        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None, None).await.unwrap();

        let deleted = manager.delete_session(&id).await.unwrap();
        assert!(deleted);

        let result = manager.get_session(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_sessions() {
        let (manager, _tmp) = setup_test_manager().await;

        manager
            .create_session(Some("Project Alpha".to_string()), None)
            .await
            .unwrap();
        manager
            .create_session(Some("Project Beta".to_string()), None)
            .await
            .unwrap();
        manager
            .create_session(Some("Other".to_string()), None)
            .await
            .unwrap();

        let results = manager.search_sessions("Project").await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_set_title() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager
            .create_session(Some("Original".to_string()), None)
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
        let id = manager.create_session(None, None).await.unwrap();

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
            manager.create_session(None, None).await.unwrap();
        }

        assert_eq!(manager.count_sessions().await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_count_messages() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None, None).await.unwrap();

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
            .create_session(Some("In-Memory Test".to_string()), None)
            .await
            .unwrap();
        let session = manager.get_session(&id).await.unwrap();

        assert_eq!(session.title, "In-Memory Test");
    }

    #[tokio::test]
    async fn test_fork_session_all_messages() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager
            .create_session(Some("Original".to_string()), None)
            .await
            .unwrap();

        manager.add_message(&id, "user", "Hello").await.unwrap();
        manager
            .add_message(&id, "assistant", "Hi there!")
            .await
            .unwrap();

        let forked = manager.fork_session(&id, None).await.unwrap();

        assert_eq!(forked.title, "Forked: Original");
        assert_eq!(forked.parent_id, Some(id.clone()));

        let forked_messages = manager.get_messages(&forked.id).await.unwrap();
        assert_eq!(forked_messages.len(), 2);
    }

    #[tokio::test]
    async fn test_fork_session_partial() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(Some("Original".to_string()), None).await.unwrap();

        let msg1_id = manager.add_message(&id, "user", "First").await.unwrap();
        manager.add_message(&id, "assistant", "Second").await.unwrap();
        manager.add_message(&id, "user", "Third").await.unwrap();

        let forked = manager.fork_session(&id, Some(&msg1_id)).await.unwrap();

        assert_eq!(forked.title, "Forked: Original");
        
        let forked_messages = manager.get_messages(&forked.id).await.unwrap();
        assert_eq!(forked_messages.len(), 1);
        assert_eq!(forked_messages[0].content, "First");
    }

    #[tokio::test]
    async fn test_set_revert() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None, None).await.unwrap();

        let msg_id = manager.add_message(&id, "user", "Test").await.unwrap();

        let updated = manager
            .set_revert(&id, &msg_id, None, None)
            .await
            .unwrap();

        assert!(updated.revert.is_some());
        let revert = updated.revert.unwrap();
        assert_eq!(revert.message_id, msg_id);
    }

    #[tokio::test]
    async fn test_set_revert_with_part_and_summary() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None, None).await.unwrap();

        let msg_id = manager.add_message(&id, "user", "Test").await.unwrap();

        let summary = crate::session::types::SessionSummary {
            additions: 10,
            deletions: 5,
            files: 2,
        };

        let updated = manager
            .set_revert(&id, &msg_id, Some("part-1"), Some(summary))
            .await
            .unwrap();

        assert!(updated.revert.is_some());
        let revert = updated.revert.unwrap();
        assert_eq!(revert.message_id, msg_id);
        assert_eq!(revert.part_id, Some("part-1".to_string()));
        assert!(revert.summary.is_some());
        assert_eq!(revert.summary.unwrap().additions, 10);
    }

    #[tokio::test]
    async fn test_clear_revert() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None, None).await.unwrap();

        let msg_id = manager.add_message(&id, "user", "Test").await.unwrap();
        manager.set_revert(&id, &msg_id, None, None).await.unwrap();

        let cleared = manager.clear_revert(&id).await.unwrap();

        assert!(cleared.revert.is_none());
    }

    #[tokio::test]
    async fn test_get_messages_until() {
        let (manager, _tmp) = setup_test_manager().await;
        let id = manager.create_session(None, None).await.unwrap();

        let msg1_id = manager.add_message(&id, "user", "First").await.unwrap();
        manager.add_message(&id, "assistant", "Second").await.unwrap();
        manager.add_message(&id, "user", "Third").await.unwrap();

        let messages = manager.get_messages_until(&id, &msg1_id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "First");
    }

    #[tokio::test]
    async fn test_copy_messages() {
        let (manager, _tmp) = setup_test_manager().await;
        
        let session1 = manager.create_session(None, None).await.unwrap();
        let session2 = manager.create_session(None, None).await.unwrap();

        manager.add_message(&session1, "user", "Msg 1").await.unwrap();
        manager.add_message(&session1, "assistant", "Msg 2").await.unwrap();

        let messages = manager.get_messages(&session1).await.unwrap();
        let count = manager
            .copy_messages(&session1, &session2, &messages)
            .await
            .unwrap();

        assert_eq!(count, 2);

        let session2_messages = manager.get_messages(&session2).await.unwrap();
        assert_eq!(session2_messages.len(), 2);
    }
}
