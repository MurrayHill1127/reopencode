//! Permission system — rule evaluation, async ask/reply, and approved-rule persistence.
//!
//! Mirrors opencode's permission/index.ts + permission/evaluate.ts behaviour:
//!   evaluate()  — last-match wins across ordered rulesets, default "ask"
//!   ask()       — deny → error, all allow → proceed, else suspend on oneshot
//!   reply()     — resolves pending requests; "always" cascades to other pending

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

// ─── types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub permission: String,
    pub pattern: String,
    pub action: Action,
}

pub type Ruleset = Vec<Rule>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub session_id: String,
    pub permission: String,
    pub patterns: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Patterns that should be added to the approved list on "always" reply.
    #[serde(default)]
    pub always: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRef {
    pub message_id: String,
    pub call_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reply {
    Once,
    Always,
    Reject,
}

// ─── errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PermissionError {
    #[error("The user has specified a rule which prevents you from using this specific tool call. Relevant rules: {0}")]
    Denied(String),
    #[error("The user rejected permission to use this specific tool call.")]
    Rejected,
    #[error("The user rejected permission to use this specific tool call with the following feedback: {0}")]
    Corrected(String),
}

// ─── wildcard matching ───────────────────────────────────────────────────────

/// Returns true when `text` matches `pattern` using `*` as a multi-segment wildcard.
fn wildcard_match(text: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return text.eq_ignore_ascii_case(pattern);
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;
    let text_bytes = text.as_bytes();

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        let part_bytes = part.as_bytes();
        if i == 0 {
            // First segment must match at the start.
            if !text_bytes.starts_with(part_bytes) {
                return false;
            }
            pos = part.len();
        } else {
            // Find the next occurrence of `part` at or after `pos`.
            let window = &text_bytes[pos..];
            match window
                .windows(part_bytes.len())
                .position(|w| w.eq_ignore_ascii_case(part_bytes))
            {
                Some(idx) => pos += idx + part_bytes.len(),
                None => return false,
            }
        }
    }

    // If the pattern doesn't end with '*', the tail must match exactly.
    if !pattern.ends_with('*') {
        let last = parts.last().unwrap_or(&"");
        if !text_bytes.ends_with(last.as_bytes()) {
            return false;
        }
    }

    true
}

// ─── rule evaluation ─────────────────────────────────────────────────────────

/// Last-matching rule across all rulesets wins; default is "ask".
pub fn evaluate(permission: &str, pattern: &str, rulesets: &[&Ruleset]) -> Rule {
    let mut result: Option<Rule> = None;
    for ruleset in rulesets {
        for rule in ruleset.iter() {
            if wildcard_match(permission, &rule.permission) && wildcard_match(pattern, &rule.pattern)
            {
                result = Some(rule.clone());
            }
        }
    }
    result.unwrap_or_else(|| Rule {
        permission: permission.to_string(),
        pattern: "*".to_string(),
        action: Action::Ask,
    })
}

/// Compute disabled tools from a ruleset (tools with deny on "*" pattern).
pub fn disabled_tools(tools: &[&str], ruleset: &Ruleset) -> Vec<String> {
    let edit_tools = ["edit", "write", "apply_patch"];
    let mut result = Vec::new();
    for &tool in tools {
        let permission = if edit_tools.contains(&tool) { "edit" } else { tool };
        // Last-match wins.
        let rule = ruleset
            .iter()
            .rev()
            .find(|r| wildcard_match(permission, &r.permission));
        if let Some(r) = rule {
            if r.pattern == "*" && r.action == Action::Deny {
                result.push(tool.to_string());
            }
        }
    }
    result
}

// ─── pending entry ────────────────────────────────────────────────────────────

type ResolveResult = Result<(), PermissionError>;

struct PendingEntry {
    info: Request,
    tx: oneshot::Sender<ResolveResult>,
}

// ─── PermissionStore ─────────────────────────────────────────────────────────

/// Shared permission state for a single project/instance.
#[derive(Clone)]
pub struct PermissionStore {
    inner: Arc<Mutex<StoreInner>>,
}

struct StoreInner {
    pending: HashMap<String, PendingEntry>,
    approved: Ruleset,
}

impl PermissionStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StoreInner {
                pending: HashMap::new(),
                approved: Vec::new(),
            })),
        }
    }

    /// Load pre-approved rules (e.g. from DB on startup).
    pub async fn load_approved(&self, rules: Ruleset) {
        self.inner.lock().await.approved = rules;
    }

    /// List all pending requests.
    pub async fn list_pending(&self) -> Vec<Request> {
        self.inner
            .lock()
            .await
            .pending
            .values()
            .map(|e| e.info.clone())
            .collect()
    }

    /// Evaluate rules and either allow immediately, deny with error, or suspend
    /// until a `reply()` call resolves the request.
    pub async fn ask(
        &self,
        session_id: &str,
        permission: &str,
        patterns: &[String],
        always: &[String],
        ruleset: &Ruleset,
        metadata: serde_json::Value,
        tool: Option<ToolRef>,
    ) -> Result<(), PermissionError> {
        let mut needs_ask = false;

        {
            let guard = self.inner.lock().await;
            let rulesets = [ruleset, &guard.approved];

            for pattern in patterns {
                let rule = evaluate(permission, pattern, &rulesets);
                match rule.action {
                    Action::Deny => {
                        let relevant: Vec<_> = ruleset
                            .iter()
                            .filter(|r| wildcard_match(permission, &r.permission))
                            .collect();
                        return Err(PermissionError::Denied(
                            serde_json::to_string(&relevant).unwrap_or_default(),
                        ));
                    }
                    Action::Allow => {}
                    Action::Ask => needs_ask = true,
                }
            }
        }

        if !needs_ask {
            return Ok(());
        }

        let id = Uuid::new_v4().to_string();
        let info = Request {
            id: id.clone(),
            session_id: session_id.to_string(),
            permission: permission.to_string(),
            patterns: patterns.to_vec(),
            metadata,
            always: always.to_vec(),
            tool,
        };

        let (tx, rx) = oneshot::channel();
        {
            let mut guard = self.inner.lock().await;
            guard.pending.insert(id.clone(), PendingEntry { info, tx });
        }

        rx.await.unwrap_or(Err(PermissionError::Rejected))
    }

    /// Resolve a pending request.
    pub async fn reply(
        &self,
        request_id: &str,
        reply: Reply,
        message: Option<String>,
    ) -> Option<()> {
        let entry = {
            let mut guard = self.inner.lock().await;
            guard.pending.remove(request_id)?
        };

        match reply {
            Reply::Reject => {
                let err = match message {
                    Some(msg) => PermissionError::Corrected(msg),
                    None => PermissionError::Rejected,
                };
                let session_id = entry.info.session_id.clone();
                let _ = entry.tx.send(Err(err));
                self.reject_session_pending(&session_id).await;
            }
            Reply::Once => {
                let _ = entry.tx.send(Ok(()));
            }
            Reply::Always => {
                let session_id = entry.info.session_id.clone();
                let always = entry.info.always.clone();
                let permission = entry.info.permission.clone();

                {
                    let mut guard = self.inner.lock().await;
                    for pattern in &always {
                        guard.approved.push(Rule {
                            permission: permission.clone(),
                            pattern: pattern.clone(),
                            action: Action::Allow,
                        });
                    }
                }

                let _ = entry.tx.send(Ok(()));
                self.cascade_approved(&session_id).await;
            }
        }
        Some(())
    }

    /// Reject all pending requests from the same session (used on "reject" reply).
    async fn reject_session_pending(&self, session_id: &str) {
        let mut guard = self.inner.lock().await;
        let to_remove: Vec<String> = guard
            .pending
            .iter()
            .filter(|(_, e)| e.info.session_id == session_id)
            .map(|(id, _)| id.clone())
            .collect();
        for id in to_remove {
            if let Some(e) = guard.pending.remove(&id) {
                let _ = e.tx.send(Err(PermissionError::Rejected));
            }
        }
    }

    /// After "always" reply, auto-resolve other pending requests from the same
    /// session whose patterns are now all covered by approved rules.
    async fn cascade_approved(&self, session_id: &str) {
        let mut to_resolve = Vec::new();
        {
            let guard = self.inner.lock().await;
            for (id, entry) in &guard.pending {
                if entry.info.session_id != session_id {
                    continue;
                }
                let all_allowed = entry.info.patterns.iter().all(|pattern| {
                    evaluate(&entry.info.permission, pattern, &[&guard.approved]).action
                        == Action::Allow
                });
                if all_allowed {
                    to_resolve.push(id.clone());
                }
            }
        }
        let mut guard = self.inner.lock().await;
        for id in to_resolve {
            if let Some(e) = guard.pending.remove(&id) {
                let _ = e.tx.send(Ok(()));
            }
        }
    }
}

impl Default for PermissionStore {
    fn default() -> Self {
        Self::new()
    }
}

// ─── config helpers ──────────────────────────────────────────────────────────

/// Build a Ruleset from a flat map: `{ "edit": "allow", "bash": { "~/safe/*": "allow" } }`.
pub fn ruleset_from_config(map: &serde_json::Value) -> Ruleset {
    let mut rules = Vec::new();
    if let Some(obj) = map.as_object() {
        for (key, val) in obj {
            match val {
                serde_json::Value::String(action_str) => {
                    if let Some(action) = parse_action(action_str) {
                        rules.push(Rule {
                            permission: key.clone(),
                            pattern: "*".to_string(),
                            action,
                        });
                    }
                }
                serde_json::Value::Object(patterns) => {
                    for (pattern, action_val) in patterns {
                        if let Some(action_str) = action_val.as_str() {
                            if let Some(action) = parse_action(action_str) {
                                let expanded = expand_home(pattern);
                                rules.push(Rule {
                                    permission: key.clone(),
                                    pattern: expanded,
                                    action,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    rules
}

fn parse_action(s: &str) -> Option<Action> {
    match s {
        "allow" => Some(Action::Allow),
        "deny" => Some(Action::Deny),
        "ask" => Some(Action::Ask),
        _ => None,
    }
}

fn expand_home(pattern: &str) -> String {
    let home = dirs::home_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    if pattern.starts_with("~/") {
        return format!("{}{}", home, &pattern[1..]);
    }
    if pattern == "~" {
        return home;
    }
    if pattern.starts_with("$HOME/") {
        return format!("{}{}", home, &pattern[5..]);
    }
    if pattern == "$HOME" {
        return home;
    }
    pattern.to_string()
}

// ─── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn allow(permission: &str, pattern: &str) -> Rule {
        Rule { permission: permission.to_string(), pattern: pattern.to_string(), action: Action::Allow }
    }
    fn deny(permission: &str, pattern: &str) -> Rule {
        Rule { permission: permission.to_string(), pattern: pattern.to_string(), action: Action::Deny }
    }

    #[test]
    fn wildcard_exact() {
        assert!(wildcard_match("bash", "bash"));
        assert!(!wildcard_match("bash", "edit"));
    }

    #[test]
    fn wildcard_star() {
        assert!(wildcard_match("anything", "*"));
        assert!(wildcard_match("", "*"));
    }

    #[test]
    fn wildcard_prefix() {
        assert!(wildcard_match("/home/user/code/foo.rs", "/home/user/*"));
        assert!(!wildcard_match("/tmp/foo.rs", "/home/user/*"));
    }

    #[test]
    fn evaluate_last_match_wins() {
        let r1 = vec![deny("edit", "*")];
        let r2 = vec![allow("edit", "/safe/*")];
        let rulesets = [&r1, &r2];
        // "/safe/foo" matches both r1 and r2; r2 is later, so allow.
        let rule = evaluate("edit", "/safe/foo", &rulesets);
        assert_eq!(rule.action, Action::Allow);
    }

    #[test]
    fn evaluate_default_ask() {
        let empty: Ruleset = vec![];
        let rule = evaluate("bash", "ls", &[&empty]);
        assert_eq!(rule.action, Action::Ask);
    }

    #[test]
    fn disabled_tools_deny_star() {
        let ruleset = vec![deny("edit", "*")];
        let disabled = disabled_tools(&["edit", "bash", "write"], &ruleset);
        assert!(disabled.contains(&"edit".to_string()));
        assert!(disabled.contains(&"write".to_string())); // write → "edit" permission
        assert!(!disabled.contains(&"bash".to_string()));
    }

    #[tokio::test]
    async fn ask_immediate_allow() {
        let store = PermissionStore::new();
        let ruleset = vec![allow("bash", "*")];
        let result = store
            .ask("s1", "bash", &["ls".to_string()], &[], &ruleset, serde_json::Value::Null, None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ask_immediate_deny() {
        let store = PermissionStore::new();
        let ruleset = vec![deny("bash", "*")];
        let result = store
            .ask("s1", "bash", &["ls".to_string()], &[], &ruleset, serde_json::Value::Null, None)
            .await;
        assert!(matches!(result, Err(PermissionError::Denied(_))));
    }

    #[tokio::test]
    async fn ask_reply_once() {
        let store = PermissionStore::new();
        let ruleset: Ruleset = vec![];

        let store2 = store.clone();
        let handle = tokio::spawn(async move {
            store2
                .ask("s1", "bash", &["ls".to_string()], &[], &ruleset, serde_json::Value::Null, None)
                .await
        });

        // Give the ask a moment to register.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let pending = store.list_pending().await;
        assert_eq!(pending.len(), 1);

        store.reply(&pending[0].id, Reply::Once, None).await;
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn ask_reply_reject() {
        let store = PermissionStore::new();
        let ruleset: Ruleset = vec![];

        let store2 = store.clone();
        let handle = tokio::spawn(async move {
            store2
                .ask("s1", "bash", &["ls".to_string()], &[], &ruleset, serde_json::Value::Null, None)
                .await
        });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let pending = store.list_pending().await;
        store.reply(&pending[0].id, Reply::Reject, None).await;
        let result = handle.await.unwrap();
        assert!(matches!(result, Err(PermissionError::Rejected)));
    }
}
