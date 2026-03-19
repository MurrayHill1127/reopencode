//! Built-in hook implementations
//!
//! Provides factory functions for creating standard hooks that ship with ROC.

use serde::{Deserialize, Serialize};

use super::{
    Hook, HookConfig, HookContext, HookEventType, HookId, HookPriority, HookRegistry, HookResult,
    SessionEventType,
};

// ==================== Model Fallback Hook ====================

/// Configuration for the model fallback hook
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelFallbackConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub toast_on_fallback: bool,
}

/// Creates a model fallback hook
pub fn create_model_fallback_hook(config: ModelFallbackConfig) -> impl Hook {
    ModelFallbackHook {
        id: HookId::new("model-fallback"),
        config,
    }
}

struct ModelFallbackHook {
    id: HookId,
    config: ModelFallbackConfig,
}

#[async_trait::async_trait]
impl Hook for ModelFallbackHook {
    fn id(&self) -> &HookId {
        &self.id
    }

    fn priority(&self) -> HookPriority {
        HookPriority::OnAfter
    }

    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::ChatMessage, HookEventType::SessionEvent]
    }

    async fn execute(&self, ctx: &mut HookContext) -> HookResult {
        match &ctx.event {
            super::HookEvent::ChatMessage { .. } => {
                if self.config.enabled {
                    tracing::debug!("Model fallback hook checking for fallback conditions");
                }
                HookResult::Continue
            }
            super::HookEvent::SessionEvent { event_type, .. } => {
                if *event_type == SessionEventType::Error && self.config.enabled {
                    tracing::debug!("Model fallback hook handling session error");
                }
                HookResult::Continue
            }
            _ => HookResult::Continue,
        }
    }
}

// ==================== Session Recovery Hook ====================

/// Creates a session recovery hook
pub fn create_session_recovery_hook() -> impl Hook {
    SessionRecoveryHook {
        id: HookId::new("session-recovery"),
    }
}

struct SessionRecoveryHook {
    id: HookId,
}

#[async_trait::async_trait]
impl Hook for SessionRecoveryHook {
    fn id(&self) -> &HookId {
        &self.id
    }

    fn priority(&self) -> HookPriority {
        HookPriority::OnError
    }

    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::SessionEvent]
    }

    async fn execute(&self, ctx: &mut HookContext) -> HookResult {
        if let super::HookEvent::SessionEvent { event_type, .. } = &ctx.event
            && *event_type == SessionEventType::Error
        {
            tracing::info!("Session recovery hook activated for error recovery");
        }
        HookResult::Continue
    }
}

// ==================== Context Window Monitor Hook ====================

/// Configuration for the context window monitor hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextWindowMonitorConfig {
    #[serde(default = "default_threshold")]
    pub threshold_percent: u8,
    #[serde(default)]
    pub toast_on_warning: bool,
}

fn default_threshold() -> u8 {
    80
}

impl Default for ContextWindowMonitorConfig {
    fn default() -> Self {
        Self {
            threshold_percent: default_threshold(),
            toast_on_warning: false,
        }
    }
}

/// Creates a context window monitor hook
pub fn create_context_window_monitor_hook(config: ContextWindowMonitorConfig) -> impl Hook {
    ContextWindowMonitorHook {
        id: HookId::new("context-window-monitor"),
        config,
    }
}

struct ContextWindowMonitorHook {
    id: HookId,
    config: ContextWindowMonitorConfig,
}

#[async_trait::async_trait]
impl Hook for ContextWindowMonitorHook {
    fn id(&self) -> &HookId {
        &self.id
    }

    fn priority(&self) -> HookPriority {
        HookPriority::OnAfter
    }

    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::SessionEvent, HookEventType::ChatMessage]
    }

    async fn execute(&self, ctx: &mut HookContext) -> HookResult {
        if let Some(session) = &ctx.session
            && let Some(usage) = &session.token_usage
            && usage.total > 0
        {
            let usage_ratio = (usage.input + usage.output) as f64 / usage.total as f64;
            if usage_ratio * 100.0 > self.config.threshold_percent as f64 {
                tracing::warn!(
                    threshold = self.config.threshold_percent,
                    current_percent = (usage_ratio * 100.0) as u8,
                    "Context window usage exceeds threshold"
                );
            }
        }
        HookResult::Continue
    }
}

// ==================== Think Mode Hook ====================

/// Configuration for the think mode hook
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThinkModeConfig {
    #[serde(default)]
    pub enabled: bool,
}

/// Creates a think mode hook
pub fn create_think_mode_hook(config: ThinkModeConfig) -> impl Hook {
    ThinkModeHook {
        id: HookId::new("think-mode"),
        config,
    }
}

struct ThinkModeHook {
    id: HookId,
    config: ThinkModeConfig,
}

#[async_trait::async_trait]
impl Hook for ThinkModeHook {
    fn id(&self) -> &HookId {
        &self.id
    }

    fn priority(&self) -> HookPriority {
        HookPriority::OnBefore
    }

    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::ChatParams]
    }

    async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
        if self.config.enabled {
            tracing::debug!("Think mode hook active");
        }
        HookResult::Continue
    }
}

// ==================== Comment Checker Hook ====================

/// Creates a comment checker hook
pub fn create_comment_checker_hook() -> impl Hook {
    CommentCheckerHook {
        id: HookId::new("comment-checker"),
    }
}

struct CommentCheckerHook {
    id: HookId,
}

#[async_trait::async_trait]
impl Hook for CommentCheckerHook {
    fn id(&self) -> &HookId {
        &self.id
    }

    fn priority(&self) -> HookPriority {
        HookPriority::OnAfter
    }

    fn events(&self) -> Vec<HookEventType> {
        vec![HookEventType::ToolExecuteAfter]
    }

    async fn execute(&self, ctx: &mut HookContext) -> HookResult {
        if let super::HookEvent::ToolExecuteAfter {
            tool_name,
            tool_output,
            ..
        } = &ctx.event
            && (tool_name == "write" || tool_name == "edit")
            && let Some(content) = tool_output.get("content").and_then(|c| c.as_str())
        {
            let ai_patterns = ["我是AI", "我是一个AI", "I am an AI", "作为AI"];
            for pattern in ai_patterns {
                if content.contains(pattern) {
                    tracing::warn!(pattern = pattern, "Detected AI self-reference in output");
                }
            }
        }
        HookResult::Continue
    }
}

// ==================== Registration ====================

/// Registers all built-in hooks into the registry
pub fn register_all(registry: &mut HookRegistry, config: &HookConfig) {
    let hooks: Vec<Box<dyn Hook>> = vec![
        Box::new(ModelFallbackHook {
            id: HookId::new("model-fallback"),
            config: ModelFallbackConfig::default(),
        }),
        Box::new(SessionRecoveryHook {
            id: HookId::new("session-recovery"),
        }),
        Box::new(ContextWindowMonitorHook {
            id: HookId::new("context-window-monitor"),
            config: ContextWindowMonitorConfig::default(),
        }),
        Box::new(ThinkModeHook {
            id: HookId::new("think-mode"),
            config: ThinkModeConfig::default(),
        }),
        Box::new(CommentCheckerHook {
            id: HookId::new("comment-checker"),
        }),
    ];

    for hook in hooks {
        if config.disabled_hooks.contains(&hook.id().0) {
            tracing::debug!(hook_id = hook.id().as_str(), "Skipping disabled hook");
            continue;
        }

        if let Err(e) = registry.register(hook) {
            tracing::warn!("Failed to register hook: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_fallback_config_default() {
        let config = ModelFallbackConfig::default();
        assert!(!config.enabled);
        assert!(!config.toast_on_fallback);
    }

    #[test]
    fn test_context_window_monitor_config_default() {
        let config = ContextWindowMonitorConfig::default();
        assert_eq!(config.threshold_percent, 80);
        assert!(!config.toast_on_warning);
    }

    #[test]
    fn test_create_model_fallback_hook() {
        let hook = create_model_fallback_hook(ModelFallbackConfig::default());
        assert_eq!(hook.id().as_str(), "model-fallback");
        assert_eq!(hook.priority(), HookPriority::OnAfter);
        assert!(!hook.events().is_empty());
    }

    #[test]
    fn test_create_session_recovery_hook() {
        let hook = create_session_recovery_hook();
        assert_eq!(hook.id().as_str(), "session-recovery");
        assert_eq!(hook.priority(), HookPriority::OnError);
    }

    #[test]
    fn test_create_context_window_monitor_hook() {
        let hook = create_context_window_monitor_hook(ContextWindowMonitorConfig::default());
        assert_eq!(hook.id().as_str(), "context-window-monitor");
        assert_eq!(hook.priority(), HookPriority::OnAfter);
    }

    #[test]
    fn test_create_think_mode_hook() {
        let hook = create_think_mode_hook(ThinkModeConfig::default());
        assert_eq!(hook.id().as_str(), "think-mode");
        assert_eq!(hook.priority(), HookPriority::OnBefore);
    }

    #[test]
    fn test_create_comment_checker_hook() {
        let hook = create_comment_checker_hook();
        assert_eq!(hook.id().as_str(), "comment-checker");
        assert_eq!(hook.priority(), HookPriority::OnAfter);
    }

    #[test]
    fn test_register_all() {
        let mut registry = HookRegistry::new();
        let config = HookConfig::default();

        register_all(&mut registry, &config);

        assert!(!registry.is_empty());
        assert!(registry.get_hook(&HookId::new("model-fallback")).is_some());
        assert!(
            registry
                .get_hook(&HookId::new("session-recovery"))
                .is_some()
        );
        assert!(
            registry
                .get_hook(&HookId::new("context-window-monitor"))
                .is_some()
        );
    }

    #[test]
    fn test_register_all_with_disabled() {
        let mut registry = HookRegistry::new();
        let config = HookConfig {
            disabled_hooks: vec!["model-fallback".to_string()],
            overrides: std::collections::HashMap::new(),
        };

        register_all(&mut registry, &config);

        assert!(registry.get_hook(&HookId::new("model-fallback")).is_none());
        assert!(
            registry
                .get_hook(&HookId::new("session-recovery"))
                .is_some()
        );
    }
}
