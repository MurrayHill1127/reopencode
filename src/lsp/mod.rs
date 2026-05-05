//! Language Server Protocol (LSP) integration.
//!
//! Manages language server processes and provides hover/definition/references
//! queries against them.  One server process per (language, root-directory).

pub mod client;
pub mod language;
pub mod server;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Result};
use serde_json::Value;
use tokio::sync::Mutex;

use server::LspServer;

/// Manages a pool of language servers, keyed by `(language_id, root_dir)`.
pub struct LspManager {
    servers: Arc<Mutex<HashMap<(String, PathBuf), Arc<LspServer>>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Clone the Arc for the server managing `file`, starting it if needed.
    async fn acquire(&self, file: &Path, root: &Path) -> Result<Arc<LspServer>> {
        let lang = match language::language_id(file) {
            Some(l) => l.to_string(),
            None => bail!("no language ID for file: {}", file.display()),
        };
        let cmd = match language::server_command(&lang) {
            Some(c) => c,
            None => bail!("no language server configured for: {}", lang),
        };
        let key = (lang.clone(), root.to_path_buf());

        {
            let guard = self.servers.lock().await;
            if let Some(srv) = guard.get(&key) {
                return Ok(Arc::clone(srv));
            }
        }

        // Start outside the lock; another task may race us — that's fine, the
        // loser's server will be dropped when we re-lock below.
        let srv = Arc::new(LspServer::start(&lang, root, &cmd).await?);

        let mut guard = self.servers.lock().await;
        Ok(Arc::clone(guard.entry(key).or_insert(srv)))
    }

    /// Open `file` in its language server (sends textDocument/didOpen).
    pub async fn open(&self, file: &Path, root: &Path, content: &str) -> Result<()> {
        let srv = self.acquire(file, root).await?;
        srv.open_document(file, content).await
    }

    /// Hover at (line, character) in `file`.
    pub async fn hover(&self, file: &Path, root: &Path, line: u32, character: u32) -> Result<Value> {
        let srv = self.acquire(file, root).await?;
        srv.hover(file, line, character).await
    }

    /// Go-to-definition at (line, character) in `file`.
    pub async fn definition(
        &self,
        file: &Path,
        root: &Path,
        line: u32,
        character: u32,
    ) -> Result<Value> {
        let srv = self.acquire(file, root).await?;
        srv.definition(file, line, character).await
    }

    /// Find-references at (line, character) in `file`.
    pub async fn references(
        &self,
        file: &Path,
        root: &Path,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Value> {
        let srv = self.acquire(file, root).await?;
        srv.references(file, line, character, include_declaration).await
    }

    /// Shutdown all managed language servers.
    pub async fn shutdown_all(&self) {
        let mut guard = self.servers.lock().await;
        for (_, srv) in guard.iter() {
            let _ = srv.shutdown().await;
        }
        guard.clear();
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_id_rust() {
        assert_eq!(language::language_id(Path::new("main.rs")), Some("rust"));
    }

    #[test]
    fn language_id_python() {
        assert_eq!(language::language_id(Path::new("app.py")), Some("python"));
    }

    #[test]
    fn manager_creates() {
        let _mgr = LspManager::new();
    }
}
