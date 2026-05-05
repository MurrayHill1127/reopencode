//! JSON-RPC 2.0 client over a language server's stdin/stdout.
//!
//! The LSP framing spec (base protocol):
//!   Content-Length: N\r\n\r\n<N bytes of UTF-8 JSON>
//!
//! We implement the write side synchronously (tokio mutex) and the read side
//! in a background task that dispatches responses into a pending-request map.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::{oneshot, Mutex};

// ─── JSON-RPC primitives ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct Request {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

#[derive(Debug, Deserialize)]
struct Response {
    id: Option<Value>,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

// ─── writer ──────────────────────────────────────────────────────────────────

/// Sends LSP messages to the server's stdin.
///
/// We write to stdin synchronously (via a `std::io::Write`) under a mutex
/// to prevent interleaved frames.
pub struct LspWriter {
    stdin: Arc<Mutex<std::process::ChildStdin>>,
}

impl LspWriter {
    pub fn new(stdin: std::process::ChildStdin) -> Self {
        Self {
            stdin: Arc::new(Mutex::new(stdin)),
        }
    }

    pub async fn send(&self, id: u64, method: &str, params: Value) -> Result<()> {
        let msg = Request {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };
        let body = serde_json::to_vec(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut guard = self.stdin.lock().await;
        guard.write_all(header.as_bytes())?;
        guard.write_all(&body)?;
        guard.flush()?;
        Ok(())
    }

    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        #[derive(Serialize)]
        struct Notification {
            jsonrpc: &'static str,
            method: String,
            params: Value,
        }
        let msg = Notification { jsonrpc: "2.0", method: method.to_string(), params };
        let body = serde_json::to_vec(&msg)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        let mut guard = self.stdin.lock().await;
        guard.write_all(header.as_bytes())?;
        guard.write_all(&body)?;
        guard.flush()?;
        Ok(())
    }
}

// ─── reader / dispatcher ──────────────────────────────────────────────────────

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<Value, RpcError>>>>>;

/// Background task: reads frames from stdout and dispatches to pending callers.
pub fn spawn_reader(stdout: tokio::process::ChildStdout, pending: PendingMap) {
    tokio::spawn(async move {
        if let Err(e) = read_loop(stdout, pending).await {
            tracing::warn!("LSP reader exited: {}", e);
        }
    });
}

async fn read_loop(stdout: ChildStdout, pending: PendingMap) -> Result<()> {
    let mut reader = BufReader::new(stdout);

    loop {
        // Read headers until blank line.
        let mut content_length: Option<usize> = None;
        loop {
            let mut line = String::new();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                return Ok(()); // EOF
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_length = rest.trim().parse().ok();
            }
        }

        let len = match content_length {
            Some(n) => n,
            None => continue,
        };

        let mut body = vec![0u8; len];
        reader.read_exact(&mut body).await?;

        let resp: Response = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(_) => continue, // notification or malformed — skip
        };

        // Numeric id → resolve pending caller.
        if let Some(id_val) = resp.id.as_ref().and_then(|v| v.as_u64()) {
            let mut map = pending.lock().await;
            if let Some(tx) = map.remove(&id_val) {
                let result = if let Some(err) = resp.error {
                    Err(err)
                } else {
                    Ok(resp.result.unwrap_or(Value::Null))
                };
                let _ = tx.send(result);
            }
        }
        // Notifications (null id) are silently discarded for now.
    }
}

// ─── LspClient ───────────────────────────────────────────────────────────────

pub struct LspClient {
    writer: LspWriter,
    pending: PendingMap,
    next_id: Arc<Mutex<u64>>,
}

impl LspClient {
    pub fn new(stdin: std::process::ChildStdin, stdout: tokio::process::ChildStdout) -> Self {
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        spawn_reader(stdout, Arc::clone(&pending));
        Self {
            writer: LspWriter::new(stdin),
            pending,
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    async fn next_id(&self) -> u64 {
        let mut id = self.next_id.lock().await;
        let v = *id;
        *id += 1;
        v
    }

    /// Send a request and await the response with a 30-second timeout.
    pub async fn request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id().await;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.writer.send(id, method, params).await?;

        let result = tokio::time::timeout(std::time::Duration::from_secs(30), rx).await;
        match result {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => bail!("LSP error {}: {}", e.code, e.message),
            Ok(Err(_)) => bail!("LSP channel closed"),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                bail!("LSP request timed out: {}", method)
            }
        }
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Value) -> Result<()> {
        self.writer.notify(method, params).await
    }
}
