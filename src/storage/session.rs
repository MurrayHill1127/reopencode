//! Session storage operations

use std::sync::Arc;

use crate::storage::{
    MemoryCache, MessageInfo, MessagePart, MessageRecord, MessageRole, MessageTime,
    MessageWithParts, Page, SessionRecord, StorageBackend, StorageError, TodoItem, TodoPriority,
    TodoStatus,
    backend::{read, write},
};

/// Generate a session ID
pub fn generate_session_id() -> String {
    format!(
        "ses_{}",
        &crate::util::generate_uuid().replace('-', "")[..16]
    )
}

/// Generate a message ID
pub fn generate_message_id() -> String {
    format!(
        "msg_{}",
        &crate::util::generate_uuid().replace('-', "")[..16]
    )
}

/// Generate a part ID
pub fn generate_part_id() -> String {
    format!(
        "part_{}",
        &crate::util::generate_uuid().replace('-', "")[..16]
    )
}

// ==================== SessionStore ====================

/// Session creation input
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionCreateInput {
    pub project_id: String,
    pub workspace_id: Option<String>,
    pub parent_id: Option<String>,
    pub slug: String,
    pub directory: String,
    pub title: Option<String>,
}

impl Default for SessionCreateInput {
    fn default() -> Self {
        Self {
            project_id: "default".to_string(),
            workspace_id: None,
            parent_id: None,
            slug: crate::util::generate_uuid()[..8].to_string(),
            directory: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            title: None,
        }
    }
}

/// Session list options
#[derive(Debug, Clone, Default)]
pub struct SessionListOptions {
    pub directory: Option<String>,
    pub roots: bool,
    pub start: Option<i64>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

/// Session storage
pub struct SessionStore {
    backend: Arc<dyn StorageBackend>,
    cache: MemoryCache,
}

impl SessionStore {
    pub fn new(backend: Arc<dyn StorageBackend>, cache: &MemoryCache) -> Self {
        Self {
            backend,
            cache: cache.clone(),
        }
    }

    /// Create a new session
    pub async fn create(&self, input: SessionCreateInput) -> Result<SessionRecord, StorageError> {
        let now = chrono::Utc::now().timestamp_millis();
        let session = SessionRecord {
            id: generate_session_id(),
            project_id: input.project_id,
            workspace_id: input.workspace_id,
            parent_id: input.parent_id,
            slug: input.slug,
            directory: input.directory,
            title: input.title.unwrap_or_else(|| "New Session".to_string()),
            version: "0.1.0".to_string(),
            time_created: now,
            time_updated: now,
            ..Default::default()
        };

        let key = [
            "session",
            &session.project_id,
            &format!("{}.json", session.id),
        ];
        write(&*self.backend, &key, &session).await?;

        self.cache
            .set(format!("session:{}", session.id), session.clone());

        Ok(session)
    }

    /// Get a session by ID
    pub async fn get(&self, session_id: &str) -> Result<Option<SessionRecord>, StorageError> {
        if let Some(cached) = self
            .cache
            .get::<SessionRecord>(&format!("session:{}", session_id))
        {
            return Ok(Some(cached));
        }

        let keys = self.backend.list(&["session"]).await?;

        for key in keys {
            if key.len() >= 3 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(session) = read::<SessionRecord>(&*self.backend, &key_refs).await? {
                    if session.id == session_id {
                        self.cache
                            .set(format!("session:{}", session_id), session.clone());
                        return Ok(Some(session));
                    }
                }
            }
        }

        Ok(None)
    }

    /// List sessions with filtering
    pub async fn list(
        &self,
        options: SessionListOptions,
    ) -> Result<Vec<SessionRecord>, StorageError> {
        let mut sessions = Vec::new();

        let keys = self.backend.list(&["session"]).await?;

        for key in keys {
            if key.len() >= 3 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(session) = read::<SessionRecord>(&*self.backend, &key_refs).await? {
                    if let Some(ref directory) = options.directory {
                        if session.directory != *directory {
                            continue;
                        }
                    }

                    if options.roots && session.parent_id.is_some() {
                        continue;
                    }

                    if let Some(start) = options.start {
                        if session.time_updated < start {
                            continue;
                        }
                    }

                    if let Some(ref search) = options.search {
                        if !session
                            .title
                            .to_lowercase()
                            .contains(&search.to_lowercase())
                        {
                            continue;
                        }
                    }

                    sessions.push(session);
                }
            }

            if let Some(limit) = options.limit {
                if sessions.len() >= limit {
                    break;
                }
            }
        }

        sessions.sort_by(|a, b| b.time_updated.cmp(&a.time_updated));

        Ok(sessions)
    }

    /// Delete a session
    pub async fn remove(&self, session_id: &str) -> Result<(), StorageError> {
        if let Some(session) = self.get(session_id).await? {
            let key = [
                "session",
                &session.project_id,
                &format!("{}.json", session_id),
            ];
            self.backend.remove(&key).await?;
            self.cache.remove(&format!("session:{}", session_id));
        }
        Ok(())
    }

    /// Update session title
    pub async fn set_title(
        &self,
        session_id: &str,
        title: &str,
    ) -> Result<SessionRecord, StorageError> {
        let mut session = self
            .get(session_id)
            .await?
            .ok_or_else(|| StorageError::NotFound(session_id.to_string()))?;

        session.title = title.to_string();
        session.time_updated = chrono::Utc::now().timestamp_millis();

        let key = [
            "session",
            &session.project_id,
            &format!("{}.json", session_id),
        ];
        write(&*self.backend, &key, &session).await?;

        self.cache
            .set(format!("session:{}", session_id), session.clone());

        Ok(session)
    }
}

// ==================== MessageStore ====================

/// Message storage
pub struct MessageStore {
    backend: Arc<dyn StorageBackend>,
    cache: MemoryCache,
}

impl MessageStore {
    pub fn new(backend: Arc<dyn StorageBackend>, cache: &MemoryCache) -> Self {
        Self {
            backend,
            cache: cache.clone(),
        }
    }

    /// List all messages for a session
    pub async fn list(&self, session_id: &str) -> Result<Vec<MessageWithParts>, StorageError> {
        let cache_key = format!("messages:{}", session_id);

        if let Some(cached) = self.cache.get::<Vec<MessageWithParts>>(&cache_key) {
            return Ok(cached);
        }

        let keys = self.backend.list(&["message", session_id]).await?;
        let mut messages = Vec::new();

        for key in keys {
            if key.len() >= 3 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(record) = read::<MessageRecord>(&*self.backend, &key_refs).await? {
                    let parts = self.load_parts(&record.id).await?;

                    let role: MessageRole = serde_json::from_value(
                        record
                            .data
                            .get("role")
                            .cloned()
                            .unwrap_or(serde_json::json!("user")),
                    )
                    .unwrap_or(MessageRole::User);

                    let agent = record
                        .data
                        .get("agent")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    messages.push(MessageWithParts {
                        info: MessageInfo {
                            id: record.id,
                            session_id: record.session_id,
                            role,
                            agent,
                            time: MessageTime {
                                created: record.time_created,
                                updated: Some(record.time_updated),
                            },
                        },
                        parts,
                    });
                }
            }
        }

        messages.sort_by_key(|m| m.info.time.created);

        self.cache.set(cache_key.clone(), messages.clone());

        Ok(messages)
    }

    /// Get paginated messages
    pub async fn page(
        &self,
        session_id: &str,
        limit: usize,
        before: Option<&str>,
    ) -> Result<Page<MessageWithParts>, StorageError> {
        let all_messages = self.list(session_id).await?;

        let start_idx = if let Some(cursor) = before {
            all_messages
                .iter()
                .position(|m| m.info.id == cursor)
                .unwrap_or(all_messages.len())
        } else {
            all_messages.len()
        };

        let end_idx = start_idx.saturating_sub(limit);
        let items: Vec<MessageWithParts> = all_messages
            .into_iter()
            .skip(end_idx)
            .take(start_idx - end_idx)
            .collect();

        let cursor = if end_idx > 0 {
            items.first().map(|m| m.info.id.clone())
        } else {
            None
        };

        Ok(Page { items, cursor })
    }

    /// Load parts for a message
    async fn load_parts(&self, message_id: &str) -> Result<Vec<MessagePart>, StorageError> {
        let keys = self.backend.list(&["part", message_id]).await?;
        let mut parts = Vec::new();

        for key in keys {
            if key.len() >= 3 {
                let key_refs: Vec<&str> = key.iter().map(String::as_str).collect();
                if let Some(part) = read::<MessagePart>(&*self.backend, &key_refs).await? {
                    parts.push(part);
                }
            }
        }

        Ok(parts)
    }

    /// Create a message
    pub async fn create(
        &self,
        session_id: &str,
        info: MessageInfo,
        parts: Vec<MessagePart>,
    ) -> Result<MessageWithParts, StorageError> {
        let now = chrono::Utc::now().timestamp_millis();

        let record = MessageRecord {
            id: info.id.clone(),
            session_id: session_id.to_string(),
            time_created: now,
            time_updated: now,
            data: serde_json::json!({
                "role": info.role,
                "agent": info.agent,
            }),
        };

        let key = ["message", session_id, &format!("{}.json", info.id)];
        write(&*self.backend, &key, &record).await?;

        for part in &parts {
            let part_key = ["part", &info.id, &format!("{}.json", part.id())];
            write(&*self.backend, &part_key, part).await?;
        }

        self.cache.remove(&format!("messages:{}", session_id));

        Ok(MessageWithParts { info, parts })
    }

    /// Delete a message
    pub async fn remove(&self, session_id: &str, message_id: &str) -> Result<(), StorageError> {
        let keys = self.backend.list(&["part", message_id]).await?;
        for key in keys {
            if key.len() >= 3 {
                let key_refs: Vec<&str> = key.iter().map(|s| s.as_str()).collect();
                self.backend.remove(&key_refs).await?;
            }
        }

        let key = ["message", session_id, &format!("{}.json", message_id)];
        self.backend.remove(&key).await?;

        self.cache.remove(&format!("messages:{}", session_id));

        Ok(())
    }
}

// ==================== TodoStore ====================

/// Todo storage
pub struct TodoStore {
    backend: Arc<dyn StorageBackend>,
}

impl TodoStore {
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }

    /// Get todos for a session
    pub async fn get(&self, session_id: &str) -> Result<Vec<TodoItem>, StorageError> {
        let key = ["todo", &format!("{}.json", session_id)];
        Ok(read::<Vec<TodoItem>>(&*self.backend, &key)
            .await?
            .unwrap_or_default())
    }

    /// Set todos for a session
    pub async fn set(&self, session_id: &str, todos: Vec<TodoItem>) -> Result<(), StorageError> {
        let key = ["todo", &format!("{}.json", session_id)];
        write(&*self.backend, &key, &todos).await
    }

    /// Add a todo
    pub async fn add(
        &self,
        session_id: &str,
        content: String,
        priority: TodoPriority,
    ) -> Result<TodoItem, StorageError> {
        let mut todos = self.get(session_id).await?;
        let position = todos.len() as i32;

        let mut todo = TodoItem::new(content, position);
        todo.priority = priority;

        todos.push(todo.clone());
        self.set(session_id, todos).await?;

        Ok(todo)
    }

    /// Update todo status
    pub async fn update_status(
        &self,
        session_id: &str,
        todo_id: &str,
        status: TodoStatus,
    ) -> Result<(), StorageError> {
        let mut todos = self.get(session_id).await?;

        if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
            todo.status = status;
            self.set(session_id, todos).await?;
        }

        Ok(())
    }

    /// Remove a todo
    pub async fn remove(&self, session_id: &str, todo_id: &str) -> Result<(), StorageError> {
        let mut todos = self.get(session_id).await?;
        todos.retain(|t| t.id != todo_id);

        // Re-position
        for (i, todo) in todos.iter_mut().enumerate() {
            todo.position = i as i32;
        }

        self.set(session_id, todos).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{JsonBackend, cache::CacheConfig};
    use tempfile::TempDir;

    fn create_test_store() -> (SessionStore, MessageStore, TodoStore, TempDir) {
        let temp = TempDir::new().unwrap();
        let backend = Arc::new(JsonBackend::new(temp.path()).unwrap());
        let cache = MemoryCache::new(CacheConfig::default());

        (
            SessionStore::new(backend.clone(), &cache),
            MessageStore::new(backend.clone(), &cache),
            TodoStore::new(backend),
            temp,
        )
    }

    #[tokio::test]
    async fn test_session_create() {
        let (store, _, _, _temp) = create_test_store();

        let session = store
            .create(SessionCreateInput {
                project_id: "proj_test".to_string(),
                slug: "test-slug".to_string(),
                directory: "/tmp".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(session.id.starts_with("ses_"));
        assert_eq!(session.project_id, "proj_test");
        assert_eq!(session.title, "New Session");
    }

    #[tokio::test]
    async fn test_session_get() {
        let (store, _, _, _temp) = create_test_store();

        let created = store
            .create(SessionCreateInput {
                project_id: "proj_test".to_string(),
                slug: "test-slug".to_string(),
                directory: "/tmp".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        let loaded = store.get(&created.id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().id, created.id);
    }

    #[tokio::test]
    async fn test_session_list() {
        let (store, _, _, _temp) = create_test_store();

        store
            .create(SessionCreateInput {
                project_id: "proj1".to_string(),
                slug: "s1".to_string(),
                directory: "/dir1".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        store
            .create(SessionCreateInput {
                project_id: "proj2".to_string(),
                slug: "s2".to_string(),
                directory: "/dir2".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        let sessions = store.list(SessionListOptions::default()).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_session_remove() {
        let (store, _, _, _temp) = create_test_store();

        let session = store
            .create(SessionCreateInput {
                project_id: "proj_test".to_string(),
                slug: "test-slug".to_string(),
                directory: "/tmp".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        store.remove(&session.id).await.unwrap();

        let loaded = store.get(&session.id).await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_message_create_and_list() {
        let (_, store, _, _temp) = create_test_store();

        let session_id = "ses_test123";
        let info = MessageInfo {
            id: generate_message_id(),
            session_id: session_id.to_string(),
            role: MessageRole::User,
            agent: None,
            time: MessageTime {
                created: 0,
                updated: None,
            },
        };

        let parts = vec![MessagePart::Text {
            id: generate_part_id(),
            text: "Hello".to_string(),
        }];

        store
            .create(session_id, info.clone(), parts.clone())
            .await
            .unwrap();

        let messages = store.list(session_id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].info.id, info.id);
    }

    #[tokio::test]
    async fn test_todo_operations() {
        let (_, _, store, _temp) = create_test_store();

        let session_id = "ses_todo_test";

        let todo1 = store
            .add(session_id, "Task 1".to_string(), TodoPriority::High)
            .await
            .unwrap();
        let todo2 = store
            .add(session_id, "Task 2".to_string(), TodoPriority::Low)
            .await
            .unwrap();

        let todos = store.get(session_id).await.unwrap();
        assert_eq!(todos.len(), 2);

        store
            .update_status(session_id, &todo1.id, TodoStatus::Completed)
            .await
            .unwrap();

        let todos = store.get(session_id).await.unwrap();
        let completed = todos.iter().find(|t| t.id == todo1.id).unwrap();
        assert_eq!(completed.status, TodoStatus::Completed);

        store.remove(session_id, &todo2.id).await.unwrap();

        let todos = store.get(session_id).await.unwrap();
        assert_eq!(todos.len(), 1);
    }

    #[test]
    fn test_id_generation() {
        let session_id = generate_session_id();
        assert!(session_id.starts_with("ses_"));
        assert_eq!(session_id.len(), 20); // ses_ + 16 chars

        let message_id = generate_message_id();
        assert!(message_id.starts_with("msg_"));

        let part_id = generate_part_id();
        assert!(part_id.starts_with("part_"));
    }
}
