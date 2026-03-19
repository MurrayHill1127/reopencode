//! Hook module for ROC (reopencode)
//!
//! Provides a lifecycle hook system for event-driven callbacks with:
//! - Event-driven callbacks at key lifecycle points
//! - 5-level priority execution (onBefore, onInstead, onAfter, onSuccess, onError)
//! - Sync and async hook support
//! - Dynamic registration and management
//! - Error isolation for hook failures
//!
//! # Example
//!
//! ```rust
//! use reopencode::hook::{HookRegistry, Hook, HookId, HookContext, HookResult, HookEventType, HookPriority};
//!
//! struct MyHook {
//!     id: HookId,
//! }
//!
//! impl MyHook {
//!     fn new() -> Self {
//!         Self { id: HookId::new("my-hook") }
//!     }
//! }
//!
//! #[async_trait::async_trait]
//! impl Hook for MyHook {
//!     fn id(&self) -> &HookId {
//!         &self.id
//!     }
//!
//!     fn events(&self) -> Vec<HookEventType> {
//!         vec![HookEventType::ChatMessage]
//!     }
//!
//!     async fn execute(&self, _ctx: &mut HookContext) -> HookResult {
//!         HookResult::Continue
//!     }
//! }
//!
//! let mut registry = HookRegistry::new();
//! registry.register(Box::new(MyHook::new())).unwrap();
//! ```

pub mod builtin;
pub mod context;
pub mod error;
pub mod executor;
pub mod registry;
pub mod types;

// Re-export error types
pub use error::HookError;

// Re-export types
pub use types::{
    HookConfig, HookEvent, HookId, HookOutput, HookPriority, HookResult, SessionEventType,
};

// Re-export context types
pub use context::HookContext;

// Re-export registry
pub use registry::HookRegistry;

// Re-export executor

// Re-export builtin factories

/// Hook event types for subscription filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookEventType {
    ChatMessage,
    ChatParams,
    ChatHeaders,
    ToolRegister,
    ToolExecuteBefore,
    ToolExecuteAfter,
    SessionEvent,
    MessagesTransform,
}

/// Core hook trait that all hooks must implement
///
/// Hooks are event-driven callbacks that execute at specific lifecycle points
/// with configurable priority levels.
#[async_trait::async_trait]
pub trait Hook: Send + Sync {
    fn id(&self) -> &HookId;

    fn priority(&self) -> HookPriority {
        HookPriority::default()
    }

    fn events(&self) -> Vec<HookEventType>;

    async fn execute(&self, ctx: &mut HookContext) -> HookResult;

    fn is_enabled(&self) -> bool {
        true
    }

    fn dispose(&self) {}
}

/// Creates a new hook registry
pub fn create_registry() -> HookRegistry {
    HookRegistry::new()
}

/// Registers all built-in hooks into the registry
pub fn register_builtin_hooks(registry: &mut HookRegistry, config: &HookConfig) {
    builtin::register_all(registry, config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_type_variants() {
        assert_eq!(HookEventType::ChatMessage, HookEventType::ChatMessage);
        assert_ne!(HookEventType::ChatMessage, HookEventType::ChatParams);
    }

    #[test]
    fn test_create_registry() {
        let registry = create_registry();
        assert!(
            registry
                .get_hooks_for_event(HookEventType::ChatMessage)
                .is_empty()
        );
    }
}
