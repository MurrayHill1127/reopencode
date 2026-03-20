//! LLM streaming and tool execution layer
//!
//! This module provides the LLM streaming interface with tool resolution
//! and permission filtering, mirroring the TypeScript llm.ts functionality.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use futures::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::info;

use crate::agent::AgentInfo;
use crate::provider::{
    Message, Provider, ToolDefinition as ProviderToolDefinition,
};
use crate::session::message::{ModelMessage, UserMessage};
use crate::session::parts::SessionId;

/// Maximum output tokens for LLM responses
pub const OUTPUT_TOKEN_MAX: u32 = 32_000;

/// Tool definition for LLM calls
#[derive(Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON schema for tool parameters
    pub parameters: serde_json::Value,
    /// Tool execution function (boxed for flexibility)
    #[serde(skip)]
    pub execute: Option<Arc<dyn Fn(serde_json::Value) -> serde_json::Value + Send + Sync>>,
}

impl std::fmt::Debug for ToolDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDef")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("parameters", &self.parameters)
            .field("execute", &self.execute.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl ToolDef {
    /// Create a new tool definition
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
            execute: None,
        }
    }

    /// Add an execute function to the tool
    pub fn with_execute<F>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.execute = Some(Arc::new(f));
        self
    }
}

impl From<ToolDef> for ProviderToolDefinition {
    fn from(tool: ToolDef) -> Self {
        ProviderToolDefinition {
            tool_type: "function".to_string(),
            function: crate::provider::provider_trait::ToolFunction {
                name: tool.name,
                description: tool.description,
                parameters: tool.parameters,
            },
        }
    }
}

/// Tool choice options
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Let the model decide
    Auto,
    /// Force tool use
    Required,
    /// Disable tool use
    None,
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::Auto
    }
}

/// Permission ruleset for tool filtering
#[derive(Debug, Clone, Default)]
pub struct PermissionRuleset {
    /// Allowed tool names (empty means all allowed)
    pub allowed: HashSet<String>,
    /// Denied tool names
    pub denied: HashSet<String>,
}

impl PermissionRuleset {
    /// Create a new empty ruleset
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow a tool
    pub fn allow(mut self, tool: impl Into<String>) -> Self {
        self.allowed.insert(tool.into());
        self
    }

    /// Deny a tool
    pub fn deny(mut self, tool: impl Into<String>) -> Self {
        self.denied.insert(tool.into());
        self
    }

    /// Check if a tool is allowed
    pub fn is_allowed(&self, tool: &str) -> bool {
        // If denied, never allowed
        if self.denied.contains(tool) {
            return false;
        }
        // If allowed list is empty, all non-denied are allowed
        // Otherwise, must be in allowed list
        self.allowed.is_empty() || self.allowed.contains(tool)
    }
}

/// Input for LLM streaming
#[derive(Debug, Clone)]
pub struct StreamInput {
    /// User message that initiated this request
    pub user: UserMessage,
    /// Session ID
    pub session_id: SessionId,
    /// Model identifier (provider/model format)
    pub model: String,
    /// Agent configuration
    pub agent: AgentInfo,
    /// Permission ruleset for tool filtering
    pub permission: Option<PermissionRuleset>,
    /// System prompts to prepend
    pub system: Vec<String>,
    /// Abort signal receiver
    pub abort: Option<watch::Receiver<bool>>,
    /// Messages to send to the model
    pub messages: Vec<ModelMessage>,
    /// Use small model variant
    pub small: bool,
    /// Available tools
    pub tools: HashMap<String, ToolDef>,
    /// Number of retries on failure
    pub retries: Option<u32>,
    /// Tool choice strategy
    pub tool_choice: Option<ToolChoice>,
}

impl Default for StreamInput {
    fn default() -> Self {
        Self {
            user: UserMessage::default(),
            session_id: String::new(),
            model: String::new(),
            agent: AgentInfo::default(),
            permission: None,
            system: Vec::new(),
            abort: None,
            messages: Vec::new(),
            small: false,
            tools: HashMap::new(),
            retries: None,
            tool_choice: None,
        }
    }
}

/// Streaming event from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StreamEvent {
    /// Text delta from the model
    TextDelta {
        delta: String,
    },
    /// Reasoning/thinking delta (for models that support it)
    ReasoningDelta {
        delta: String,
    },
    /// Tool call initiated
    ToolCall {
        id: String,
        name: String,
        arguments: String,
    },
    /// Tool call completed with result
    ToolResult {
        id: String,
        result: String,
    },
    /// Usage statistics update
    Usage {
        input: u32,
        output: u32,
        total: u32,
    },
    /// Stream finished
    Finish {
        reason: String,
    },
    /// Error occurred
    Error {
        message: String,
    },
}

/// Output from LLM streaming
pub type StreamOutput = Box<dyn Stream<Item = StreamEvent> + Send + Unpin>;

/// Streaming result containing the stream and metadata
pub struct StreamResult {
    /// The event stream
    pub stream: StreamOutput,
    /// Model used for this request
    pub model: String,
    /// Provider ID
    pub provider_id: String,
}

/// Resolve tools with permission filtering
///
/// Filters available tools based on:
/// 1. User-specified tool enable/disable in the message
/// 2. Permission ruleset from agent and session
///
/// Returns the filtered set of tools available for use.
pub fn resolve_tools(input: &StreamInput) -> HashMap<String, ToolDef> {
    let mut tools = input.tools.clone();

    // Compute disabled tools from permissions
    let disabled = compute_disabled_tools(
        tools.keys().map(|s| s.as_str()).collect(),
        &input.agent,
        &input.permission,
    );

    // Remove disabled tools and tools explicitly disabled by user
    let to_remove: Vec<String> = tools
        .keys()
        .filter(|name| {
            // Check user override
            if let Some(user_tools) = &input.user.tools {
                if user_tools.get(*name) == Some(&false) {
                    return true;
                }
            }
            // Check permission ruleset
            disabled.contains(*name)
        })
        .cloned()
        .collect();

    for name in to_remove {
        tools.remove(&name);
    }

    tools
}

/// Compute the set of disabled tools based on permissions
fn compute_disabled_tools(
    tool_names: Vec<&str>,
    agent: &AgentInfo,
    permission: &Option<PermissionRuleset>,
) -> HashSet<String> {
    let mut disabled = HashSet::new();

    // Check agent-level permission restrictions using the permission engine
    for tool_name in &tool_names {
        let action = crate::agent::permission::evaluate(&agent.permission, "tool", tool_name);
        if action == crate::agent::permission::Action::Deny {
            disabled.insert(tool_name.to_string());
        }
    }

    // Apply permission ruleset if provided
    if let Some(ruleset) = permission {
        for name in tool_names {
            if !ruleset.is_allowed(name) {
                disabled.insert(name.to_string());
            }
        }
    }

    disabled
}

/// Check if messages contain any tool calls
///
/// Used for LiteLLM proxy compatibility - these proxies require
/// the tools parameter when message history contains tool calls.
pub fn has_tool_calls(messages: &[ModelMessage]) -> bool {
    for msg in messages {
        match msg {
            ModelMessage::Assistant { tool_calls, .. } => {
                if let Some(calls) = tool_calls {
                    if !calls.is_empty() {
                        return true;
                    }
                }
            }
            ModelMessage::Tool { .. } => {
                return true;
            }
            _ => {}
        }
    }
    false
}

/// Build provider messages from stream input
///
/// Converts ModelMessage to provider Message format with system prompts prepended.
pub fn build_provider_messages(input: &StreamInput) -> Vec<Message> {
    let mut messages = Vec::new();

    // Add system prompts
    for system in &input.system {
        messages.push(Message::system(system));
    }

    // Add user system prompt if present
    if let Some(user_system) = &input.user.system {
        messages.push(Message::system(user_system));
    }

    // Convert ModelMessages to provider Messages
    for msg in &input.messages {
        match msg {
            ModelMessage::User { content, .. } => {
                // Extract text from content parts
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        crate::session::message::ModelContent::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                messages.push(Message::user(&text));
            }
            ModelMessage::Assistant {
                content, tool_calls, ..
            } => {
                // Extract text from content parts
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        crate::session::message::AssistantContent::Text { text } => {
                            Some(text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                // Convert tool calls
                let provider_tool_calls: Vec<crate::provider::provider_trait::ProviderToolCall> = tool_calls
                    .as_ref()
                    .map(|calls| {
                        calls
                            .iter()
                            .map(|tc| crate::provider::provider_trait::ProviderToolCall {
                                id: tc.tool_call_id.clone(),
                                call_type: "function".to_string(),
                                function: crate::provider::provider_trait::ProviderToolCallFunction {
                                    name: tc.tool_name.clone(),
                                    arguments: tc.args.to_string(),
                                },
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                if provider_tool_calls.is_empty() {
                    messages.push(Message::assistant(&text));
                } else {
                    messages.push(Message::assistant_with_tool_calls(&text, provider_tool_calls));
                }
            }
            ModelMessage::Tool {
                tool_call_id,
                content,
            } => {
                messages.push(Message::tool(content, tool_call_id));
            }
        }
    }

    messages
}

/// Stream from LLM with tool support
///
/// This is the main entry point for LLM streaming. It handles:
/// - Tool resolution with permission filtering
/// - Message preparation
/// - Provider call with streaming
/// - Abort signal handling
///
/// # Arguments
/// * `input` - Stream input configuration
/// * `provider` - Provider to use for the call
///
/// # Returns
/// * `StreamResult` containing the event stream and metadata
pub async fn stream(input: StreamInput, provider: Arc<dyn Provider>) -> StreamResult {
    let model = input.model.clone();
    let session_id = input.session_id.clone();

    info!(
        "stream called: model={}, session={}, small={}",
        model, session_id, input.small
    );

    // Resolve tools with permission filtering
    let tools = resolve_tools(&input);

    // Convert tools to provider format
    let provider_tools: Vec<ProviderToolDefinition> = tools
        .values()
        .map(|t| t.clone().into())
        .collect();

    // Build provider messages
    let messages = build_provider_messages(&input);

    // Get temperature from agent or use default
    let temperature = input.agent.temperature.unwrap_or(0.7);

    // Get max tokens
    let max_tokens = if input.small {
        Some(OUTPUT_TOKEN_MAX / 4) // Lower limit for small mode
    } else {
        Some(OUTPUT_TOKEN_MAX)
    };

    // Create stream using provider's chat_stream
    let provider_stream = provider.chat_stream(
        messages,
        &model,
        temperature,
        max_tokens,
        &provider_tools,
    );

    // Convert provider stream to our stream events
    let stream = Box::new(ProviderStreamAdapter {
        inner: provider_stream,
        cancelled: input.abort,
        buffer: String::new(),
    });

    // Extract provider ID from model string (format: provider/model)
    let provider_id = model.split('/').next().unwrap_or("unknown").to_string();

    StreamResult {
        stream,
        model,
        provider_id,
    }
}

/// Adapter to convert provider stream to our StreamEvent format
struct ProviderStreamAdapter {
    inner: std::pin::Pin<Box<dyn Stream<Item = crate::provider::error::Result<String>> + Send>>,
    cancelled: Option<watch::Receiver<bool>>,
    buffer: String,
}

impl Stream for ProviderStreamAdapter {
    type Item = StreamEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        // Check for cancellation
        if let Some(ref mut rx) = self.cancelled {
            if *rx.borrow() {
                return std::task::Poll::Ready(Some(StreamEvent::Error {
                    message: "Request aborted".to_string(),
                }));
            }
        }

        // Poll inner stream
        match std::pin::Pin::new(&mut self.inner).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(chunk))) => {
                // Try to parse as JSON for structured events
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&chunk) {
                    if let Some(delta) = json.get("delta").and_then(|d| d.as_str()) {
                        return std::task::Poll::Ready(Some(StreamEvent::TextDelta {
                            delta: delta.to_string(),
                        }));
                    }
                    if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                        return std::task::Poll::Ready(Some(StreamEvent::TextDelta {
                            delta: text.to_string(),
                        }));
                    }
                    if let Some(reasoning) = json.get("reasoning").and_then(|r| r.as_str()) {
                        return std::task::Poll::Ready(Some(StreamEvent::ReasoningDelta {
                            delta: reasoning.to_string(),
                        }));
                    }
                    if let Some(tool_call) = json.get("tool_call") {
                        if let (Some(id), Some(name), Some(args)) = (
                            tool_call.get("id").and_then(|v| v.as_str()),
                            tool_call.get("name").and_then(|v| v.as_str()),
                            tool_call.get("arguments").and_then(|v| v.as_str()),
                        ) {
                            return std::task::Poll::Ready(Some(StreamEvent::ToolCall {
                                id: id.to_string(),
                                name: name.to_string(),
                                arguments: args.to_string(),
                            }));
                        }
                    }
                }

                // Fallback: treat as text delta
                if !chunk.is_empty() {
                    return std::task::Poll::Ready(Some(StreamEvent::TextDelta {
                        delta: chunk,
                    }));
                }

                // Try again with next chunk
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            std::task::Poll::Ready(Some(Err(e))) => {
                std::task::Poll::Ready(Some(StreamEvent::Error {
                    message: e.to_string(),
                }))
            }
            std::task::Poll::Ready(None) => {
                std::task::Poll::Ready(Some(StreamEvent::Finish {
                    reason: "stop".to_string(),
                }))
            }
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(name: &str) -> ToolDef {
        ToolDef::new(name, format!("{} tool", name), serde_json::json!({}))
    }

    #[test]
    fn test_tool_def_creation() {
        let tool = ToolDef::new("test", "A test tool", serde_json::json!({"type": "object"}));
        assert_eq!(tool.name, "test");
        assert_eq!(tool.description, "A test tool");
    }

    #[test]
    fn test_tool_def_to_provider() {
        let tool = make_tool("bash");
        let provider_tool: ProviderToolDefinition = tool.into();
        assert_eq!(provider_tool.tool_type, "function");
        assert_eq!(provider_tool.function.name, "bash");
    }

    #[test]
    fn test_permission_ruleset() {
        let ruleset = PermissionRuleset::new()
            .allow("read")
            .allow("write")
            .deny("delete");

        assert!(ruleset.is_allowed("read"));
        assert!(ruleset.is_allowed("write"));
        assert!(!ruleset.is_allowed("delete"));
        assert!(!ruleset.is_allowed("unknown")); // Not in allowed list
    }

    #[test]
    fn test_permission_ruleset_empty_allowed() {
        let ruleset = PermissionRuleset::new().deny("delete");

        // Empty allowed means all non-denied are allowed
        assert!(ruleset.is_allowed("read"));
        assert!(ruleset.is_allowed("write"));
        assert!(!ruleset.is_allowed("delete"));
    }

    #[test]
    fn test_tool_choice_default() {
        assert!(matches!(ToolChoice::default(), ToolChoice::Auto));
    }

    #[test]
    fn test_has_tool_calls_empty() {
        let messages: Vec<ModelMessage> = vec![];
        assert!(!has_tool_calls(&messages));
    }

    #[test]
    fn test_has_tool_calls_with_tool_message() {
        use crate::session::message::ModelContent;

        let messages = vec![
            ModelMessage::User {
                id: "1".to_string(),
                content: vec![ModelContent::Text {
                    text: "Hello".to_string(),
                }],
            },
            ModelMessage::Tool {
                tool_call_id: "call_1".to_string(),
                content: "result".to_string(),
            },
        ];
        assert!(has_tool_calls(&messages));
    }

    #[test]
    fn test_has_tool_calls_with_assistant_tool_calls() {
        use crate::session::message::{AssistantContent, ToolCall};

        let messages = vec![ModelMessage::Assistant {
            id: "1".to_string(),
            content: vec![AssistantContent::Text {
                text: "Hello".to_string(),
            }],
            tool_calls: Some(vec![ToolCall {
                tool_call_id: "call_1".to_string(),
                tool_name: "test".to_string(),
                args: serde_json::json!({}),
            }]),
        }];
        assert!(has_tool_calls(&messages));
    }

    #[test]
    fn test_resolve_tools_no_filtering() {
        let mut tools = HashMap::new();
        tools.insert("read".to_string(), make_tool("read"));
        tools.insert("write".to_string(), make_tool("write"));

        let input = StreamInput {
            tools,
            ..Default::default()
        };

        let resolved = resolve_tools(&input);
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn test_resolve_tools_with_user_override() {
        let mut tools = HashMap::new();
        tools.insert("read".to_string(), make_tool("read"));
        tools.insert("write".to_string(), make_tool("write"));

        let mut user_tools = HashMap::new();
        user_tools.insert("write".to_string(), false);

        let input = StreamInput {
            tools,
            user: UserMessage {
                tools: Some(user_tools),
                ..Default::default()
            },
            ..Default::default()
        };

        let resolved = resolve_tools(&input);
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains_key("read"));
    }

    #[test]
    fn test_resolve_tools_with_permission() {
        let mut tools = HashMap::new();
        tools.insert("read".to_string(), make_tool("read"));
        tools.insert("delete".to_string(), make_tool("delete"));

        let permission = PermissionRuleset::new().deny("delete");

        let input = StreamInput {
            tools,
            permission: Some(permission),
            ..Default::default()
        };

        let resolved = resolve_tools(&input);
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains_key("read"));
    }

    #[test]
    fn test_build_provider_messages_basic() {
        use crate::session::message::ModelContent;

        let input = StreamInput {
            system: vec!["You are helpful.".to_string()],
            messages: vec![ModelMessage::User {
                id: "1".to_string(),
                content: vec![ModelContent::Text {
                    text: "Hello".to_string(),
                }],
            }],
            ..Default::default()
        };

        let messages = build_provider_messages(&input);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, crate::provider::message::MessageRole::System);
        assert_eq!(messages[1].role, crate::provider::message::MessageRole::User);
    }

    #[test]
    fn test_output_token_max() {
        assert_eq!(OUTPUT_TOKEN_MAX, 32_000);
    }

    #[test]
    fn test_stream_input_default() {
        let input = StreamInput::default();
        assert!(input.system.is_empty());
        assert!(input.messages.is_empty());
        assert!(!input.small);
    }
}