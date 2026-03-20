//! Query functions for session management

use chrono::DateTime;
use sqlx::Row;
use tracing::debug;

use crate::session::{
    error::{Result, SessionError},
    types::{Session, SessionFilter, SessionMessage, SessionStatus},
};

/// Query interface for sessions
pub struct SessionQuery {
    pool: sqlx::SqlitePool,
}

impl SessionQuery {
    /// Create new query interface
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// List sessions with filter
    pub async fn list_sessions(&self, filter: SessionFilter) -> Result<Vec<Session>> {
        debug!("Listing sessions with filter: {:?}", filter);

        let mut query = String::from(
            r#"
            SELECT id, title, created_at, updated_at, status, message_count, metadata
            FROM sessions
            WHERE 1=1
            "#,
        );

        if let Some(ref status) = filter.status {
            let status_str = Self::status_to_string(status);
            query.push_str(&format!(" AND status = '{}'", status_str));
        }

        if let Some(ref title) = filter.title_contains {
            query.push_str(&format!(
                " AND title LIKE '%{}%'",
                title.replace('\'', "''")
            ));
        }

        if let Some(after) = filter.created_after {
            query.push_str(&format!(" AND created_at > '{}'", after.to_rfc3339()));
        }

        if let Some(before) = filter.created_before {
            query.push_str(&format!(" AND created_at < '{}'", before.to_rfc3339()));
        }

        query.push_str(" ORDER BY updated_at DESC");

        if let Some(limit) = filter.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(self.row_to_session(row)?);
        }

        Ok(sessions)
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

    /// Get messages for a session
    pub async fn get_messages(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<SessionMessage>> {
        let mut query = String::from(
            r#"
            SELECT id, session_id, role, content, created_at
            FROM messages
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        );

        if let Some(limit) = limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }

        let rows = sqlx::query(&query)
            .bind(session_id)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(messages)
    }

    /// Search sessions by title
    pub async fn search_sessions(&self, query_str: &str) -> Result<Vec<Session>> {
        debug!("Searching sessions: {}", query_str);

        let filter = SessionFilter::new().with_title_contains(query_str.to_string());
        self.list_sessions(filter).await
    }

    /// Get session history (all sessions ordered by update time)
    pub async fn get_session_history(&self, _session_id: &str) -> Result<Vec<Session>> {
        let filter = SessionFilter::new().with_limit(100);
        self.list_sessions(filter).await
    }

    /// Count total sessions
    pub async fn count_sessions(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM sessions")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    /// Count messages in a session
    pub async fn count_messages(&self, session_id: &str) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM messages WHERE session_id = ?")
            .bind(session_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
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
        let archived_at: Option<i64> = row.try_get("archived_at").ok().flatten();

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
            parent_id: None,
            archived_at,
            revert: None,
            summary: None,
            share_url: None,
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
    use crate::session::store::SessionStore;
    use crate::session::types::{Session, SessionMessage};

    async fn setup_test_query() -> (SessionQuery, SessionStore) {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create test pool");

        let store = SessionStore::new(pool.clone());
        store.initialize().await.expect("Failed to initialize");

        let query = SessionQuery::new(pool);

        (query, store)
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let (query, _store) = setup_test_query().await;
        let sessions = query.list_sessions(SessionFilter::new()).await.unwrap();
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_list_sessions_with_data() {
        let (query, store) = setup_test_query().await;

        for i in 0..3 {
            let session = Session::new(Some(format!("Test {}", i)), None);
            store.create_session(&session).await.unwrap();
        }

        let sessions = query.list_sessions(SessionFilter::new()).await.unwrap();
        assert_eq!(sessions.len(), 3);
    }

    #[tokio::test]
    async fn test_filter_by_status() {
        let (query, store) = setup_test_query().await;

        let active = Session::new(Some("Active".to_string()), None);
        store.create_session(&active).await.unwrap();

        let mut paused = Session::new(Some("Paused".to_string()), None);
        paused.set_status(SessionStatus::Paused);
        store.create_session(&paused).await.unwrap();

        let filter = SessionFilter::new().with_status(SessionStatus::Active);
        let active_sessions = query.list_sessions(filter).await.unwrap();
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].title, "Active");
    }

    #[tokio::test]
    async fn test_search_sessions() {
        let (query, store) = setup_test_query().await;

        store
            .create_session(&Session::new(Some("Project Alpha".to_string()), None))
            .await
            .unwrap();
        store
            .create_session(&Session::new(Some("Project Beta".to_string()), None))
            .await
            .unwrap();
        store
            .create_session(&Session::new(Some("Other".to_string()), None))
            .await
            .unwrap();

        let results = query.search_sessions("Project").await.unwrap();
        assert_eq!(results.len(), 2);

        let results = query.search_sessions("Alpha").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_get_messages() {
        let (query, store) = setup_test_query().await;

        let session = Session::new(None, None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        store
            .create_message(&SessionMessage::new(
                session_id.clone(),
                "user".to_string(),
                "Hello".to_string(),
            ))
            .await
            .unwrap();

        store
            .create_message(&SessionMessage::new(
                session_id.clone(),
                "assistant".to_string(),
                "Hi".to_string(),
            ))
            .await
            .unwrap();

        let messages = query.get_messages(&session_id, None).await.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_get_messages_with_limit() {
        let (query, store) = setup_test_query().await;

        let session = Session::new(None, None);
        let session_id = session.id.clone();
        store.create_session(&session).await.unwrap();

        for i in 0..5 {
            store
                .create_message(&SessionMessage::new(
                    session_id.clone(),
                    "user".to_string(),
                    format!("Msg {}", i),
                ))
                .await
                .unwrap();
        }

        let messages = query.get_messages(&session_id, Some(2)).await.unwrap();
        assert_eq!(messages.len(), 2);
    }

    #[tokio::test]
    async fn test_count_sessions() {
        let (query, store) = setup_test_query().await;

        assert_eq!(query.count_sessions().await.unwrap(), 0);

        for _ in 0..5 {
            store.create_session(&Session::new(None, None)).await.unwrap();
        }

        assert_eq!(query.count_sessions().await.unwrap(), 5);
    }
}
