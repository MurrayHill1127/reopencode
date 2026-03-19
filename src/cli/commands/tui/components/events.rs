//! Component Event Types
//!
//! Event types for the TUI component system. Wraps crossterm events
//! and adds component-specific events for pattern matching in event handlers.

use crossterm::event::{KeyEvent, MouseEvent};

/// Events that components can receive and handle.
///
/// Wraps crossterm events and adds component-specific events
/// for a unified event handling interface.
#[derive(Debug, Clone)]
pub enum ComponentEvent {
    /// Keyboard input event
    Key(KeyEvent),
    /// Mouse input event
    Mouse(MouseEvent),
    /// Component gained focus
    FocusGained,
    /// Component lost focus
    FocusLost,
    /// Terminal resize event
    Resize(u16, u16),
    /// Tick event for periodic updates
    Tick,
}

/// Actions that components can emit to communicate with the parent.
///
/// Used for upward communication from child components to parent
/// containers or the main application loop.
#[derive(Debug, Clone)]
pub enum ComponentAction {
    /// Quit the application
    Quit,
    /// Request re-render
    Render,
    /// Focus next component in focus ring
    FocusNext,
    /// Focus previous component in focus ring
    FocusPrev,
    /// Submit current input with content
    Submit(String),
    /// Show a toast notification
    ShowToast {
        /// Toast title
        title: String,
        /// Toast message content
        message: String,
        /// Visual style variant
        variant: ToastVariant,
    },
    /// Close current dialog or modal
    CloseDialog,
    /// Custom action with string payload
    Custom(String),
}

/// Visual style variants for toast notifications.
///
/// Determines the appearance and semantic meaning of toast messages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ToastVariant {
    /// Informational message (default)
    #[default]
    Info,
    /// Success message (green/green-tinted)
    Success,
    /// Warning message (yellow/amber-tinted)
    Warning,
    /// Error message (red-tinted)
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_variant_default() {
        assert_eq!(ToastVariant::default(), ToastVariant::Info);
    }

    #[test]
    fn test_toast_variant_equality() {
        assert_eq!(ToastVariant::Info, ToastVariant::Info);
        assert_ne!(ToastVariant::Info, ToastVariant::Error);
        assert_ne!(ToastVariant::Success, ToastVariant::Warning);
    }

    #[test]
    fn test_toast_variant_copy() {
        let variant = ToastVariant::Success;
        let copied = variant;
        assert_eq!(variant, copied);
    }

    #[test]
    fn test_component_action_quit() {
        let action = ComponentAction::Quit;
        match action {
            ComponentAction::Quit => {}
            _ => panic!("Expected Quit action"),
        }
    }

    #[test]
    fn test_component_action_render() {
        let action = ComponentAction::Render;
        match action {
            ComponentAction::Render => {}
            _ => panic!("Expected Render action"),
        }
    }

    #[test]
    fn test_component_action_focus_navigation() {
        let next = ComponentAction::FocusNext;
        let prev = ComponentAction::FocusPrev;

        match next {
            ComponentAction::FocusNext => {}
            _ => panic!("Expected FocusNext action"),
        }

        match prev {
            ComponentAction::FocusPrev => {}
            _ => panic!("Expected FocusPrev action"),
        }
    }

    #[test]
    fn test_component_action_submit() {
        let action = ComponentAction::Submit("hello world".to_string());
        match action {
            ComponentAction::Submit(content) => {
                assert_eq!(content, "hello world");
            }
            _ => panic!("Expected Submit action"),
        }
    }

    #[test]
    fn test_component_action_show_toast() {
        let action = ComponentAction::ShowToast {
            title: "Test".to_string(),
            message: "This is a test".to_string(),
            variant: ToastVariant::Success,
        };

        match action {
            ComponentAction::ShowToast {
                title,
                message,
                variant,
            } => {
                assert_eq!(title, "Test");
                assert_eq!(message, "This is a test");
                assert_eq!(variant, ToastVariant::Success);
            }
            _ => panic!("Expected ShowToast action"),
        }
    }

    #[test]
    fn test_component_action_close_dialog() {
        let action = ComponentAction::CloseDialog;
        match action {
            ComponentAction::CloseDialog => {}
            _ => panic!("Expected CloseDialog action"),
        }
    }

    #[test]
    fn test_component_action_custom() {
        let action = ComponentAction::Custom("my-custom-action".to_string());
        match action {
            ComponentAction::Custom(payload) => {
                assert_eq!(payload, "my-custom-action");
            }
            _ => panic!("Expected Custom action"),
        }
    }

    #[test]
    fn test_component_event_resize() {
        let event = ComponentEvent::Resize(80, 24);
        match event {
            ComponentEvent::Resize(cols, rows) => {
                assert_eq!(cols, 80);
                assert_eq!(rows, 24);
            }
            _ => panic!("Expected Resize event"),
        }
    }

    #[test]
    fn test_component_event_tick() {
        let event = ComponentEvent::Tick;
        match event {
            ComponentEvent::Tick => {}
            _ => panic!("Expected Tick event"),
        }
    }

    #[test]
    fn test_component_event_focus() {
        let gained = ComponentEvent::FocusGained;
        let lost = ComponentEvent::FocusLost;

        match gained {
            ComponentEvent::FocusGained => {}
            _ => panic!("Expected FocusGained event"),
        }

        match lost {
            ComponentEvent::FocusLost => {}
            _ => panic!("Expected FocusLost event"),
        }
    }

    #[test]
    fn test_component_event_debug_format() {
        let event = ComponentEvent::Tick;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Tick"));
    }

    #[test]
    fn test_component_action_debug_format() {
        let action = ComponentAction::Quit;
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("Quit"));
    }

    #[test]
    fn test_toast_variant_debug_format() {
        let variant = ToastVariant::Error;
        let debug_str = format!("{:?}", variant);
        assert!(debug_str.contains("Error"));
    }
}
