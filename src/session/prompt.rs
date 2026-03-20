//! Session prompt abort controller
//!
//! Manages abort signals for active sessions, allowing cancellation of
//! ongoing AI processing and command execution.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tracing::info;

use super::status::{self, SessionStatusInfo};

/// Abort controller using tokio watch channel for cancellation signaling
#[derive(Clone)]
pub struct AbortController {
    cancelled: watch::Sender<bool>,
    is_cancelled: watch::Receiver<bool>,
}

impl AbortController {
    /// Create a new abort controller
    pub fn new() -> Self {
        let (cancelled, is_cancelled) = watch::channel(false);
        Self { cancelled, is_cancelled }
    }

    /// Signal abort to all listeners
    pub fn abort(&self) {
        let _ = self.cancelled.send(true);
    }

    /// Check if already cancelled
    pub fn is_cancelled(&self) -> bool {
        *self.is_cancelled.borrow()
    }

    /// Get a receiver for cancellation signal
    pub fn cancelled_signal(&self) -> watch::Receiver<bool> {
        self.is_cancelled.clone()
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self::new()
    }
}

/// Global prompt state tracking active sessions
pub struct PromptState {
    controllers: Arc<RwLock<HashMap<String, AbortController>>>,
}

impl PromptState {
    /// Create a new prompt state
    pub fn new() -> Self {
        Self {
            controllers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking a session, returning its abort controller
    pub async fn start(&self, session_id: &str) -> AbortController {
        let controller = AbortController::new();
        self.controllers
            .write()
            .await
            .insert(session_id.to_string(), controller.clone());
        controller
    }

    /// Cancel a session's active operation
    /// Returns true if the session was active, false otherwise
    pub async fn cancel(&self, session_id: &str) -> bool {
        if let Some(controller) = self.controllers.write().await.remove(session_id) {
            controller.abort();
            true
        } else {
            false
        }
    }

    /// Check if a session has an active prompt
    pub async fn is_active(&self, session_id: &str) -> bool {
        self.controllers.read().await.contains_key(session_id)
    }
}

impl Default for PromptState {
    fn default() -> Self {
        Self::new()
    }
}

// Global singleton
static PROMPT_STATE: std::sync::OnceLock<PromptState> = std::sync::OnceLock::new();

/// Get the global prompt state singleton
pub fn global() -> &'static PromptState {
    PROMPT_STATE.get_or_init(PromptState::new)
}

/// Cancel a session's active operation
///
/// Sets the session status to idle after cancellation.
/// This is safe to call even if the session is not active.
pub async fn cancel(session_id: &str) {
    info!("Cancelling session: {}", session_id);
    global().cancel(session_id).await;
    // Set status to idle after abort
    status::set(session_id, SessionStatusInfo::Idle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abort_controller_new() {
        let ctrl = AbortController::new();
        assert!(!ctrl.is_cancelled());
    }

    #[test]
    fn test_abort_controller_abort() {
        let ctrl = AbortController::new();
        assert!(!ctrl.is_cancelled());
        ctrl.abort();
        assert!(ctrl.is_cancelled());
    }

    #[test]
    fn test_abort_controller_signal() {
        let ctrl = AbortController::new();
        let mut signal = ctrl.cancelled_signal();
        assert!(!*signal.borrow_and_update());
        ctrl.abort();
        // Signal should have changed
        assert!(signal.has_changed().unwrap());
        assert!(*signal.borrow_and_update());
    }

    #[tokio::test]
    async fn test_prompt_state_start() {
        let state = PromptState::new();
        let controller = state.start("test-session").await;
        assert!(state.is_active("test-session").await);
        assert!(!controller.is_cancelled());
    }

    #[tokio::test]
    async fn test_prompt_state_cancel_active() {
        let state = PromptState::new();
        state.start("test-session").await;
        assert!(state.is_active("test-session").await);

        let was_active = state.cancel("test-session").await;
        assert!(was_active);
        assert!(!state.is_active("test-session").await);
    }

    #[tokio::test]
    async fn test_prompt_state_cancel_inactive() {
        let state = PromptState::new();
        let was_active = state.cancel("nonexistent").await;
        assert!(!was_active);
    }

    #[tokio::test]
    async fn test_global_singleton() {
        let g1 = global();
        let g2 = global();
        // Same instance
        assert!(std::ptr::eq(g1, g2));
    }

    #[tokio::test]
    async fn test_cancel_function() {
        cancel("test-cancel-session").await;
        // Should not panic, and status should be idle
        let status = status::get("test-cancel-session");
        assert_eq!(status, SessionStatusInfo::Idle);
    }
}