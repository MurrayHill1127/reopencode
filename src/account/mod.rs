//! Account management — OAuth device-flow login, token storage, org selection.
//!
//! Mirrors the key paths from opencode's account/account.ts + repo.ts:
//!   login()   — POST /auth/device → user_code + verification URL
//!   poll()    — POST /auth/device/token → access_token or pending/expired/denied
//!   token()   — return access token, refreshing if near expiry
//!   list()    — list persisted accounts
//!   remove()  — delete an account

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ─── types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: String,
    pub email: String,
    pub url: String,
    pub active_org_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInfo {
    pub id: String,
    pub name: String,
}

/// Result of a device-flow login initiation.
#[derive(Debug, Clone)]
pub struct LoginFlow {
    pub device_code: String,
    pub user_code: String,
    /// URL the user should visit in their browser.
    pub verification_url: String,
    pub server_url: String,
    pub expires_in: Duration,
    pub interval: Duration,
}

/// One polling iteration result.
#[derive(Debug, Clone)]
pub enum PollResult {
    Success { email: String },
    Pending,
    SlowDown,
    Expired,
    Denied,
    Error(String),
}

// ─── persistence ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AccountState {
    accounts: Vec<StoredAccount>,
    active_id: Option<String>,
    active_org_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAccount {
    id: String,
    email: String,
    url: String,
    access_token: String,
    refresh_token: String,
    /// Unix timestamp in milliseconds; None means no expiry known.
    token_expiry: Option<u64>,
}

impl StoredAccount {
    fn info(&self, active_org_id: Option<String>) -> AccountInfo {
        AccountInfo {
            id: self.id.clone(),
            email: self.email.clone(),
            url: self.url.clone(),
            active_org_id,
        }
    }

    fn is_token_fresh(&self) -> bool {
        let Some(expiry) = self.token_expiry else { return false };
        let now_ms = now_ms();
        // Refresh 5 minutes early.
        expiry > now_ms + 5 * 60 * 1000
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn normalize_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    url.to_string()
}

// ─── AccountStore ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AccountStore {
    path: PathBuf,
    state: Arc<Mutex<AccountState>>,
}

impl AccountStore {
    /// Open or create the account store at `path`.
    pub async fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let state = if path.exists() {
            let bytes = tokio::fs::read(&path).await?;
            serde_json::from_slice(&bytes).unwrap_or_default()
        } else {
            AccountState::default()
        };

        Ok(Self {
            path,
            state: Arc::new(Mutex::new(state)),
        })
    }

    async fn save(&self, state: &AccountState) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(state)?;
        tokio::fs::write(&self.path, bytes).await?;
        Ok(())
    }

    pub async fn active(&self) -> Option<AccountInfo> {
        let state = self.state.lock().await;
        let active_id = state.active_id.as_deref()?;
        let acc = state.accounts.iter().find(|a| a.id == active_id)?;
        Some(acc.info(state.active_org_id.clone()))
    }

    pub async fn list(&self) -> Vec<AccountInfo> {
        let state = self.state.lock().await;
        state
            .accounts
            .iter()
            .map(|a| a.info(None))
            .collect()
    }

    pub async fn remove(&self, account_id: &str) -> Result<()> {
        let mut state = self.state.lock().await;
        state.accounts.retain(|a| a.id != account_id);
        if state.active_id.as_deref() == Some(account_id) {
            state.active_id = None;
            state.active_org_id = None;
        }
        self.save(&state).await
    }

    pub async fn set_active(&self, account_id: &str, org_id: Option<String>) -> Result<()> {
        let mut state = self.state.lock().await;
        state.active_id = Some(account_id.to_string());
        state.active_org_id = org_id;
        self.save(&state).await
    }

    async fn persist(
        &self,
        id: &str,
        email: &str,
        url: &str,
        access_token: &str,
        refresh_token: &str,
        expiry: Option<u64>,
    ) -> Result<()> {
        let mut state = self.state.lock().await;
        let url = normalize_url(url);
        if let Some(existing) = state.accounts.iter_mut().find(|a| a.id == id) {
            existing.email = email.to_string();
            existing.url = url;
            existing.access_token = access_token.to_string();
            existing.refresh_token = refresh_token.to_string();
            existing.token_expiry = expiry;
        } else {
            state.accounts.push(StoredAccount {
                id: id.to_string(),
                email: email.to_string(),
                url,
                access_token: access_token.to_string(),
                refresh_token: refresh_token.to_string(),
                token_expiry: expiry,
            });
            state.active_id = Some(id.to_string());
        }
        self.save(&state).await
    }
}

// ─── AccountService ───────────────────────────────────────────────────────────

const CLIENT_ID: &str = "opencode-cli";

pub struct AccountService {
    store: AccountStore,
    client: reqwest::Client,
}

impl AccountService {
    pub fn new(store: AccountStore) -> Self {
        Self {
            store,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }

    /// Start a device-flow login against `server_url`.
    pub async fn login(&self, server_url: &str) -> Result<LoginFlow> {
        let url = normalize_url(server_url);
        let endpoint = format!("{}/auth/device", url);

        #[derive(Deserialize)]
        struct DeviceAuthResponse {
            device_code: String,
            user_code: String,
            verification_uri_complete: String,
            expires_in: u64,
            interval: u64,
        }

        #[derive(Serialize)]
        struct DeviceAuthRequest<'a> {
            client_id: &'a str,
        }

        let resp = self
            .client
            .post(&endpoint)
            .json(&DeviceAuthRequest { client_id: CLIENT_ID })
            .send()
            .await
            .with_context(|| format!("POST {}", endpoint))?
            .error_for_status()?
            .json::<DeviceAuthResponse>()
            .await?;

        Ok(LoginFlow {
            device_code: resp.device_code,
            user_code: resp.user_code,
            verification_url: resp.verification_uri_complete,
            server_url: url,
            expires_in: Duration::from_secs(resp.expires_in),
            interval: Duration::from_secs(resp.interval.max(1)),
        })
    }

    /// Poll the device-flow token endpoint once.
    pub async fn poll(&self, flow: &LoginFlow) -> Result<PollResult> {
        let endpoint = format!("{}/auth/device/token", flow.server_url);

        #[derive(Serialize)]
        struct TokenRequest<'a> {
            grant_type: &'a str,
            device_code: &'a str,
            client_id: &'a str,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TokenResponse {
            Success {
                access_token: String,
                refresh_token: String,
                expires_in: u64,
                token_type: String,
            },
            Error {
                error: String,
                #[allow(dead_code)]
                error_description: String,
            },
        }

        #[derive(Deserialize)]
        struct UserResponse {
            id: String,
            email: String,
        }

        let resp = self
            .client
            .post(&endpoint)
            .json(&TokenRequest {
                grant_type: "urn:ietf:params:oauth:grant-type:device_code",
                device_code: &flow.device_code,
                client_id: CLIENT_ID,
            })
            .send()
            .await?;

        let token: TokenResponse = resp.json().await?;

        match token {
            TokenResponse::Success {
                access_token,
                refresh_token,
                expires_in,
                ..
            } => {
                // Fetch user info to get account id + email.
                let user_url = format!("{}/api/user", flow.server_url);
                let user: UserResponse = self
                    .client
                    .get(&user_url)
                    .bearer_auth(&access_token)
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;

                let expiry = now_ms() + expires_in * 1000;
                self.store
                    .persist(
                        &user.id,
                        &user.email,
                        &flow.server_url,
                        &access_token,
                        &refresh_token,
                        Some(expiry),
                    )
                    .await?;

                Ok(PollResult::Success { email: user.email })
            }
            TokenResponse::Error { error, .. } => Ok(match error.as_str() {
                "authorization_pending" => PollResult::Pending,
                "slow_down" => PollResult::SlowDown,
                "expired_token" => PollResult::Expired,
                "access_denied" => PollResult::Denied,
                other => PollResult::Error(other.to_string()),
            }),
        }
    }

    /// Return the active account's current access token, refreshing if needed.
    pub async fn token(&self) -> Result<Option<String>> {
        let state = self.store.state.lock().await;
        let Some(active_id) = &state.active_id else { return Ok(None) };
        let Some(acc) = state.accounts.iter().find(|a| &a.id == active_id) else {
            return Ok(None);
        };

        if acc.is_token_fresh() {
            return Ok(Some(acc.access_token.clone()));
        }

        let refresh_token = acc.refresh_token.clone();
        let url = acc.url.clone();
        let id = acc.id.clone();
        drop(state); // release lock before async calls

        self.refresh_token(&id, &url, &refresh_token).await
    }

    async fn refresh_token(
        &self,
        account_id: &str,
        server_url: &str,
        refresh_token: &str,
    ) -> Result<Option<String>> {
        let endpoint = format!("{}/auth/device/token", server_url);

        #[derive(Serialize)]
        struct RefreshRequest<'a> {
            grant_type: &'a str,
            refresh_token: &'a str,
            client_id: &'a str,
        }
        #[derive(Deserialize)]
        struct RefreshResponse {
            access_token: String,
            refresh_token: String,
            expires_in: u64,
        }

        let resp: RefreshResponse = self
            .client
            .post(&endpoint)
            .json(&RefreshRequest {
                grant_type: "refresh_token",
                refresh_token,
                client_id: CLIENT_ID,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let expiry = now_ms() + resp.expires_in * 1000;
        let mut state = self.store.state.lock().await;
        if let Some(acc) = state.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.access_token = resp.access_token.clone();
            acc.refresh_token = resp.refresh_token;
            acc.token_expiry = Some(expiry);
        }
        self.store.save(&state).await?;
        Ok(Some(resp.access_token))
    }

    /// List orgs available to the active account.
    pub async fn orgs(&self) -> Result<Vec<OrgInfo>> {
        let Some(token) = self.token().await? else {
            bail!("not logged in");
        };
        let active = self.store.active().await.ok_or_else(|| anyhow::anyhow!("not logged in"))?;
        let url = format!("{}/api/orgs", active.url);

        let orgs: Vec<OrgInfo> = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(orgs)
    }

    pub fn store(&self) -> &AccountStore {
        &self.store
    }
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn make_store() -> (AccountStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("accounts.json");
        (AccountStore::open(path).await.unwrap(), tmp)
    }

    #[tokio::test]
    async fn store_empty() {
        let (store, _t) = make_store().await;
        assert!(store.active().await.is_none());
        assert!(store.list().await.is_empty());
    }

    #[tokio::test]
    async fn store_persist_and_list() {
        let (store, _t) = make_store().await;
        store.persist("acc1", "alice@example.com", "https://opencode.ai", "tok1", "ref1", None).await.unwrap();
        let list = store.list().await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].email, "alice@example.com");
    }

    #[tokio::test]
    async fn store_active_default_to_first() {
        let (store, _t) = make_store().await;
        store.persist("acc1", "a@b.com", "https://opencode.ai", "t", "r", None).await.unwrap();
        let active = store.active().await;
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, "acc1");
    }

    #[tokio::test]
    async fn store_remove() {
        let (store, _t) = make_store().await;
        store.persist("acc1", "a@b.com", "https://x.com", "t", "r", None).await.unwrap();
        store.remove("acc1").await.unwrap();
        assert!(store.list().await.is_empty());
        assert!(store.active().await.is_none());
    }

    #[tokio::test]
    async fn normalize_url_strips_trailing_slash() {
        assert_eq!(normalize_url("https://opencode.ai/"), "https://opencode.ai");
        assert_eq!(normalize_url("https://opencode.ai"), "https://opencode.ai");
    }

    #[test]
    fn token_freshness() {
        let mut acc = StoredAccount {
            id: "x".into(),
            email: "a@b.com".into(),
            url: "https://x".into(),
            access_token: "tok".into(),
            refresh_token: "ref".into(),
            token_expiry: None,
        };
        assert!(!acc.is_token_fresh());

        // Expiry 10 minutes in the future → fresh.
        acc.token_expiry = Some(now_ms() + 10 * 60 * 1000);
        assert!(acc.is_token_fresh());

        // Expiry 3 minutes in the future → stale (< 5-min threshold).
        acc.token_expiry = Some(now_ms() + 3 * 60 * 1000);
        assert!(!acc.is_token_fresh());
    }
}
