//! Language server process lifecycle.
//!
//! One `LspServer` per (language-id, root-directory) pair.  The server is
//! started lazily on the first request and keeps running until explicitly shut
//! down or dropped.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{bail, Result};
use serde_json::{json, Value};
use tokio::process::Command;

use crate::lsp::client::LspClient;

pub struct LspServer {
    pub language_id: String,
    pub root: PathBuf,
    pub client: LspClient,
    initialized: bool,
}

impl LspServer {
    /// Start a language server for `lang` rooted at `root`.
    ///
    /// `cmd` is the full argv: `["/usr/bin/rust-analyzer"]` or
    /// `["/usr/bin/typescript-language-server", "--stdio"]`.
    pub async fn start(lang: &str, root: &Path, cmd: &[String]) -> Result<Self> {
        if cmd.is_empty() {
            bail!("empty language server command");
        }

        let mut child = Command::new(&cmd[0])
            .args(&cmd[1..])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        // `tokio::process::ChildStdin` doesn't implement `std::io::Write`, so
        // we convert it to `std::process::ChildStdin` via `into_owned_fd`.
        // The cleanest approach: take the raw fd and rebuild a std handle.
        let tokio_stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        let tokio_stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;

        // Convert tokio stdin to std stdin for synchronous writes.
        let std_stdin = tokio_stdin.into_owned_fd().map(std::process::ChildStdin::from)?;

        let client = LspClient::new(std_stdin, tokio_stdout);

        let mut srv = Self {
            language_id: lang.to_string(),
            root: root.to_path_buf(),
            client,
            initialized: false,
        };

        // Leak the child handle so the process stays alive.
        std::mem::forget(child);

        srv.initialize().await?;
        Ok(srv)
    }

    async fn initialize(&mut self) -> Result<()> {
        let root_uri = format!("file://{}", self.root.display());
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "rootPath": self.root.display().to_string(),
            "capabilities": {
                "textDocument": {
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "definition": {},
                    "references": {},
                    "synchronization": {
                        "didOpen": true,
                        "didClose": true,
                        "didChange": true
                    }
                },
                "workspace": {
                    "workspaceFolders": true
                }
            },
            "workspaceFolders": [{
                "uri": root_uri,
                "name": self.root.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
            }]
        });

        self.client.request("initialize", params).await?;
        self.client.notify("initialized", json!({})).await?;
        self.initialized = true;
        Ok(())
    }

    /// Notify the server that `file` was opened with `content`.
    pub async fn open_document(&self, file: &Path, content: &str) -> Result<()> {
        let lang_id = crate::lsp::language::language_id(file)
            .unwrap_or(&self.language_id);
        self.client.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": file_uri(file),
                    "languageId": lang_id,
                    "version": 1,
                    "text": content
                }
            }),
        ).await
    }

    /// Send a hover request for `(line, character)` in `file`.
    pub async fn hover(&self, file: &Path, line: u32, character: u32) -> Result<Value> {
        self.client.request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": file_uri(file) },
                "position": { "line": line, "character": character }
            }),
        ).await
    }

    /// Send a go-to-definition request.
    pub async fn definition(&self, file: &Path, line: u32, character: u32) -> Result<Value> {
        self.client.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": file_uri(file) },
                "position": { "line": line, "character": character }
            }),
        ).await
    }

    /// Send a find-references request.
    pub async fn references(
        &self,
        file: &Path,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<Value> {
        self.client.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": file_uri(file) },
                "position": { "line": line, "character": character },
                "context": { "includeDeclaration": include_declaration }
            }),
        ).await
    }

    /// Shutdown the server gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        let _ = self.client.request("shutdown", Value::Null).await;
        let _ = self.client.notify("exit", Value::Null).await;
        Ok(())
    }
}

fn file_uri(path: &Path) -> String {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(path)
    };
    format!("file://{}", abs.display())
}
