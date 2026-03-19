//! Hook execution context
//!
//! Provides context structures passed to hooks during execution,
//! containing session, tool, and event information.

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;

use super::types::{HookEvent, HookId, ResolvedModel, TokenUsage};

/// Session context passed to hooks
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub id: String,
    pub agent: Option<String>,
    pub model: Option<ResolvedModel>,
    pub directory: PathBuf,
    pub created_at: DateTime<Utc>,
    pub message_count: usize,
    pub token_usage: Option<TokenUsage>,
}

impl SessionContext {
    pub fn new(id: impl Into<String>, directory: PathBuf) -> Self {
        Self {
            id: id.into(),
            agent: None,
            model: None,
            directory,
            created_at: Utc::now(),
            message_count: 0,
            token_usage: None,
        }
    }

    pub fn with_agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    pub fn with_model(mut self, model: ResolvedModel) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_message_count(mut self, count: usize) -> Self {
        self.message_count = count;
        self
    }

    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }
}

/// Tool context passed to hooks
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub execution_time_ms: Option<u64>,
}

impl ToolContext {
    pub fn new(name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            input,
            output: None,
            execution_time_ms: None,
        }
    }

    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    pub fn with_execution_time(mut self, time_ms: u64) -> Self {
        self.execution_time_ms = Some(time_ms);
        self
    }
}

/// Hook execution context
#[derive(Debug, Clone)]
pub struct HookContext {
    pub hook_id: HookId,
    pub event: HookEvent,
    pub session: Option<SessionContext>,
    pub tool: Option<ToolContext>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HookContext {
    pub fn new(event: HookEvent) -> Self {
        Self {
            hook_id: HookId::new("unknown"),
            event,
            session: None,
            tool: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_hook_id(mut self, id: HookId) -> Self {
        self.hook_id = id;
        self
    }

    pub fn with_session(mut self, session: SessionContext) -> Self {
        self.session = Some(session);
        self
    }

    pub fn with_tool(mut self, tool: ToolContext) -> Self {
        self.tool = Some(tool);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        self.metadata.get(key)
    }

    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_context_new() {
        let ctx = SessionContext::new("test-session", PathBuf::from("/test"));
        assert_eq!(ctx.id, "test-session");
        assert_eq!(ctx.directory, PathBuf::from("/test"));
        assert!(ctx.agent.is_none());
        assert!(ctx.model.is_none());
        assert_eq!(ctx.message_count, 0);
    }

    #[test]
    fn test_session_context_builder() {
        let ctx = SessionContext::new("test-session", PathBuf::from("/test"))
            .with_agent("build")
            .with_message_count(5);

        assert_eq!(ctx.agent, Some("build".to_string()));
        assert_eq!(ctx.message_count, 5);
    }

    #[test]
    fn test_tool_context_new() {
        let ctx = ToolContext::new("write", serde_json::json!({"path": "/test.txt"}));
        assert_eq!(ctx.name, "write");
        assert!(ctx.output.is_none());
        assert!(ctx.execution_time_ms.is_none());
    }

    #[test]
    fn test_tool_context_builder() {
        let ctx = ToolContext::new("write", serde_json::json!({"path": "/test.txt"}))
            .with_output(serde_json::json!({"success": true}))
            .with_execution_time(150);

        assert!(ctx.output.is_some());
        assert_eq!(ctx.execution_time_ms, Some(150));
    }

    #[test]
    fn test_hook_context_new() {
        let event = HookEvent::ChatMessage {
            session_id: "test".to_string(),
            agent: None,
            model: None,
        };
        let ctx = HookContext::new(event.clone());

        assert_eq!(ctx.hook_id.as_str(), "unknown");
        assert!(ctx.session.is_none());
        assert!(ctx.tool.is_none());
    }

    #[test]
    fn test_hook_context_builder() {
        let event = HookEvent::ToolExecuteBefore {
            session_id: "test".to_string(),
            tool_name: "write".to_string(),
            tool_input: serde_json::json!({}),
        };

        let tool_ctx = ToolContext::new("write", serde_json::json!({}));
        let session_ctx = SessionContext::new("test-session", PathBuf::from("/test"));

        let ctx = HookContext::new(event)
            .with_hook_id(HookId::new("test-hook"))
            .with_session(session_ctx)
            .with_tool(tool_ctx)
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(ctx.hook_id.as_str(), "test-hook");
        assert!(ctx.session.is_some());
        assert!(ctx.tool.is_some());
        assert!(ctx.get_metadata("key").is_some());
    }

    #[test]
    fn test_hook_context_metadata() {
        let event = HookEvent::ChatMessage {
            session_id: "test".to_string(),
            agent: None,
            model: None,
        };
        let mut ctx = HookContext::new(event);

        ctx.set_metadata("count", serde_json::json!(42));
        assert_eq!(ctx.get_metadata("count"), Some(&serde_json::json!(42)));

        ctx.set_metadata("name", serde_json::json!("test"));
        assert_eq!(ctx.get_metadata("name"), Some(&serde_json::json!("test")));
    }
}
