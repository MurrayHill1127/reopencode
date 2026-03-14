//! SQLite storage layer for sessions

use chrono::DateTime;
use sqlx::Row;
use tracing::{debug, info};

use crate::session::{
    error::{Result, SessionError},
    types::{Session, SessionMessage, SessionStatus},
};

/// SQLite store for sessions
pub struct SessionStore {
    pool: sqlx::SqlitePool,
}

impl SessionStore {
    /// Create new store with existing pool
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize database tables
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing session database tables");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                message_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT NOT NULL DEFAULT '{}'
            ) STRICT
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY NOT NULL,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            ) STRICT
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        debug!("Session database tables initialized");
        Ok(())
    }

    // ========== Session CRUD ==========

    /// Create a new session
    pub async fn create_session(&self, session: &Session) -> Result<()> {
        debug!("Creating session: {}", session.id);

        sqlx::query(
            r#"
            INSERT INTO sessions (id, title, created_at, updated_at, status, message_count, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(Self::status_to_string(&session.status))
        .bind(session.message_count as i64)
        .bind(session.metadata.to_string())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get session by ID
    pub async fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, title, created_at, updated_at, status, message_count, metadata
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_session(row)?)),
            None => Ok(None),
        }
    }

    /// Update session
    pub async fn update_session(&self, session: &Session) -> Result<()> {
        debug!("Updating session: {}", session.id);

        sqlx::query(
            r#"
            UPDATE sessions
            SET title = ?, updated_at = ?, status = ?, message_count = ?, metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(&session.title)
        .bind(session.updated_at.to_rfc3339())
        .bind(Self::status_to_string(&session.status))
        .bind(session.message_count as i64)
        .bind(session.metadata.to_string())
        .bind(&session.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Delete session
    pub async fn delete_session(&self, id: &str) -> Result<bool> {
        debug!("Deleting session: {}", id);

        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, created_at, updated_at, status, message_count, metadata
            FROM sessions
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(self.row_to_session(row)?);
        }

        Ok(sessions)
    }

    // ========== Message CRUD ==========

    /// Create a new message
    pub async fn create_message(&self, message: &SessionMessage) -> Result<()> {
        debug!("Creating message: {} in session: {}", message.id, message.session_id);

        sqlx::query(
            r#"
            INSERT INTO messages (id, session_id, role, content, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&message.id)
        .bind(&message.session_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all messages for a session
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<SessionMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, role, content, created_at
            FROM messages
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }

    /// Get messages with limit (most recent first)
    pub async fn get_messages_with_limit(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionMessage>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, role, content, created_at
            FROM messages
            WHERE session_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }
        messages.reverse();

        Ok(messages)
    }

    /// Delete all messages for a session
    pub async fn delete_messages(&self, session_id: &str) -> Result<u64> {
        let result = sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    // ========== Helper Methods ==========

    fn status_to_string(status: &SessionStatus) -> &'static str {
        match status {
            SessionStatus::Active => "Active",
            SessionStatus::Paused => "Paused",
            SessionStatus::Completed => "Completed",
        }
    }

    fn string_to_status(s: &str) -> SessionStatus {
        match s {
            "Paused" => SessionStatus::Paused,
            "Completed" => SessionStatus::Completed,
            _ => SessionStatus::Active,
        }
    }

    fn row_to_session(&self, row: sqlx::sqlite::SqliteRow) -> Result<Session> {
        let id: String = row.get("id");
        let title: String = row.get("title");
        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");
        let status: String = row.get("status");
        let message_count: i64 = row.get("message_count");
        let metadata: String = row.get("metadata");

        Ok(Session {
            id,
            title,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| SessionError::DateTimeParse(e.to_string()))?
                .into(),
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map_err(|e| SessionError::DateTimeParse(e.to_string()))?
                .into(),
            status: Self::string_to_status(&status),
            message_count: message_count as u32,
            metadata: serde_json::from_str(&metadata)?,
        })
    }

    fn row_to_message(&self, row: sqlx::sqlite::SqliteRow) -> Result<SessionMessage> {
        let id: String = row.get("id");
        let session_id: String = row.get("session_id");
        let role: String = row.get("role");
        let content: String = row.get("content");
        let created_at: String = row.get("created_at");

        Ok(SessionMessage {
            id,
            session_id,
            role,
            content,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| SessionError::DateTimeParse(e.to_string()))?
                .into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::{Session, SessionMessage};

    async fn setup_test_store() -> SessionStore {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        let store = SessionStore::new(pool);
        store.initialize().await.expect("Failed to initialize");

        store
    }

    #[tokio::test]
    async fn test_create_and_get_session() {
        let store = setup_test_store().await;
        let session = Session::new(Some("Test".to_string()));
        let session_id = session.id.clone();

        store.create_session(&session).await.unwrap();
        let retrieved = store.get_session(&session_id).await.unwrap().unwrap();

        assert_eq!(retrieved.id, session_id);
        assert_eq!(retrieved.title, "Test");
    }

    #[tokio::test]
    async fn test_get_nonexistent_session() {
        let store = setup_test_store().await;
        let result = store.get_session("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_session() {
        let store = setup_test_store().await;
        let mut session = Session::new(Some("Original".to_string()));
        store.create_session(&session).await.unwrap();

        session.title = "Updated".to_string();
        session.add_message();
        store.update_session(&session).await.unwrap();

        let retrieved = store.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated");
        assert_eq!(retrieved.message_count, 1);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let store = setup_test_store().await;
        let session = Session::new(None);
        let session_id = session.id.clone();

        store.create_session(&session).await.unwrap();
        let deleted = store.delete_session(&session_id).await.unwrap();
        assert!(deleted);

        let retrieved = store.get_session(&session_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let store = setup_test_store().await;

        for i in 0..3 {
            let session = Session::new(Some(format!("Session {}", i)));
            store.create_session(&session).await.unwrap();
        }

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_create_and_get_messages() {
        let store = setup_test_store().await;
        let session = Session::new(None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        let msg1 = SessionMessage::new(session_id.clone(), "user".to_string(), "Hello".to_string());
        let msg2 = SessionMessage::new(session_id.clone(), "assistant".to_string(), "Hi".to_string());

        store.create_message(&msg1).await.unwrap();
        store.create_message(&msg2).await.unwrap();

        let messages = store.get_messages(&session_id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi");
    }

    #[tokio::test]
    async fn test_get_messages_with_limit() {
        let store = setup_test_store().await;
        let session = Session::new(None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        for i in 0..5 {
            let msg = SessionMessage::new(
                session_id.clone(),
                "user".to_string(),
                format!("Message {}", i),
            );
            store.create_message(&msg).await.unwrap();
        }

        let messages = store.get_messages_with_limit(&session_id, 2).await.unwrap();
        assert_eq!(messages.len(), 2);
    }
}