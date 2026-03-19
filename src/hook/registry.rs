//! Hook registry for managing registered hooks
//!
//! Provides registration, unregistration, and query capabilities
//! for hooks organized by event type and priority.

use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{Hook, HookError, HookEventType, HookId};

/// Hook entry containing the hook and its metadata
pub struct HookEntry {
    pub hook: Box<dyn Hook>,
    pub registered_at: DateTime<Utc>,
    pub execution_count: u64,
    pub last_executed_at: Option<DateTime<Utc>>,
}

impl std::fmt::Debug for HookEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookEntry")
            .field("hook_id", &self.hook.id())
            .field("registered_at", &self.registered_at)
            .field("execution_count", &self.execution_count)
            .field("last_executed_at", &self.last_executed_at)
            .finish()
    }
}

/// Execution statistics for a single hook
#[derive(Debug, Default, Clone)]
pub struct HookExecutionStats {
    pub count: u64,
    pub errors: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
}

/// Overall hook execution statistics
#[derive(Debug, Default, Clone)]
pub struct HookStats {
    pub total_executions: u64,
    pub total_errors: u64,
    pub total_time_ms: u64,
    pub by_hook: HashMap<String, HookExecutionStats>,
}

/// Hook registry for managing registered hooks
pub struct HookRegistry {
    hooks_by_event: HashMap<HookEventType, Vec<String>>,
    hooks_by_id: HashMap<String, HookEntry>,
    disabled_hooks: HashSet<String>,
    stats: Arc<RwLock<HookStats>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks_by_event: HashMap::new(),
            hooks_by_id: HashMap::new(),
            disabled_hooks: HashSet::new(),
            stats: Arc::new(RwLock::new(HookStats::default())),
        }
    }

    /// Registers a new hook
    pub fn register(&mut self, hook: Box<dyn Hook>) -> Result<(), HookError> {
        let id_str = hook.id().as_str().to_string();
        let events = hook.events();

        if self.hooks_by_id.contains_key(&id_str) {
            return Err(HookError::AlreadyRegistered(id_str));
        }

        let entry = HookEntry {
            hook,
            registered_at: Utc::now(),
            execution_count: 0,
            last_executed_at: None,
        };

        for event_type in events {
            self.hooks_by_event
                .entry(event_type)
                .or_default()
                .push(id_str.clone());
        }

        self.hooks_by_id.insert(id_str.clone(), entry);

        for hooks in self.hooks_by_event.values_mut() {
            hooks.sort_by(|a, b| {
                let entry_a = self.hooks_by_id.get(a);
                let entry_b = self.hooks_by_id.get(b);
                match (entry_a, entry_b) {
                    (Some(ea), Some(eb)) => ea.hook.priority().cmp(&eb.hook.priority()),
                    _ => std::cmp::Ordering::Equal,
                }
            });
        }

        Ok(())
    }

    /// Unregisters a hook by ID
    pub fn unregister(&mut self, id: &HookId) -> Result<(), HookError> {
        let id_str = id.as_str();
        let entry = self
            .hooks_by_id
            .remove(id_str)
            .ok_or_else(|| HookError::NotFound(id_str.to_string()))?;

        for event_type in entry.hook.events() {
            if let Some(hooks) = self.hooks_by_event.get_mut(&event_type) {
                hooks.retain(|h| h != id_str);
            }
        }

        Ok(())
    }

    /// Disables a hook by name
    pub fn disable(&mut self, name: &str) {
        self.disabled_hooks.insert(name.to_string());
    }

    /// Enables a previously disabled hook
    pub fn enable(&mut self, name: &str) {
        self.disabled_hooks.remove(name);
    }

    /// Checks if a hook is enabled
    pub fn is_enabled(&self, id: &HookId) -> bool {
        !self.disabled_hooks.contains(id.as_str())
    }

    /// Gets all hooks registered for an event type
    pub fn get_hooks_for_event(&self, event_type: HookEventType) -> Vec<&HookEntry> {
        self.hooks_by_event
            .get(&event_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.hooks_by_id.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Gets execution statistics
    pub async fn stats(&self) -> HookStats {
        self.stats.read().await.clone()
    }

    /// Records an execution for statistics
    pub async fn record_execution(&self, hook_id: &str, time_ms: u64, is_error: bool) {
        let mut stats = self.stats.write().await;
        stats.total_executions += 1;
        stats.total_time_ms += time_ms;

        if is_error {
            stats.total_errors += 1;
        }

        let hook_stats = stats.by_hook.entry(hook_id.to_string()).or_default();
        hook_stats.count += 1;
        hook_stats.total_time_ms += time_ms;
        hook_stats.avg_time_ms = hook_stats.total_time_ms as f64 / hook_stats.count as f64;

        if is_error {
            hook_stats.errors += 1;
        }
    }

    /// Gets a hook by ID
    pub fn get_hook(&self, id: &HookId) -> Option<&HookEntry> {
        self.hooks_by_id.get(id.as_str())
    }

    /// Lists all registered hook IDs
    pub fn list_hooks(&self) -> Vec<&str> {
        self.hooks_by_id.keys().map(|s| s.as_str()).collect()
    }

    /// Returns the number of registered hooks
    pub fn len(&self) -> usize {
        self.hooks_by_id.len()
    }

    /// Returns true if no hooks are registered
    pub fn is_empty(&self) -> bool {
        self.hooks_by_id.is_empty()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::{HookContext, HookResult};

    struct TestHook {
        id: HookId,
        events: Vec<HookEventType>,
    }

    impl TestHook {
        fn new(name: &str, events: Vec<HookEventType>) -> Self {
            Self {
                id: HookId::new(name),
                events,
            }
        }
    }

    #[async_trait::async_trait]
    impl Hook for TestHook {
        fn id(&self) -> &HookId {
            &self.id
        }

        fn events(&self) -> Vec<HookEventType> {
            self.events.clone()
        }

        async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
            HookResult::Continue
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = HookRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_hook() {
        let mut registry = HookRegistry::new();
        let hook = TestHook::new("test-hook", vec![HookEventType::ChatMessage]);

        assert!(registry.register(Box::new(hook)).is_ok());
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_duplicate_registration() {
        let mut registry = HookRegistry::new();

        let hook1 = TestHook::new("same-id", vec![HookEventType::ChatMessage]);
        let hook2 = TestHook::new("same-id", vec![HookEventType::ChatMessage]);

        assert!(registry.register(Box::new(hook1)).is_ok());

        let result = registry.register(Box::new(hook2));
        assert!(matches!(result, Err(HookError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_unregister_hook() {
        let mut registry = HookRegistry::new();
        let hook = TestHook::new("test-hook", vec![HookEventType::ChatMessage]);

        registry.register(Box::new(hook)).unwrap();
        assert_eq!(registry.len(), 1);

        let result = registry.unregister(&HookId::new("test-hook"));
        assert!(result.is_ok());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let mut registry = HookRegistry::new();
        let result = registry.unregister(&HookId::new("nonexistent"));

        assert!(matches!(result, Err(HookError::NotFound(_))));
    }

    #[test]
    fn test_disable_enable_hook() {
        let mut registry = HookRegistry::new();

        registry.disable("test-hook");
        assert!(!registry.is_enabled(&HookId::new("test-hook")));

        registry.enable("test-hook");
        assert!(registry.is_enabled(&HookId::new("test-hook")));
    }

    #[test]
    fn test_get_hooks_for_event() {
        let mut registry = HookRegistry::new();

        let hook1 = TestHook::new("hook1", vec![HookEventType::ChatMessage]);
        let hook2 = TestHook::new(
            "hook2",
            vec![HookEventType::ChatMessage, HookEventType::ToolExecuteBefore],
        );

        registry.register(Box::new(hook1)).unwrap();
        registry.register(Box::new(hook2)).unwrap();

        let chat_hooks = registry.get_hooks_for_event(HookEventType::ChatMessage);
        assert_eq!(chat_hooks.len(), 2);

        let tool_hooks = registry.get_hooks_for_event(HookEventType::ToolExecuteBefore);
        assert_eq!(tool_hooks.len(), 1);

        let session_hooks = registry.get_hooks_for_event(HookEventType::SessionEvent);
        assert!(session_hooks.is_empty());
    }

    #[test]
    fn test_list_hooks() {
        let mut registry = HookRegistry::new();

        let hook1 = TestHook::new("hook1", vec![HookEventType::ChatMessage]);
        let hook2 = TestHook::new("hook2", vec![HookEventType::ChatMessage]);

        registry.register(Box::new(hook1)).unwrap();
        registry.register(Box::new(hook2)).unwrap();

        let hooks = registry.list_hooks();
        assert_eq!(hooks.len(), 2);
    }

    #[tokio::test]
    async fn test_record_execution() {
        let registry = HookRegistry::new();

        registry.record_execution("test-hook", 100, false).await;
        registry.record_execution("test-hook", 200, true).await;

        let stats = registry.stats().await;
        assert_eq!(stats.total_executions, 2);
        assert_eq!(stats.total_errors, 1);
        assert_eq!(stats.total_time_ms, 300);

        let hook_stats = stats.by_hook.get("test-hook").unwrap();
        assert_eq!(hook_stats.count, 2);
        assert_eq!(hook_stats.errors, 1);
        assert_eq!(hook_stats.avg_time_ms, 150.0);
    }
}
