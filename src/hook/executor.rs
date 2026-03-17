//! Hook executor for running hook chains
//!
//! Provides execution of hooks with error isolation, timeout handling,
//! and result merging capabilities.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use super::{
    Hook, HookContext, HookError, HookEvent, HookEventType, HookOutput,
    HookPriority, HookRegistry, HookResult,
};

/// Hook executor for running hook chains
pub struct HookExecutor {
    registry: Arc<RwLock<HookRegistry>>,
    error_isolation: bool,
    timeout_ms: u64,
}

impl HookExecutor {
    /// Creates a new executor with the given registry
    pub fn new(registry: Arc<RwLock<HookRegistry>>) -> Self {
        Self {
            registry,
            error_isolation: true,
            timeout_ms: 30000,
        }
    }

    /// Enables or disables error isolation
    pub fn with_error_isolation(mut self, enabled: bool) -> Self {
        self.error_isolation = enabled;
        self
    }

    /// Sets the timeout for hook execution
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Executes the hook chain for the given event
    pub async fn execute(
        &self,
        event: HookEvent,
        mut context: HookContext,
    ) -> Result<HookOutput, HookError> {
        let event_type = event_to_type(&event);

        let hook_ids: Vec<String> = {
            let registry = self.registry.read().await;
            registry
                .get_hooks_for_event(event_type)
                .into_iter()
                .filter(|entry| entry.hook.is_enabled() && registry.is_enabled(entry.hook.id()))
                .map(|entry| entry.hook.id().as_str().to_string())
                .collect()
        };

        let mut output = HookOutput::default();
        let mut skip_remaining = false;

        for hook_id in &hook_ids {
            if skip_remaining {
                break;
            }

            let (priority, result, elapsed) = {
                let registry = self.registry.read().await;
                let Some(entry) = registry.get_hook(&super::HookId::new(hook_id)) else {
                    continue;
                };

                let priority = entry.hook.priority();
                let start = Instant::now();
                let result = self.execute_with_timeout(&*entry.hook, &mut context).await;
                let elapsed = start.elapsed().as_millis() as u64;
                (priority, result, elapsed)
            };

            let is_error = result.is_err();
            self.registry
                .read()
                .await
                .record_execution(hook_id, elapsed, is_error)
                .await;

            match result {
                Ok(HookResult::Continue) => {}
                Ok(HookResult::Modified(modified)) => {
                    output = self.merge_output(output, modified);
                }
                Ok(HookResult::Skip(skipped_output)) => {
                    if priority == HookPriority::OnInstead {
                        output = skipped_output;
                        skip_remaining = true;
                    }
                }
                Ok(HookResult::Stop(error)) => {
                    return Err(error);
                }
                Err(error) => {
                    if !self.error_isolation {
                        return Err(error);
                    }
                    tracing::warn!(
                        hook_id = %hook_id,
                        error = %error,
                        "Hook execution failed, but continuing due to error isolation"
                    );
                }
            }
        }

        Ok(output)
    }

    async fn execute_with_timeout(
        &self,
        hook: &dyn Hook,
        ctx: &mut HookContext,
    ) -> Result<HookResult, HookError> {
        let timeout = tokio::time::Duration::from_millis(self.timeout_ms);

        tokio::time::timeout(timeout, hook.execute(ctx))
            .await
            .map_err(|_| HookError::Timeout(hook.id().as_str().to_string()))
    }

    fn merge_output(&self, base: HookOutput, overlay: HookOutput) -> HookOutput {
        HookOutput {
            message: overlay.message.or(base.message),
            params: overlay.params.or(base.params),
            headers: overlay.headers.or(base.headers),
            toast: overlay.toast.or(base.toast),
            injected_content: overlay.injected_content.or(base.injected_content),
            custom: {
                let mut merged = base.custom;
                merged.extend(overlay.custom);
                merged
            },
        }
    }
}

/// Converts a HookEvent to its corresponding HookEventType
pub fn event_to_type(event: &HookEvent) -> HookEventType {
    match event {
        HookEvent::ChatMessage { .. } => HookEventType::ChatMessage,
        HookEvent::ChatParams { .. } => HookEventType::ChatParams,
        HookEvent::ChatHeaders { .. } => HookEventType::ChatHeaders,
        HookEvent::ToolRegister { .. } => HookEventType::ToolRegister,
        HookEvent::ToolExecuteBefore { .. } => HookEventType::ToolExecuteBefore,
        HookEvent::ToolExecuteAfter { .. } => HookEventType::ToolExecuteAfter,
        HookEvent::SessionEvent { .. } => HookEventType::SessionEvent,
        HookEvent::MessagesTransform { .. } => HookEventType::MessagesTransform,
    }
}

/// Convenience function to execute a hook chain
pub async fn execute_chain(
    registry: Arc<RwLock<HookRegistry>>,
    event: HookEvent,
    context: HookContext,
) -> Result<HookOutput, HookError> {
    let executor = HookExecutor::new(registry);
    executor.execute(event, context).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::HookId;
    use std::collections::HashMap;

    struct ContinueHook {
        id: HookId,
    }

    impl ContinueHook {
        fn new() -> Self {
            Self { id: HookId::new("continue-hook") }
        }
    }

    #[async_trait::async_trait]
    impl Hook for ContinueHook {
        fn id(&self) -> &HookId {
            &self.id
        }

        fn events(&self) -> Vec<HookEventType> {
            vec![HookEventType::ChatMessage]
        }

        async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
            HookResult::Continue
        }
    }

    struct ModifyHook {
        id: HookId,
    }

    impl ModifyHook {
        fn new() -> Self {
            Self { id: HookId::new("modify-hook") }
        }
    }

    #[async_trait::async_trait]
    impl Hook for ModifyHook {
        fn id(&self) -> &HookId {
            &self.id
        }

        fn events(&self) -> Vec<HookEventType> {
            vec![HookEventType::ChatMessage]
        }

        async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
            let output = HookOutput {
                message: Some(serde_json::json!({"modified": true})),
                ..Default::default()
            };
            HookResult::Modified(output)
        }
    }

    struct ErrorHook {
        id: HookId,
    }

    impl ErrorHook {
        fn new() -> Self {
            Self { id: HookId::new("error-hook") }
        }
    }

    #[async_trait::async_trait]
    impl Hook for ErrorHook {
        fn id(&self) -> &HookId {
            &self.id
        }

        fn events(&self) -> Vec<HookEventType> {
            vec![HookEventType::ChatMessage]
        }

        async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
            HookResult::Stop(HookError::ExecutionFailed("intentional error".to_string()))
        }
    }

    struct SkipHook {
        id: HookId,
    }

    impl SkipHook {
        fn new() -> Self {
            Self { id: HookId::new("skip-hook") }
        }
    }

    #[async_trait::async_trait]
    impl Hook for SkipHook {
        fn id(&self) -> &HookId {
            &self.id
        }

        fn priority(&self) -> HookPriority {
            HookPriority::OnInstead
        }

        fn events(&self) -> Vec<HookEventType> {
            vec![HookEventType::ChatMessage]
        }

        async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
            let output = HookOutput {
                injected_content: Some("skipped".to_string()),
                ..Default::default()
            };
            HookResult::Skip(output)
        }
    }

    fn create_test_event() -> HookEvent {
        HookEvent::ChatMessage {
            session_id: "test".to_string(),
            agent: None,
            model: None,
        }
    }

    #[tokio::test]
    async fn test_execute_chain_continue() {
        let registry = Arc::new(RwLock::new(HookRegistry::new()));
        registry.write().await.register(Box::new(ContinueHook::new())).unwrap();

        let event = create_test_event();
        let context = HookContext::new(event.clone());

        let result = execute_chain(registry, event, context).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.message.is_none());
    }

    #[tokio::test]
    async fn test_execute_chain_modified() {
        let registry = Arc::new(RwLock::new(HookRegistry::new()));
        registry.write().await.register(Box::new(ModifyHook::new())).unwrap();

        let executor = HookExecutor::new(registry);
        let event = create_test_event();
        let context = HookContext::new(event.clone());

        let result = executor.execute(event, context).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.message.is_some());
        assert_eq!(output.message.unwrap()["modified"], true);
    }

    #[tokio::test]
    async fn test_execute_chain_stop() {
        let registry = Arc::new(RwLock::new(HookRegistry::new()));
        registry.write().await.register(Box::new(ErrorHook::new())).unwrap();

        let executor = HookExecutor::new(registry).with_error_isolation(false);
        let event = create_test_event();
        let context = HookContext::new(event.clone());

        let result = executor.execute(event, context).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(HookError::ExecutionFailed(_))));
    }

    #[tokio::test]
    async fn test_execute_chain_skip() {
        let registry = Arc::new(RwLock::new(HookRegistry::new()));
        registry.write().await.register(Box::new(SkipHook::new())).unwrap();

        let executor = HookExecutor::new(registry);
        let event = create_test_event();
        let context = HookContext::new(event.clone());

        let result = executor.execute(event, context).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.injected_content, Some("skipped".to_string()));
    }

    #[tokio::test]
    async fn test_error_isolation() {
        struct TimeoutHook {
            id: HookId,
        }

        impl TimeoutHook {
            fn new() -> Self {
                Self { id: HookId::new("timeout-hook") }
            }
        }

        #[async_trait::async_trait]
        impl Hook for TimeoutHook {
            fn id(&self) -> &HookId {
                &self.id
            }

            fn events(&self) -> Vec<HookEventType> {
                vec![HookEventType::ChatMessage]
            }

            async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                HookResult::Continue
            }
        }

        let registry = Arc::new(RwLock::new(HookRegistry::new()));
        registry.write().await.register(Box::new(TimeoutHook::new())).unwrap();
        registry.write().await.register(Box::new(ContinueHook::new())).unwrap();

        let executor = HookExecutor::new(registry)
            .with_error_isolation(true)
            .with_timeout(50);
        let event = create_test_event();
        let context = HookContext::new(event.clone());

        let result = executor.execute(event, context).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_event_to_type() {
        assert_eq!(
            event_to_type(&HookEvent::ChatMessage {
                session_id: "test".to_string(),
                agent: None,
                model: None
            }),
            HookEventType::ChatMessage
        );

        assert_eq!(
            event_to_type(&HookEvent::ToolExecuteBefore {
                session_id: "test".to_string(),
                tool_name: "write".to_string(),
                tool_input: serde_json::json!({})
            }),
            HookEventType::ToolExecuteBefore
        );

        assert_eq!(
            event_to_type(&HookEvent::SessionEvent {
                event_type: crate::hook::SessionEventType::Created,
                session_id: "test".to_string(),
                properties: None
            }),
            HookEventType::SessionEvent
        );
    }

    #[tokio::test]
    async fn test_merge_output() {
        let registry = Arc::new(RwLock::new(HookRegistry::new()));
        let executor = HookExecutor::new(registry);

        let base = HookOutput {
            message: Some(serde_json::json!({"base": true})),
            params: None,
            headers: Some(HashMap::new()),
            toast: None,
            injected_content: None,
            custom: HashMap::new(),
        };

        let overlay = HookOutput {
            message: None,
            params: Some(serde_json::json!({"overlay": true})),
            headers: None,
            toast: None,
            injected_content: Some("injected".to_string()),
            custom: HashMap::new(),
        };

        let merged = executor.merge_output(base, overlay);

        assert!(merged.message.is_some());
        assert!(merged.params.is_some());
        assert!(merged.headers.is_some());
        assert!(merged.injected_content.is_some());
    }
}