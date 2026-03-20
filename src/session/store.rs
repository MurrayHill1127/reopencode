//! SQLite storage layer for sessions

use chrono::{DateTime, Utc};
use sqlx::Row;
use tracing::{debug, info};

use crate::session::{
    error::{Result, SessionError},
    parts::Part,
    todo::TodoInfo,
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
                metadata TEXT NOT NULL DEFAULT '{}',
                parent_id TEXT,
                archived_at INTEGER,
                revert TEXT,
                summary TEXT,
                share_url TEXT
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
            CREATE TABLE IF NOT EXISTS parts (
                id TEXT PRIMARY KEY NOT NULL,
                session_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                part_type TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
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
            CREATE INDEX IF NOT EXISTS idx_parts_message_id ON parts(message_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_parts_session_id ON parts(session_id)
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

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_parent_id ON sessions(parent_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS todos (
                session_id TEXT NOT NULL,
                content TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                priority TEXT NOT NULL DEFAULT 'medium',
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (session_id, position),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            ) STRICT
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_todos_session_id ON todos(session_id)
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
            INSERT INTO sessions (id, title, created_at, updated_at, status, message_count, metadata, parent_id, archived_at, revert, summary, share_url)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(session.created_at.to_rfc3339())
        .bind(session.updated_at.to_rfc3339())
        .bind(Self::status_to_string(&session.status))
        .bind(session.message_count as i64)
        .bind(session.metadata.to_string())
        .bind(&session.parent_id)
        .bind(session.archived_at)
        .bind(session.revert.as_ref().map(|r| serde_json::to_string(r).unwrap()))
        .bind(session.summary.as_ref().map(|s| serde_json::to_string(s).unwrap()))
        .bind(&session.share_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get session by ID
    pub async fn get_session(&self, id: &str) -> Result<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, title, created_at, updated_at, status, message_count, metadata, parent_id, archived_at, revert, summary, share_url
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
            SET title = ?, updated_at = ?, status = ?, message_count = ?, metadata = ?, archived_at = ?, revert = ?, summary = ?, share_url = ?
            WHERE id = ?
            "#,
        )
        .bind(&session.title)
        .bind(session.updated_at.to_rfc3339())
        .bind(Self::status_to_string(&session.status))
        .bind(session.message_count as i64)
        .bind(session.metadata.to_string())
        .bind(session.archived_at)
        .bind(session.revert.as_ref().map(|r| serde_json::to_string(r).unwrap()))
        .bind(session.summary.as_ref().map(|s| serde_json::to_string(s).unwrap()))
        .bind(&session.share_url)
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
            SELECT id, title, created_at, updated_at, status, message_count, metadata, parent_id, archived_at, revert, summary, share_url
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

    /// Get child sessions of a parent session
    pub async fn get_children(&self, parent_id: &str) -> Result<Vec<Session>> {
        debug!("Getting children for session: {}", parent_id);

        let rows = sqlx::query(
            r#"
            SELECT id, title, created_at, updated_at, status, message_count, metadata, parent_id, archived_at, revert, summary, share_url
            FROM sessions
            WHERE parent_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(parent_id)
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
        debug!(
            "Creating message: {} in session: {}",
            message.id, message.session_id
        );

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

    // ========== Parts CRUD ==========

    /// Create a new message part
    pub async fn create_part(&self, part: &Part) -> Result<()> {
        debug!("Creating part: {} for message: {}", part.id(), part.message_id());

        let part_type = match part {
            Part::Text(_) => "text",
            Part::Reasoning(_) => "reasoning",
            Part::Tool(_) => "tool",
            Part::File(_) => "file",
            Part::Snapshot(_) => "snapshot",
            Part::Patch(_) => "patch",
            Part::Compaction(_) => "compaction",
            Part::Subtask(_) => "subtask",
            Part::StepStart(_) => "step-start",
            Part::StepFinish(_) => "step-finish",
            Part::Agent(_) => "agent",
            Part::Retry(_) => "retry",
        };

        let data = serde_json::to_string(part)?;

        sqlx::query(
            r#"
            INSERT INTO parts (id, session_id, message_id, part_type, data)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(part.id())
        .bind(part.session_id())
        .bind(part.message_id())
        .bind(part_type)
        .bind(&data)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all parts for a message
    pub async fn get_parts(&self, message_id: &str) -> Result<Vec<Part>> {
        debug!("Getting parts for message: {}", message_id);

        let rows = sqlx::query(
            r#"
            SELECT id, session_id, message_id, part_type, data
            FROM parts
            WHERE message_id = ?
            ORDER BY id ASC
            "#,
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await?;

        let mut parts = Vec::new();
        for row in rows {
            let data: String = row.get("data");
            match serde_json::from_str(&data) {
                Ok(part) => parts.push(part),
                Err(e) => {
                    tracing::warn!("Failed to deserialize part: {}", e);
                    continue;
                }
            }
        }

        Ok(parts)
    }

    /// Get parts for multiple messages (batch operation)
    pub async fn get_parts_for_messages(&self, message_ids: &[String]) -> Result<std::collections::HashMap<String, Vec<Part>>> {
        if message_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let placeholders = message_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT id, session_id, message_id, part_type, data FROM parts WHERE message_id IN ({}) ORDER BY message_id, id",
            placeholders
        );

        let mut query_builder = sqlx::query(&query);
        for id in message_ids {
            query_builder = query_builder.bind(id);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut parts_by_message = std::collections::HashMap::new();
        for row in rows {
            let message_id: String = row.get("message_id");
            let data: String = row.get("data");
            
            match serde_json::from_str::<Part>(&data) {
                Ok(part) => {
                    parts_by_message
                        .entry(message_id)
                        .or_insert_with(Vec::new)
                        .push(part);
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize part: {}", e);
                }
            }
        }

        Ok(parts_by_message)
    }

    /// Update a part
    pub async fn update_part(&self, part: &Part) -> Result<()> {
        debug!("Updating part: {}", part.id());

        let data = serde_json::to_string(part)?;

        let result = sqlx::query(
            r#"
            UPDATE parts SET data = ? WHERE id = ?
            "#,
        )
        .bind(&data)
        .bind(part.id())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(SessionError::NotFound {
                session_id: format!("part {}", part.id()),
            });
        }

        Ok(())
    }

    /// Delete a part
    pub async fn delete_part(&self, part_id: &str) -> Result<bool> {
        debug!("Deleting part: {}", part_id);

        let result = sqlx::query("DELETE FROM parts WHERE id = ?")
            .bind(part_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete all parts for a message
    pub async fn delete_parts_for_message(&self, message_id: &str) -> Result<u64> {
        debug!("Deleting parts for message: {}", message_id);

        let result = sqlx::query("DELETE FROM parts WHERE message_id = ?")
            .bind(message_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Create multiple parts in a transaction
    pub async fn create_parts_batch(&self, parts: &[Part]) -> Result<()> {
        if parts.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for part in parts {
            let part_type = match part {
                Part::Text(_) => "text",
                Part::Reasoning(_) => "reasoning",
                Part::Tool(_) => "tool",
                Part::File(_) => "file",
                Part::Snapshot(_) => "snapshot",
                Part::Patch(_) => "patch",
                Part::Compaction(_) => "compaction",
                Part::Subtask(_) => "subtask",
                Part::StepStart(_) => "step-start",
                Part::StepFinish(_) => "step-finish",
                Part::Agent(_) => "agent",
                Part::Retry(_) => "retry",
            };

            let data = serde_json::to_string(part)?;

            sqlx::query(
                r#"
                INSERT INTO parts (id, session_id, message_id, part_type, data)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(part.id())
            .bind(part.session_id())
            .bind(part.message_id())
            .bind(part_type)
            .bind(&data)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // ========== Todo CRUD ==========

    pub async fn get_todos(&self, session_id: &str) -> Result<Vec<TodoInfo>> {
        debug!("Getting todos for session: {}", session_id);

        let rows = sqlx::query(
            r#"
            SELECT content, status, priority
            FROM todos
            WHERE session_id = ?
            ORDER BY position ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let todos = rows
            .iter()
            .map(|row| TodoInfo {
                content: row.get("content"),
                status: row.get("status"),
                priority: row.get("priority"),
            })
            .collect();

        Ok(todos)
    }

    pub async fn update_todos(&self, session_id: &str, todos: &[TodoInfo]) -> Result<()> {
        debug!("Updating todos for session: {}", session_id);

        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM todos WHERE session_id = ?")
            .bind(session_id)
            .execute(&mut *tx)
            .await?;

        for (position, todo) in todos.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO todos (session_id, content, status, priority, position)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(session_id)
            .bind(&todo.content)
            .bind(&todo.status)
            .bind(&todo.priority)
            .bind(position as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // ========== Session Revert Operations ==========

    /// Update session with revert info
    pub async fn update_session_revert(
        &self,
        session_id: &str,
        revert: Option<crate::session::types::SessionRevert>,
    ) -> Result<()> {
        debug!("Updating session revert: {}", session_id);

        sqlx::query(
            r#"
            UPDATE sessions
            SET revert = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(revert.as_ref().map(|r| serde_json::to_string(r).unwrap()))
        .bind(Utc::now().to_rfc3339())
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get messages up to a specific message ID (inclusive)
    pub async fn get_messages_until(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<Vec<SessionMessage>> {
        debug!("Getting messages until {} in session {}", message_id, session_id);

        // First, get the created_at timestamp of the target message
        let target_row = sqlx::query(
            r#"
            SELECT created_at FROM messages
            WHERE id = ? AND session_id = ?
            "#,
        )
        .bind(message_id)
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        match target_row {
            Some(row) => {
                let target_created_at: String = row.get("created_at");
                
                // Get all messages up to and including this timestamp
                let rows = sqlx::query(
                    r#"
                    SELECT id, session_id, role, content, created_at
                    FROM messages
                    WHERE session_id = ? AND created_at <= ?
                    ORDER BY created_at ASC
                    "#,
                )
                .bind(session_id)
                .bind(&target_created_at)
                .fetch_all(&self.pool)
                .await?;

                let mut messages = Vec::new();
                for row in rows {
                    messages.push(self.row_to_message(row)?);
                }

                Ok(messages)
            }
            None => Err(SessionError::NotFound {
                session_id: format!("message {}", message_id),
            }),
        }
    }

    /// Copy message to another session
    pub async fn copy_message_to_session(
        &self,
        to_session_id: &str,
        message: &SessionMessage,
    ) -> Result<String> {
        debug!(
            "Copying message {} to session {}",
            message.id, to_session_id
        );

        // Create a new message with a new ID but same content
        let new_id = uuid::Uuid::new_v4().to_string();
        
        sqlx::query(
            r#"
            INSERT INTO messages (id, session_id, role, content, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&new_id)
        .bind(to_session_id)
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(new_id)
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
        let parent_id: Option<String> = row.get("parent_id");
        let archived_at: Option<i64> = row.get("archived_at");
        let revert: Option<String> = row.get("revert");
        let summary: Option<String> = row.get("summary");
        let share_url: Option<String> = row.get("share_url");

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
            parent_id,
            archived_at,
            revert: revert.and_then(|r| serde_json::from_str(&r).ok()),
            summary: summary.and_then(|s| serde_json::from_str(&s).ok()),
            share_url,
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
            parts: Vec::new(),
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
        let session = Session::new(Some("Test".to_string()), None);
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
        let mut session = Session::new(Some("Original".to_string()), None);
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
        let session = Session::new(None, None);
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
            let session = Session::new(Some(format!("Session {}", i)), None);
            store.create_session(&session).await.unwrap();
        }

        let sessions = store.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_create_and_get_messages() {
        let store = setup_test_store().await;
        let session = Session::new(None, None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        let msg1 = SessionMessage::new(session_id.clone(), "user".to_string(), "Hello".to_string());
        let msg2 = SessionMessage::new(
            session_id.clone(),
            "assistant".to_string(),
            "Hi".to_string(),
        );

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
        let session = Session::new(None, None);
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

    #[tokio::test]
    async fn test_parent_child_sessions() {
        let store = setup_test_store().await;
        
        let parent = Session::new(Some("Parent".to_string()), None);
        let parent_id = parent.id.clone();
        store.create_session(&parent).await.unwrap();
        
        let child1 = Session::new(Some("Child 1".to_string()), Some(parent_id.clone()));
        let child2 = Session::new(Some("Child 2".to_string()), Some(parent_id.clone()));
        store.create_session(&child1).await.unwrap();
        store.create_session(&child2).await.unwrap();
        
        let children = store.get_children(&parent_id).await.unwrap();
        assert_eq!(children.len(), 2);
        
        let orphan = Session::new(Some("Orphan".to_string()), None);
        store.create_session(&orphan).await.unwrap();
        let orphan_children = store.get_children(&orphan.id).await.unwrap();
        assert_eq!(orphan_children.len(), 0);
    }

    #[tokio::test]
    async fn test_update_session_revert() {
        let store = setup_test_store().await;
        let session = Session::new(Some("Test".to_string()), None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        let revert = crate::session::types::SessionRevert {
            message_id: "msg-123".to_string(),
            part_id: Some("part-456".to_string()),
            snapshot: None,
            diff: None,
            summary: None,
        };

        store.update_session_revert(&session_id, Some(revert)).await.unwrap();

        let retrieved = store.get_session(&session_id).await.unwrap().unwrap();
        assert!(retrieved.revert.is_some());
        let revert_data = retrieved.revert.unwrap();
        assert_eq!(revert_data.message_id, "msg-123");
        assert_eq!(revert_data.part_id, Some("part-456".to_string()));
    }

    #[tokio::test]
    async fn test_clear_session_revert() {
        let store = setup_test_store().await;
        let mut session = Session::new(Some("Test".to_string()), None);
        let session_id = session.id.clone();
        
        session.revert = Some(crate::session::types::SessionRevert {
            message_id: "msg-123".to_string(),
            part_id: None,
            snapshot: None,
            diff: None,
            summary: None,
        });
        store.create_session(&session).await.unwrap();

        store.update_session_revert(&session_id, None).await.unwrap();

        let retrieved = store.get_session(&session_id).await.unwrap().unwrap();
        assert!(retrieved.revert.is_none());
    }

    #[tokio::test]
    async fn test_get_messages_until() {
        let store = setup_test_store().await;
        let session = Session::new(None, None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        let msg1 = SessionMessage::new(session_id.clone(), "user".to_string(), "First".to_string());
        let msg1_id = msg1.id.clone();
        let msg2 = SessionMessage::new(session_id.clone(), "assistant".to_string(), "Second".to_string());
        let msg2_id = msg2.id.clone();
        let msg3 = SessionMessage::new(session_id.clone(), "user".to_string(), "Third".to_string());

        store.create_message(&msg1).await.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.create_message(&msg2).await.unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.create_message(&msg3).await.unwrap();

        let messages = store.get_messages_until(&session_id, &msg2_id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].id, msg1_id);
        assert_eq!(messages[1].id, msg2_id);
    }

    #[tokio::test]
    async fn test_get_messages_until_not_found() {
        let store = setup_test_store().await;
        let session = Session::new(None, None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        let result = store.get_messages_until(&session_id, "nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_copy_message_to_session() {
        let store = setup_test_store().await;
        
        let session1 = Session::new(None, None);
        let session1_id = session1.id.clone();
        store.create_session(&session1).await.unwrap();

        let session2 = Session::new(None, None);
        let session2_id = session2.id.clone();
        store.create_session(&session2).await.unwrap();

        let msg = SessionMessage::new(session1_id.clone(), "user".to_string(), "Hello".to_string());
        let original_id = msg.id.clone();

        let new_id = store.copy_message_to_session(&session2_id, &msg).await.unwrap();
        
        assert_ne!(new_id, original_id);
        
        let session2_messages = store.get_messages(&session2_id).await.unwrap();
        assert_eq!(session2_messages.len(), 1);
        assert_eq!(session2_messages[0].content, "Hello");
        assert_eq!(session2_messages[0].id, new_id);
    }
}
