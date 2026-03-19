//! TUI Event Handling Module
//!
//! This module provides event handling for TUI events via the Bus system.
//! It subscribes to TUI events and updates the TUI application state accordingly.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::bus::Bus;
use crate::bus::{
    TUI_COMMAND_EXECUTE, TUI_PROMPT_APPEND, TUI_SESSION_SELECT, TUI_TOAST_SHOW,
    TuiCommandExecuteProperties, TuiPromptAppendProperties, TuiSessionSelectProperties,
    TuiToastShowProperties,
};

use super::TuiApp;

/// TUI Event Subscriber - manages Bus subscriptions for TUI events
pub struct TuiEventSubscriber {
    bus: Arc<Bus>,
}

impl TuiEventSubscriber {
    /// Create a new TUI event subscriber
    pub fn new(bus: Arc<Bus>) -> Self {
        Self { bus }
    }

    /// Subscribe to all TUI events and wire them to handler functions
    ///
    /// # Arguments
    /// * `app` - Shared reference to TUI application state
    ///
    /// # Returns
    /// * `Result<()>` - Ok if all subscriptions successful, Err otherwise
    pub async fn subscribe_to_events(&self, app: Arc<RwLock<TuiApp>>) -> Result<()> {
        // Subscribe to TUI_TOAST_SHOW
        let app_clone = Arc::clone(&app);
        let _unsubscribe = self
            .bus
            .subscribe(&TUI_TOAST_SHOW, move |event| {
                if let Some(props) = event.properties::<TuiToastShowProperties>() {
                    let app_clone = Arc::clone(&app_clone);
                    tokio::spawn(async move {
                        let mut app = app_clone.write().await;
                        Self::handle_toast_show(&mut app, props);
                    });
                }
            })
            .await;
        drop(_unsubscribe); // Intentionally drop - subscriptions live for app lifetime

        // Subscribe to TUI_PROMPT_APPEND
        let app_clone = Arc::clone(&app);
        let _unsubscribe = self
            .bus
            .subscribe(&TUI_PROMPT_APPEND, move |event| {
                if let Some(props) = event.properties::<TuiPromptAppendProperties>() {
                    let app_clone = Arc::clone(&app_clone);
                    tokio::spawn(async move {
                        let mut app = app_clone.write().await;
                        Self::handle_prompt_append(&mut app, props);
                    });
                }
            })
            .await;
        drop(_unsubscribe);

        // Subscribe to TUI_COMMAND_EXECUTE
        let app_clone = Arc::clone(&app);
        let _unsubscribe = self
            .bus
            .subscribe(&TUI_COMMAND_EXECUTE, move |event| {
                if let Some(props) = event.properties::<TuiCommandExecuteProperties>() {
                    let app_clone = Arc::clone(&app_clone);
                    tokio::spawn(async move {
                        let mut app = app_clone.write().await;
                        Self::handle_command_execute(&mut app, props);
                    });
                }
            })
            .await;
        drop(_unsubscribe);

        // Subscribe to TUI_SESSION_SELECT
        let app_clone = Arc::clone(&app);
        let _unsubscribe = self
            .bus
            .subscribe(&TUI_SESSION_SELECT, move |event| {
                if let Some(props) = event.properties::<TuiSessionSelectProperties>() {
                    let app_clone = Arc::clone(&app_clone);
                    tokio::spawn(async move {
                        let mut app = app_clone.write().await;
                        Self::handle_session_select(&mut app, props);
                    });
                }
            })
            .await;
        drop(_unsubscribe);

        Ok(())
    }

    /// Handle TUI_TOAST_SHOW event - add toast to messages list
    ///
    /// # Arguments
    /// * `app` - Mutable reference to TUI application state
    /// * `props` - Toast show properties (title, message, variant, duration)
    pub fn handle_toast_show(app: &mut TuiApp, props: TuiToastShowProperties) {
        let variant_prefix = match props.variant.as_deref() {
            Some("error") => "[Error] ",
            Some("warning") => "[Warning] ",
            Some("success") => "[Success] ",
            _ => "[Toast] ",
        };

        let toast_msg = format!("{}{}: {}", variant_prefix, props.title, props.message);
        app.messages.push(toast_msg);

        // Store toast with expiry timestamp for auto-dismiss
        if let Some(duration_ms) = props.duration {
            let expires_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + duration_ms;
            app.toasts.push((app.messages.len() - 1, expires_at));
        }
    }

    /// Handle TUI_PROMPT_APPEND event - append text to input field
    ///
    /// # Arguments
    /// * `app` - Mutable reference to TUI application state
    /// * `props` - Prompt append properties (prompt text)
    pub fn handle_prompt_append(app: &mut TuiApp, props: TuiPromptAppendProperties) {
        let current = app.input_component.text();
        app.input_component.set_text(current + &props.prompt);
    }

    /// Handle TUI_COMMAND_EXECUTE event - log command to messages
    ///
    /// # Arguments
    /// * `app` - Mutable reference to TUI application state
    /// * `props` - Command execute properties (command string)
    pub fn handle_command_execute(app: &mut TuiApp, props: TuiCommandExecuteProperties) {
        app.messages.push(format!("[Command] {}", props.command));
        // Future enhancement: execute known commands here
    }

    /// Handle TUI_SESSION_SELECT event - update session ID and status
    ///
    /// # Arguments
    /// * `app` - Mutable reference to TUI application state
    /// * `props` - Session select properties (session ID)
    pub fn handle_session_select(app: &mut TuiApp, props: TuiSessionSelectProperties) {
        app.session_id = Some(props.session_id.clone());
        app.status = format!("Session: {}", props.session_id);
    }

    /// Process expired toasts - remove from messages list
    ///
    /// # Arguments
    /// * `app` - Mutable reference to TUI application state
    pub fn process_expired_toasts(app: &mut TuiApp) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        // Collect indices to remove (in reverse order to maintain indices)
        let mut to_remove: Vec<usize> = app
            .toasts
            .iter()
            .filter(|(_, expires)| *expires <= now)
            .map(|(idx, _)| *idx)
            .collect();

        to_remove.sort_by(|a, b| b.cmp(a)); // Reverse order

        for idx in to_remove {
            if idx < app.messages.len() {
                app.messages.remove(idx);
            }
        }

        // Remove processed toasts
        app.toasts.retain(|(_, expires)| *expires > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_toast_show_default() {
        let mut app = TuiApp::new();
        let props = TuiToastShowProperties {
            title: "Test Title".to_string(),
            message: "Test Message".to_string(),
            variant: None,
            duration: None,
        };

        TuiEventSubscriber::handle_toast_show(&mut app, props);

        assert_eq!(app.messages.len(), 2); // Welcome + toast
        assert!(app.messages[1].contains("[Toast] Test Title: Test Message"));
    }

    #[tokio::test]
    async fn test_handle_toast_show_error_variant() {
        let mut app = TuiApp::new();
        let props = TuiToastShowProperties {
            title: "Error".to_string(),
            message: "Something went wrong".to_string(),
            variant: Some("error".to_string()),
            duration: None,
        };

        TuiEventSubscriber::handle_toast_show(&mut app, props);

        assert!(app.messages[1].contains("[Error] Error: Something went wrong"));
    }

    #[tokio::test]
    async fn test_handle_toast_show_success_variant() {
        let mut app = TuiApp::new();
        let props = TuiToastShowProperties {
            title: "Success".to_string(),
            message: "Operation completed".to_string(),
            variant: Some("success".to_string()),
            duration: None,
        };

        TuiEventSubscriber::handle_toast_show(&mut app, props);

        assert!(app.messages[1].contains("[Success] Success: Operation completed"));
    }

    #[tokio::test]
    async fn test_handle_toast_show_warning_variant() {
        let mut app = TuiApp::new();
        let props = TuiToastShowProperties {
            title: "Warning".to_string(),
            message: "Low memory".to_string(),
            variant: Some("warning".to_string()),
            duration: None,
        };

        TuiEventSubscriber::handle_toast_show(&mut app, props);

        assert!(app.messages[1].contains("[Warning] Warning: Low memory"));
    }

    #[tokio::test]
    async fn test_handle_prompt_append() {
        let mut app = TuiApp::new();
        app.input_component.set_text("existing ");
        let props = TuiPromptAppendProperties {
            prompt: "appended text".to_string(),
        };

        TuiEventSubscriber::handle_prompt_append(&mut app, props);

        assert_eq!(app.input_component.text(), "existing appended text");
    }

    #[tokio::test]
    async fn test_handle_prompt_append_empty() {
        let mut app = TuiApp::new();
        let props = TuiPromptAppendProperties {
            prompt: "new prompt".to_string(),
        };

        TuiEventSubscriber::handle_prompt_append(&mut app, props);

        assert_eq!(app.input_component.text(), "new prompt");
    }

    #[tokio::test]
    async fn test_handle_session_select() {
        let mut app = TuiApp::new();
        app.session_id = None;
        let props = TuiSessionSelectProperties {
            session_id: "test-session-123".to_string(),
        };

        TuiEventSubscriber::handle_session_select(&mut app, props);

        assert_eq!(app.session_id, Some("test-session-123".to_string()));
        assert_eq!(app.status, "Session: test-session-123");
    }

    #[tokio::test]
    async fn test_handle_session_select_overwrite() {
        let mut app = TuiApp::new();
        app.session_id = Some("old-session".to_string());
        let props = TuiSessionSelectProperties {
            session_id: "new-session-456".to_string(),
        };

        TuiEventSubscriber::handle_session_select(&mut app, props);

        assert_eq!(app.session_id, Some("new-session-456".to_string()));
        assert_eq!(app.status, "Session: new-session-456");
    }

    #[tokio::test]
    async fn test_handle_command_execute() {
        let mut app = TuiApp::new();
        let props = TuiCommandExecuteProperties {
            command: "cargo test".to_string(),
        };

        TuiEventSubscriber::handle_command_execute(&mut app, props);

        assert!(
            app.messages
                .iter()
                .any(|m| m.contains("[Command] cargo test"))
        );
    }

    #[tokio::test]
    async fn test_handle_command_execute_multiple() {
        let mut app = TuiApp::new();
        let props1 = TuiCommandExecuteProperties {
            command: "ls -la".to_string(),
        };
        let props2 = TuiCommandExecuteProperties {
            command: "pwd".to_string(),
        };

        TuiEventSubscriber::handle_command_execute(&mut app, props1);
        TuiEventSubscriber::handle_command_execute(&mut app, props2);

        assert!(app.messages.iter().any(|m| m.contains("[Command] ls -la")));
        assert!(app.messages.iter().any(|m| m.contains("[Command] pwd")));
    }

    #[tokio::test]
    async fn test_process_expired_toasts() {
        let mut app = TuiApp::new();

        // Add a toast that expires immediately (duration 0)
        let props = TuiToastShowProperties {
            title: "Quick Toast".to_string(),
            message: "Gone fast".to_string(),
            variant: None,
            duration: Some(0), // Expires immediately
        };

        TuiEventSubscriber::handle_toast_show(&mut app, props);
        let initial_count = app.messages.len();

        // Wait a tiny bit to ensure time passes
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        TuiEventSubscriber::process_expired_toasts(&mut app);

        // Toast should be removed
        assert!(app.messages.len() < initial_count);
    }

    #[tokio::test]
    async fn test_process_active_toasts_not_removed() {
        let mut app = TuiApp::new();

        // Add a toast with long duration (10 seconds)
        let props = TuiToastShowProperties {
            title: "Persistent Toast".to_string(),
            message: "Stays around".to_string(),
            variant: None,
            duration: Some(10000), // 10 seconds
        };

        TuiEventSubscriber::handle_toast_show(&mut app, props);
        let initial_count = app.messages.len();

        TuiEventSubscriber::process_expired_toasts(&mut app);

        // Toast should NOT be removed (still active)
        assert_eq!(app.messages.len(), initial_count);
    }

    #[tokio::test]
    async fn test_bus_subscription_integration() {
        let bus = Arc::new(Bus::new("test"));
        let app = Arc::new(RwLock::new(TuiApp::new()));
        let subscriber = TuiEventSubscriber::new(Arc::clone(&bus));

        // Subscribe to events
        subscriber
            .subscribe_to_events(Arc::clone(&app))
            .await
            .unwrap();

        // Publish a toast event
        bus.publish(
            &TUI_TOAST_SHOW,
            TuiToastShowProperties {
                title: "Integration Test".to_string(),
                message: "This tests the full flow".to_string(),
                variant: Some("success".to_string()),
                duration: None,
            },
        )
        .await;

        // Small delay for async handler
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify event was handled
        let app = app.read().await;
        assert!(
            app.messages
                .iter()
                .any(|m| m.contains("[Success] Integration Test: This tests the full flow"))
        );
    }

    #[tokio::test]
    async fn test_bus_subscription_prompt_append() {
        let bus = Arc::new(Bus::new("test"));
        let app = Arc::new(RwLock::new(TuiApp::new()));
        let subscriber = TuiEventSubscriber::new(Arc::clone(&bus));

        subscriber
            .subscribe_to_events(Arc::clone(&app))
            .await
            .unwrap();

        // Publish prompt append event
        bus.publish(
            &TUI_PROMPT_APPEND,
            TuiPromptAppendProperties {
                prompt: "test input".to_string(),
            },
        )
        .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let app = app.read().await;
        assert_eq!(app.input_component.text(), "test input");
    }

    #[tokio::test]
    async fn test_bus_subscription_session_select() {
        let bus = Arc::new(Bus::new("test"));
        let app = Arc::new(RwLock::new(TuiApp::new()));
        let subscriber = TuiEventSubscriber::new(Arc::clone(&bus));

        subscriber
            .subscribe_to_events(Arc::clone(&app))
            .await
            .unwrap();

        // Publish session select event
        bus.publish(
            &TUI_SESSION_SELECT,
            TuiSessionSelectProperties {
                session_id: "integration-session-789".to_string(),
            },
        )
        .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let app = app.read().await;
        assert_eq!(app.session_id, Some("integration-session-789".to_string()));
        assert_eq!(app.status, "Session: integration-session-789");
    }

    #[tokio::test]
    async fn test_bus_subscription_command_execute() {
        let bus = Arc::new(Bus::new("test"));
        let app = Arc::new(RwLock::new(TuiApp::new()));
        let subscriber = TuiEventSubscriber::new(Arc::clone(&bus));

        subscriber
            .subscribe_to_events(Arc::clone(&app))
            .await
            .unwrap();

        // Publish command execute event
        bus.publish(
            &TUI_COMMAND_EXECUTE,
            TuiCommandExecuteProperties {
                command: "cargo build".to_string(),
            },
        )
        .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let app = app.read().await;
        assert!(
            app.messages
                .iter()
                .any(|m| m.contains("[Command] cargo build"))
        );
    }
}
