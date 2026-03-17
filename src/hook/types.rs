//! Hook data structures
//!
//! Defines all core types for the hook system including identifiers,
//! priorities, events, results, and configuration types.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ==================== Hook Identity ====================

/// Unique identifier for a hook
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HookId(pub String);

impl HookId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for HookId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Built-in hook names enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinHookName {
    // Session level hooks (23)
    ContextWindowMonitor,
    PreemptiveCompaction,
    SessionRecovery,
    SessionNotification,
    ThinkMode,
    ModelFallback,
    AnthropicContextWindowLimitRecovery,
    AutoUpdateChecker,
    AgentUsageReminder,
    NonInteractiveEnv,
    InteractiveBashSession,
    RalphLoop,
    EditErrorRecovery,
    DelegateTaskRetry,
    StartWork,
    PrometheusMdOnly,
    SisyphusJuniorNotepad,
    NoSisyphusGpt,
    NoHephaestusNonGpt,
    QuestionLabelTruncator,
    TaskResumeInfo,
    AnthropicEffort,
    RuntimeFallback,

    // Tool Guard hooks (10)
    CommentChecker,
    ToolOutputTruncator,
    DirectoryAgentsInjector,
    DirectoryReadmeInjector,
    EmptyTaskResponseDetector,
    RulesInjector,
    TasksTodowriteDisabler,
    WriteExistingFileGuard,
    HashlineReadEnhancer,
    JsonErrorRecovery,

    // Transform hooks (4)
    ClaudeCodeHooks,
    KeywordDetector,
    ContextInjectorMessagesTransform,
    ThinkingBlockValidator,

    // Continuation hooks (7)
    StopContinuationGuard,
    CompactionContextInjector,
    CompactionTodoPreserver,
    TodoContinuationEnforcer,
    UnstableAgentBabysitter,
    BackgroundNotification,
    Atlas,

    // Skill hooks (2)
    CategorySkillReminder,
    AutoSlashCommand,
}

// ==================== Hook Priority ====================

/// Hook priority levels for execution ordering
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HookPriority {
    /// Pre-hook - executes before the target operation
    OnBefore = 1,

    /// Instead hook - replaces the target operation
    OnInstead = 2,

    /// Post-hook - executes after the target operation
    #[default]
    OnAfter = 3,

    /// Success hook - executes on successful completion
    OnSuccess = 4,

    /// Error hook - executes on failure
    OnError = 5,
}

// ==================== Hook Event ====================

/// Session event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventType {
    Created,
    Deleted,
    Idle,
    Compacted,
    Error,
}

/// Resolved model information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedModel {
    pub provider_id: String,
    pub model_id: String,
    pub variant: Option<String>,
}

/// Image source for message content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

/// Message content types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MessageContent {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

/// Chat message structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Vec<MessageContent>,
}

/// Hook event types for triggering callbacks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "type", content = "properties")]
pub enum HookEvent {
    // ===== Chat Events =====
    ChatMessage {
        session_id: String,
        agent: Option<String>,
        model: Option<ResolvedModel>,
    },

    ChatParams {
        session_id: String,
        provider_id: String,
        model_id: String,
    },

    ChatHeaders {
        session_id: String,
        headers: HashMap<String, String>,
    },

    // ===== Tool Events =====
    ToolRegister {
        tools: Vec<String>,
    },

    ToolExecuteBefore {
        session_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
    },

    ToolExecuteAfter {
        session_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
        tool_output: serde_json::Value,
    },

    // ===== Session Events =====
    SessionEvent {
        event_type: SessionEventType,
        session_id: String,
        properties: Option<serde_json::Value>,
    },

    // ===== Transform Events =====
    MessagesTransform {
        session_id: String,
        messages: Vec<ChatMessage>,
    },
}

// ==================== Hook Result ====================

/// Toast notification variant
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToastVariant {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

/// Toast notification request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastRequest {
    pub title: String,
    pub message: String,
    pub variant: ToastVariant,
    #[serde(default)]
    pub duration_ms: u64,
}

/// Hook output for modifying execution flow
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookOutput {
    pub message: Option<serde_json::Value>,
    pub params: Option<serde_json::Value>,
    pub headers: Option<HashMap<String, String>>,
    pub toast: Option<ToastRequest>,
    pub injected_content: Option<String>,
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Hook execution result for controlling flow
#[derive(Debug)]
pub enum HookResult {
    /// Continue executing next hook
    Continue,

    /// Continue with modified output
    Modified(HookOutput),

    /// Skip remaining hooks (OnInstead only)
    Skip(HookOutput),

    /// Stop execution chain with error
    Stop(crate::hook::error::HookError),
}

// ==================== Token Usage ====================

/// Token usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub input: u64,
    #[serde(default)]
    pub output: u64,
    #[serde(default)]
    pub cache_read: u64,
    #[serde(default)]
    pub cache_write: u64,
}

// ==================== Hook Configuration ====================

/// Hook override configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOverride {
    #[serde(default)]
    pub disabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<HookPriority>,

    #[serde(flatten)]
    pub config: serde_json::Value,
}

impl Default for HookOverride {
    fn default() -> Self {
        Self {
            disabled: false,
            priority: None,
            config: serde_json::Value::Null,
        }
    }
}

/// Hook configuration for the module
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {
    #[serde(default)]
    pub disabled_hooks: Vec<String>,

    #[serde(default)]
    pub overrides: HashMap<String, HookOverride>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_id() {
        let id = HookId::new("test-hook");
        assert_eq!(id.as_str(), "test-hook");
        assert_eq!(id.to_string(), "test-hook");
    }

    #[test]
    fn test_hook_id_equality() {
        let id1 = HookId::new("hook-a");
        let id2 = HookId::new("hook-a");
        let id3 = HookId::new("hook-b");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_hook_priority_ordering() {
        assert!(HookPriority::OnBefore < HookPriority::OnInstead);
        assert!(HookPriority::OnInstead < HookPriority::OnAfter);
        assert!(HookPriority::OnAfter < HookPriority::OnSuccess);
        assert!(HookPriority::OnSuccess < HookPriority::OnError);
    }

    #[test]
    fn test_hook_priority_default() {
        assert_eq!(HookPriority::default(), HookPriority::OnAfter);
    }

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::ChatMessage {
            session_id: "test-session".to_string(),
            agent: Some("build".to_string()),
            model: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("chat-message"));
        assert!(json.contains("test-session"));
    }

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.total, 0);
        assert_eq!(usage.input, 0);
        assert_eq!(usage.output, 0);
    }

    #[test]
    fn test_hook_output_default() {
        let output = HookOutput::default();
        assert!(output.message.is_none());
        assert!(output.params.is_none());
        assert!(output.headers.is_none());
        assert!(output.toast.is_none());
        assert!(output.injected_content.is_none());
        assert!(output.custom.is_empty());
    }

    #[test]
    fn test_hook_config_default() {
        let config = HookConfig::default();
        assert!(config.disabled_hooks.is_empty());
        assert!(config.overrides.is_empty());
    }

    #[test]
    fn test_builtin_hook_name_serialization() {
        let name = BuiltinHookName::ModelFallback;
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"model-fallback\"");

        let name = BuiltinHookName::ContextWindowMonitor;
        let json = serde_json::to_string(&name).unwrap();
        assert_eq!(json, "\"context-window-monitor\"");
    }

    #[test]
    fn test_session_event_type_serialization() {
        let event = SessionEventType::Created;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"created\"");

        let event = SessionEventType::Compacted;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"compacted\"");
    }

    #[test]
    fn test_toast_variant_default() {
        assert_eq!(ToastVariant::default(), ToastVariant::Info);
    }
}