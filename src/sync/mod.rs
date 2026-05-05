//! Event sourcing — SQLite-backed append-only event log.
//!
//! Mirrors the key paths from opencode's sync/index.ts:
//!   append() — insert an event and run projectors
//!   replay()  — re-run a set of serialised events through projectors
//!   list()    — query events by aggregate ID
//!   remove()  — hard-delete events for an aggregate (for GDPR / cleanup)
//!
//! Events are stored in a `sync_event` table (created by an inline schema).
//! Projectors and multi-device sync require a live control-plane connection,
//! so only the local event-log half is implemented here.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqlitePoolOptions}};
use tokio::sync::Mutex;

// ─── types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub id: String,
    /// Sequential position within the aggregate.
    pub seq: u64,
    pub aggregate_id: String,
    pub event_type: String,
    pub version: u32,
    /// JSON payload.
    pub data: serde_json::Value,
    pub created_at: u64,
}

// ─── projector registry ───────────────────────────────────────────────────────

pub type Projector = Arc<dyn Fn(&SyncEvent) + Send + Sync>;

#[derive(Clone, Default)]
pub struct ProjectorRegistry {
    by_type: HashMap<String, Vec<Projector>>,
}

impl ProjectorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, event_type: &str, f: impl Fn(&SyncEvent) + Send + Sync + 'static) {
        self.by_type
            .entry(event_type.to_string())
            .or_default()
            .push(Arc::new(f));
    }

    fn run(&self, event: &SyncEvent) {
        if let Some(projectors) = self.by_type.get(&event.event_type) {
            for p in projectors {
                p(event);
            }
        }
    }
}

// ─── SyncStore ───────────────────────────────────────────────────────────────

pub struct SyncStore {
    pool: SqlitePool,
    projectors: Arc<Mutex<ProjectorRegistry>>,
}

impl SyncStore {
    /// Open (or create) a sync event store at `db_path`.
    pub async fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(opts)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sync_event (
                id          TEXT PRIMARY KEY,
                seq         INTEGER NOT NULL,
                aggregate_id TEXT NOT NULL,
                event_type  TEXT NOT NULL,
                version     INTEGER NOT NULL DEFAULT 1,
                data        TEXT NOT NULL,
                created_at  INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_sync_event_aggregate
             ON sync_event(aggregate_id, seq)",
        )
        .execute(&pool)
        .await?;

        Ok(Self {
            pool,
            projectors: Arc::new(Mutex::new(ProjectorRegistry::new())),
        })
    }

    pub async fn register_projector(
        &self,
        event_type: &str,
        f: impl Fn(&SyncEvent) + Send + Sync + 'static,
    ) {
        self.projectors.lock().await.register(event_type, f);
    }

    /// Append an event to the log and run registered projectors.
    pub async fn append(
        &self,
        aggregate_id: &str,
        event_type: &str,
        version: u32,
        data: serde_json::Value,
    ) -> Result<SyncEvent> {
        let id = crate::util::generate_uuid();
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let seq: u64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(seq), 0) FROM sync_event WHERE aggregate_id = ?",
        )
        .bind(aggregate_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0u64)
            + 1;

        let data_str = serde_json::to_string(&data)?;

        sqlx::query(
            "INSERT INTO sync_event (id, seq, aggregate_id, event_type, version, data, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(seq as i64)
        .bind(aggregate_id)
        .bind(event_type)
        .bind(version as i64)
        .bind(&data_str)
        .bind(created_at as i64)
        .execute(&self.pool)
        .await?;

        let event = SyncEvent {
            id,
            seq,
            aggregate_id: aggregate_id.to_string(),
            event_type: event_type.to_string(),
            version,
            data,
            created_at,
        };

        self.projectors.lock().await.run(&event);
        Ok(event)
    }

    /// List events for an aggregate, ordered by seq.
    pub async fn list(&self, aggregate_id: &str) -> Result<Vec<SyncEvent>> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            seq: i64,
            aggregate_id: String,
            event_type: String,
            version: i64,
            data: String,
            created_at: i64,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, seq, aggregate_id, event_type, version, data, created_at
             FROM sync_event WHERE aggregate_id = ? ORDER BY seq",
        )
        .bind(aggregate_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|r| {
                Ok(SyncEvent {
                    id: r.id,
                    seq: r.seq as u64,
                    aggregate_id: r.aggregate_id,
                    event_type: r.event_type,
                    version: r.version as u32,
                    data: serde_json::from_str(&r.data)?,
                    created_at: r.created_at as u64,
                })
            })
            .collect()
    }

    /// Replay a slice of pre-serialised events through projectors without
    /// persisting them (used for catch-up from the control plane).
    pub async fn replay(&self, events: &[SyncEvent]) -> Result<()> {
        let projectors = self.projectors.lock().await;
        for event in events {
            projectors.run(event);
        }
        Ok(())
    }

    /// Hard-delete all events for `aggregate_id`.
    pub async fn remove(&self, aggregate_id: &str) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM sync_event WHERE aggregate_id = ?")
                .bind(aggregate_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn make_store() -> (SyncStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sync.db");
        (SyncStore::open(&path).await.unwrap(), tmp)
    }

    #[tokio::test]
    async fn append_and_list() {
        let (store, _t) = make_store().await;
        store.append("agg1", "created", 1, serde_json::json!({"x": 1})).await.unwrap();
        store.append("agg1", "updated", 1, serde_json::json!({"x": 2})).await.unwrap();

        let events = store.list("agg1").await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[0].event_type, "created");
    }

    #[tokio::test]
    async fn separate_aggregates() {
        let (store, _t) = make_store().await;
        store.append("a1", "e1", 1, serde_json::json!({})).await.unwrap();
        store.append("a2", "e1", 1, serde_json::json!({})).await.unwrap();

        assert_eq!(store.list("a1").await.unwrap().len(), 1);
        assert_eq!(store.list("a2").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn remove_clears_events() {
        let (store, _t) = make_store().await;
        store.append("agg1", "e1", 1, serde_json::json!({})).await.unwrap();
        store.append("agg1", "e2", 1, serde_json::json!({})).await.unwrap();

        let deleted = store.remove("agg1").await.unwrap();
        assert_eq!(deleted, 2);
        assert!(store.list("agg1").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn projector_fires_on_append() {
        use std::sync::atomic::{AtomicU32, Ordering};
        let counter = Arc::new(AtomicU32::new(0));
        let (store, _t) = make_store().await;

        let c = Arc::clone(&counter);
        store.register_projector("ping", move |_| { c.fetch_add(1, Ordering::Relaxed); }).await;

        store.append("a1", "ping", 1, serde_json::json!({})).await.unwrap();
        store.append("a1", "ping", 1, serde_json::json!({})).await.unwrap();
        store.append("a1", "other", 1, serde_json::json!({})).await.unwrap();

        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn replay_does_not_persist() {
        let (store, _t) = make_store().await;
        let events = vec![SyncEvent {
            id: "e1".into(),
            seq: 1,
            aggregate_id: "agg".into(),
            event_type: "test".into(),
            version: 1,
            data: serde_json::json!({}),
            created_at: 0,
        }];
        store.replay(&events).await.unwrap();
        // replay does not write to DB
        assert!(store.list("agg").await.unwrap().is_empty());
    }
}
