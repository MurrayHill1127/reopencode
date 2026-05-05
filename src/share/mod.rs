//! Session sharing — create/remove public share links via the control plane.
//!
//! The control plane endpoint can be overridden with `OPENCODE_SHARE_API`
//! (defaults to https://api.opencode.ai).  Set `OPENCODE_DISABLE_SHARE=1` to
//! disable sharing entirely.
//!
//! Mirrors the key paths from opencode's share/share-next.ts:
//!   create() — POST /share → { id, url, secret }
//!   remove() — DELETE /share/{id}
//!
//! Share records are persisted locally so that shares survive process restarts
//! and can be removed later.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ─── types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareInfo {
    pub id: String,
    pub url: String,
    pub secret: String,
    pub session_id: String,
}

// ─── config ──────────────────────────────────────────────────────────────────

fn api_base() -> String {
    std::env::var("OPENCODE_SHARE_API")
        .unwrap_or_else(|_| "https://api.opencode.ai".to_string())
}

pub fn is_disabled() -> bool {
    matches!(
        std::env::var("OPENCODE_DISABLE_SHARE").as_deref(),
        Ok("1") | Ok("true")
    )
}

// ─── local store ─────────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
struct ShareStore {
    shares: HashMap<String, ShareInfo>, // session_id → share
}

impl ShareStore {
    fn load(path: &std::path::Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &std::path::Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

// ─── ShareManager ─────────────────────────────────────────────────────────────

/// Manages session shares for a single project.
pub struct ShareManager {
    store_path: PathBuf,
    inner: Arc<Mutex<ShareStore>>,
    client: reqwest::Client,
}

impl ShareManager {
    pub fn new(store_path: PathBuf) -> Self {
        let inner = ShareStore::load(&store_path);
        Self {
            store_path,
            inner: Arc::new(Mutex::new(inner)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    /// Return the existing share URL for `session_id`, if any.
    pub async fn get(&self, session_id: &str) -> Option<ShareInfo> {
        self.inner.lock().await.shares.get(session_id).cloned()
    }

    /// Create a share link for `session_id`, posting a minimal payload.
    ///
    /// `access_token` is the account bearer token (may be None for anonymous).
    /// `session_data` is arbitrary JSON (e.g. the full session record).
    pub async fn create(
        &self,
        session_id: &str,
        access_token: Option<&str>,
        session_data: serde_json::Value,
    ) -> Result<ShareInfo> {
        if is_disabled() {
            return Ok(ShareInfo {
                id: uuid::Uuid::new_v4().to_string(),
                url: format!("https://opencode.ai/s/disabled-{}", session_id),
                secret: "disabled".to_string(),
                session_id: session_id.to_string(),
            });
        }

        // Check if already shared.
        if let Some(existing) = self.get(session_id).await {
            return Ok(existing);
        }

        let api = api_base();
        let endpoint = format!("{}/share", api);

        #[derive(Serialize)]
        struct CreatePayload<'a> {
            session_id: &'a str,
            data: serde_json::Value,
        }
        #[derive(Deserialize)]
        struct CreateResponse {
            id: String,
            url: String,
            secret: String,
        }

        let mut req = self.client.post(&endpoint).json(&CreatePayload {
            session_id,
            data: session_data,
        });
        if let Some(tok) = access_token {
            req = req.bearer_auth(tok);
        }

        let resp: CreateResponse = req
            .send()
            .await
            .with_context(|| format!("POST {}", endpoint))?
            .error_for_status()?
            .json()
            .await?;

        let info = ShareInfo {
            id: resp.id,
            url: resp.url,
            secret: resp.secret,
            session_id: session_id.to_string(),
        };

        let mut store = self.inner.lock().await;
        store.shares.insert(session_id.to_string(), info.clone());
        store.save(&self.store_path)?;

        Ok(info)
    }

    /// Remove the share for `session_id`, notifying the server and clearing
    /// the local record.
    pub async fn remove(&self, session_id: &str, access_token: Option<&str>) -> Result<bool> {
        let mut store = self.inner.lock().await;
        let Some(info) = store.shares.remove(session_id) else {
            return Ok(false);
        };
        store.save(&self.store_path)?;
        drop(store);

        if is_disabled() {
            return Ok(true);
        }

        let api = api_base();
        let endpoint = format!("{}/share/{}", api, info.id);
        let mut req = self.client.delete(&endpoint).header("x-share-secret", &info.secret);
        if let Some(tok) = access_token {
            req = req.bearer_auth(tok);
        }

        // Best-effort — if the request fails we still removed the local record.
        let _ = req.send().await;
        Ok(true)
    }

    /// List all locally tracked shares.
    pub async fn list(&self) -> Vec<ShareInfo> {
        self.inner.lock().await.shares.values().cloned().collect()
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_mgr() -> (ShareManager, TempDir) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("shares.json");
        (ShareManager::new(path), tmp)
    }

    #[tokio::test]
    async fn empty_list() {
        let (mgr, _t) = make_mgr();
        assert!(mgr.list().await.is_empty());
    }

    #[tokio::test]
    async fn get_missing() {
        let (mgr, _t) = make_mgr();
        assert!(mgr.get("ses_xyz").await.is_none());
    }

    #[tokio::test]
    async fn disabled_create_returns_stub() {
        unsafe { std::env::set_var("OPENCODE_DISABLE_SHARE", "1"); }
        let (mgr, _t) = make_mgr();
        let info = mgr.create("ses1", None, serde_json::json!({})).await.unwrap();
        assert!(info.url.contains("disabled"));
        unsafe { std::env::remove_var("OPENCODE_DISABLE_SHARE"); }
    }

    #[tokio::test]
    async fn disabled_remove_returns_false_when_not_found() {
        let (mgr, _t) = make_mgr();
        let removed = mgr.remove("ses_none", None).await.unwrap();
        assert!(!removed);
    }

    #[test]
    fn is_disabled_default() {
        unsafe { std::env::remove_var("OPENCODE_DISABLE_SHARE"); }
        assert!(!is_disabled());
    }
}
