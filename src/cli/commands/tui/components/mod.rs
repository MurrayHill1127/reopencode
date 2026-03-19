//! TUI Component System
//!
//! This module provides a modular component architecture for the TUI.

pub mod command_dialog;
pub mod composite;
pub mod dialog;
pub mod error;
pub mod events;
pub mod file_tree;
pub mod focus;
pub mod footer;
pub mod header;
pub mod help_dialog;
pub mod list;
pub mod mcp_status;
pub mod model_selector;
pub mod session_create;
pub mod session_list;
pub mod sidebar;
pub mod status_dialog;
pub mod textarea;
pub mod toast;
pub mod types;

pub use command_dialog::{CommandDialog, CommandEntry};
pub use events::{ComponentAction, ComponentEvent, ToastVariant};
pub use focus::FocusManager;
pub use footer::Footer;
pub use header::{Header, HoverButton};
pub use list::List;
pub use sidebar::{ContextInfo, DiffInfo, LspServerInfo, McpServerInfo, Sidebar, TodoItem};
pub use status_dialog::StatusDialog;
pub use textarea::TextArea;
pub use types::{ComponentId, EventPropagation};

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::time::Duration;

/// Component trait - all TUI components implement this
pub trait Component: Send + Sync {
    fn id(&self) -> ComponentId;
    fn render(&self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, _event: KeyEvent) -> EventPropagation {
        EventPropagation::Continue
    }
    /// Handle component events (keyboard, mouse, focus, tick, etc.)
    fn handle_event(&mut self, event: &ComponentEvent) -> EventPropagation {
        match event {
            ComponentEvent::Key(key) => self.handle_input(*key),
            _ => EventPropagation::Continue,
        }
    }
    fn update(&mut self, _delta: Duration) {}
    fn is_focusable(&self) -> bool {
        true
    }
    /// Check if this component is currently focused
    fn focused(&self) -> bool {
        false
    }
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockComponent {
        id: ComponentId,
    }

    impl MockComponent {
        fn new() -> Self {
            Self {
                id: ComponentId::new(),
            }
        }
    }

    impl Component for MockComponent {
        fn id(&self) -> ComponentId {
            self.id
        }
        fn render(&self, _frame: &mut Frame, _area: Rect) {}
        fn handle_input(&mut self, _event: KeyEvent) -> EventPropagation {
            EventPropagation::Stop
        }
    }

    #[test]
    fn test_component_id_uniqueness() {
        let comp1 = MockComponent::new();
        let comp2 = MockComponent::new();
        assert_ne!(comp1.id(), comp2.id());
    }

    #[test]
    fn test_component_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<MockComponent>();
        assert_sync::<MockComponent>();
    }
}
