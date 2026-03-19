//! Focus Manager
//!
//! Manages focus state for TUI components following the gitui pattern:
//! - Components have local `focused` flags
//! - FocusManager tracks which component ID has focus
//! - Parent delegates focus changes to components via on_focus/on_blur callbacks
//!
//! # References
//!
//! - [gitui Focus Pattern](https://github.com/gitui-org/gitui/blob/master/src/components/mod.rs)

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;

/// Manages focus state across multiple components
///
/// The FocusManager tracks which component currently has input focus
/// and provides methods for focus navigation and delegation.
///
/// # Examples
///
/// ```rust,ignore
/// let mut focus_manager = FocusManager::new();
///
/// // Register focusable components
/// focus_manager.register(textarea.id());
/// focus_manager.register(list.id());
///
/// // Set initial focus
/// focus_manager.set_focus(textarea.id());
///
/// // Navigate between components
/// focus_manager.next_focus();
/// focus_manager.prev_focus();
/// ```
pub struct FocusManager {
    /// Currently focused component ID (if any)
    focused_id: Option<ComponentId>,
    /// List of focusable component IDs in navigation order
    focusable_components: Vec<ComponentId>,
    /// Map of component focus states (true = focused, false = blur)
    focus_states: std::collections::HashMap<ComponentId, bool>,
}

impl FocusManager {
    /// Create a new focus manager with no focused component
    ///
    /// # Returns
    ///
    /// A new `FocusManager` instance with empty focus state.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::FocusManager;
    ///
    /// let manager = FocusManager::new();
    /// assert_eq!(manager.get_focused(), None);
    /// ```
    pub fn new() -> Self {
        Self {
            focused_id: None,
            focusable_components: Vec::new(),
            focus_states: std::collections::HashMap::new(),
        }
    }

    /// Register a focusable component
    ///
    /// Adds a component to the focus rotation. The component will be
    /// included in next_focus/prev_focus navigation.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the component to register
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::{FocusManager, ComponentId};
    ///
    /// let mut manager = FocusManager::new();
    /// let id = ComponentId::new();
    /// manager.register(id);
    /// ```
    pub fn register(&mut self, id: ComponentId) {
        if !self.focusable_components.contains(&id) {
            self.focusable_components.push(id);
            self.focus_states.insert(id, false);
        }
    }

    /// Unregister a component from focus management
    ///
    /// Removes a component from the focus rotation. If the component
    /// currently has focus, focus is cleared.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the component to unregister
    ///
    /// # Returns
    ///
    /// `true` if the component was registered and is now removed,
    /// `false` if the component was not registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::{FocusManager, ComponentId};
    ///
    /// let mut manager = FocusManager::new();
    /// let id = ComponentId::new();
    /// manager.register(id);
    /// assert!(manager.unregister(id));
    /// assert!(!manager.unregister(id)); // Already removed
    /// ```
    pub fn unregister(&mut self, id: ComponentId) -> bool {
        let was_registered = self.focusable_components.iter().position(|&x| x == id);

        if let Some(pos) = was_registered {
            self.focusable_components.remove(pos);
            self.focus_states.remove(&id);

            // Clear focus if this component had it
            if self.focused_id == Some(id) {
                self.focused_id = None;
            }

            true
        } else {
            false
        }
    }

    /// Set focus to a specific component
    ///
    /// Transfers focus from the currently focused component to the
    /// specified component. Calls on_blur() on the old component and
    /// on_focus() on the new component.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the component to focus
    ///
    /// # Returns
    ///
    /// - `Ok(())` if focus was successfully set
    /// - `Err(ComponentNotRegistered)` if the component is not registered
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut manager = FocusManager::new();
    /// manager.register(component.id());
    /// manager.set_focus(component.id()).unwrap();
    /// assert_eq!(manager.get_focused(), Some(component.id()));
    /// ```
    pub fn set_focus(&mut self, id: ComponentId) -> Result<(), FocusError> {
        if !self.focusable_components.contains(&id) {
            return Err(FocusError::ComponentNotRegistered(id));
        }

        // Blur current focused component
        if let Some(current_id) = self.focused_id {
            if current_id != id {
                self.focus_states.insert(current_id, false);
            }
        }

        // Focus new component
        self.focused_id = Some(id);
        self.focus_states.insert(id, true);

        Ok(())
    }

    /// Get the currently focused component ID
    ///
    /// # Returns
    ///
    /// `Some(ComponentId)` if a component has focus, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::{FocusManager, ComponentId};
    ///
    /// let mut manager = FocusManager::new();
    /// assert_eq!(manager.get_focused(), None);
    ///
    /// let id = ComponentId::new();
    /// manager.register(id);
    /// manager.set_focus(id).unwrap();
    /// assert_eq!(manager.get_focused(), Some(id));
    /// ```
    pub fn get_focused(&self) -> Option<ComponentId> {
        self.focused_id
    }

    /// Remove focus from all components
    ///
    /// Clears the current focus state. The previously focused component
    /// is blurred.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut manager = FocusManager::new();
    /// manager.register(component.id());
    /// manager.set_focus(component.id()).unwrap();
    /// manager.blur();
    /// assert_eq!(manager.get_focused(), None);
    /// ```
    pub fn blur(&mut self) {
        if let Some(current_id) = self.focused_id {
            self.focus_states.insert(current_id, false);
        }
        self.focused_id = None;
    }

    /// Move focus to the next component in the rotation
    ///
    /// Cycles through registered components in registration order.
    /// When the last component is reached, wraps around to the first.
    ///
    /// # Returns
    ///
    /// - `Some(ComponentId)` of the newly focused component
    /// - `None` if no components are registered
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut manager = FocusManager::new();
    /// manager.register(comp1.id());
    /// manager.register(comp2.id());
    ///
    /// let focused = manager.next_focus();
    /// assert_eq!(focused, Some(comp1.id()));
    ///
    /// let focused = manager.next_focus();
    /// assert_eq!(focused, Some(comp2.id()));
    ///
    /// let focused = manager.next_focus(); // Wraps around
    /// assert_eq!(focused, Some(comp1.id()));
    /// ```
    pub fn next_focus(&mut self) -> Option<ComponentId> {
        if self.focusable_components.is_empty() {
            return None;
        }

        let current_index = self.focused_id.and_then(|current_id| {
            self.focusable_components
                .iter()
                .position(|&id| id == current_id)
        });

        let next_index = match current_index {
            Some(index) => (index + 1) % self.focusable_components.len(),
            None => 0,
        };

        let next_id = self.focusable_components[next_index];
        self.set_focus(next_id).ok()?;
        Some(next_id)
    }

    /// Move focus to the previous component in the rotation
    ///
    /// Cycles through registered components in reverse order.
    /// When the first component is reached, wraps around to the last.
    ///
    /// # Returns
    ///
    /// - `Some(ComponentId)` of the newly focused component
    /// - `None` if no components are registered
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut manager = FocusManager::new();
    /// manager.register(comp1.id());
    /// manager.register(comp2.id());
    /// manager.set_focus(comp2.id()).unwrap();
    ///
    /// let focused = manager.prev_focus();
    /// assert_eq!(focused, Some(comp1.id()));
    ///
    /// let focused = manager.prev_focus(); // Wraps around
    /// assert_eq!(focused, Some(comp2.id()));
    /// ```
    pub fn prev_focus(&mut self) -> Option<ComponentId> {
        if self.focusable_components.is_empty() {
            return None;
        }

        let current_index = self.focused_id.and_then(|current_id| {
            self.focusable_components
                .iter()
                .position(|&id| id == current_id)
        });

        let prev_index = match current_index {
            Some(index) => {
                if index == 0 {
                    self.focusable_components.len() - 1
                } else {
                    index - 1
                }
            }
            None => self.focusable_components.len().saturating_sub(1),
        };

        let prev_id = self.focusable_components[prev_index];
        self.set_focus(prev_id).ok()?;
        Some(prev_id)
    }

    /// Check if a specific component has focus
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier of the component to check
    ///
    /// # Returns
    ///
    /// `true` if the component has focus, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::{FocusManager, ComponentId};
    ///
    /// let mut manager = FocusManager::new();
    /// let id = ComponentId::new();
    /// manager.register(id);
    /// assert!(!manager.is_focused(id));
    ///
    /// manager.set_focus(id).unwrap();
    /// assert!(manager.is_focused(id));
    /// ```
    pub fn is_focused(&self, id: ComponentId) -> bool {
        self.focused_id == Some(id)
    }

    /// Get the number of registered focusable components
    ///
    /// # Returns
    ///
    /// The count of components registered for focus navigation.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::{FocusManager, ComponentId};
    ///
    /// let mut manager = FocusManager::new();
    /// assert_eq!(manager.count(), 0);
    ///
    /// manager.register(ComponentId::new());
    /// manager.register(ComponentId::new());
    /// assert_eq!(manager.count(), 2);
    /// ```
    pub fn count(&self) -> usize {
        self.focusable_components.len()
    }

    /// Check if any component has focus
    ///
    /// # Returns
    ///
    /// `true` if a component is currently focused, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::{FocusManager, ComponentId};
    ///
    /// let mut manager = FocusManager::new();
    /// assert!(!manager.has_focus());
    ///
    /// let id = ComponentId::new();
    /// manager.register(id);
    /// manager.set_focus(id).unwrap();
    /// assert!(manager.has_focus());
    /// ```
    pub fn has_focus(&self) -> bool {
        self.focused_id.is_some()
    }

    /// Handle keyboard navigation events
    ///
    /// Processes common navigation keys (Tab, Shift+Tab) and updates
    /// focus accordingly. This is a convenience method for handling
    /// focus navigation in component event handlers.
    ///
    /// # Arguments
    ///
    /// * `event` - The key event to process
    ///
    /// # Returns
    ///
    /// - `EventPropagation::Stop` if the event was handled (Tab/Shift+Tab)
    /// - `EventPropagation::Continue` if the event should be handled by components
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut manager = FocusManager::new();
    /// manager.register(comp1.id());
    /// manager.register(comp2.id());
    ///
    /// let event = KeyEvent::new(KeyCode::Tab);
    /// let propagation = manager.handle_navigation(event);
    /// assert_eq!(propagation, EventPropagation::Stop);
    /// ```
    pub fn handle_navigation(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (event.code, event.modifiers) {
            (KeyCode::Tab, KeyModifiers::SHIFT) | (KeyCode::BackTab, _) => {
                self.prev_focus();
                EventPropagation::Stop
            }
            (KeyCode::Tab, _) => {
                self.next_focus();
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Focus management errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusError {
    /// Attempted to focus a component that is not registered
    ComponentNotRegistered(ComponentId),
}

impl std::fmt::Display for FocusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FocusError::ComponentNotRegistered(id) => {
                write!(
                    f,
                    "Component {:?} is not registered for focus management",
                    id
                )
            }
        }
    }
}

impl std::error::Error for FocusError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_focus_manager_initial_state() {
        let manager = FocusManager::new();
        assert_eq!(manager.get_focused(), None);
        assert!(!manager.has_focus());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_focus_manager_register() {
        let mut manager = FocusManager::new();
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();

        manager.register(id1);
        manager.register(id2);

        assert_eq!(manager.count(), 2);
    }

    #[test]
    fn test_focus_manager_register_duplicate() {
        let mut manager = FocusManager::new();
        let id = ComponentId::new();

        manager.register(id);
        manager.register(id); // Duplicate

        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_focus_manager_unregister() {
        let mut manager = FocusManager::new();
        let id = ComponentId::new();

        manager.register(id);
        assert!(manager.unregister(id));
        assert!(!manager.unregister(id)); // Already removed
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_focus_manager_unregister_clears_focus() {
        let mut manager = FocusManager::new();
        let id = ComponentId::new();

        manager.register(id);
        manager.set_focus(id).unwrap();
        assert_eq!(manager.get_focused(), Some(id));

        manager.unregister(id);
        assert_eq!(manager.get_focused(), None);
    }

    #[test]
    fn test_focus_manager_set_focus() {
        let mut manager = FocusManager::new();
        let id = ComponentId::new();

        manager.register(id);
        manager.set_focus(id).unwrap();

        assert_eq!(manager.get_focused(), Some(id));
        assert!(manager.is_focused(id));
        assert!(manager.has_focus());
    }

    #[test]
    fn test_focus_manager_set_focus_unregistered() {
        let mut manager = FocusManager::new();
        let id = ComponentId::new();

        let result = manager.set_focus(id);
        assert!(matches!(result, Err(FocusError::ComponentNotRegistered(_))));
    }

    #[test]
    fn test_focus_manager_blur() {
        let mut manager = FocusManager::new();
        let id = ComponentId::new();

        manager.register(id);
        manager.set_focus(id).unwrap();
        manager.blur();

        assert_eq!(manager.get_focused(), None);
        assert!(!manager.has_focus());
        assert!(!manager.is_focused(id));
    }

    #[test]
    fn test_focus_manager_next_focus() {
        let mut manager = FocusManager::new();
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();

        manager.register(id1);
        manager.register(id2);

        let focused = manager.next_focus();
        assert_eq!(focused, Some(id1));

        let focused = manager.next_focus();
        assert_eq!(focused, Some(id2));

        // Wraps around
        let focused = manager.next_focus();
        assert_eq!(focused, Some(id1));
    }

    #[test]
    fn test_focus_manager_prev_focus() {
        let mut manager = FocusManager::new();
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();

        manager.register(id1);
        manager.register(id2);
        manager.set_focus(id2).unwrap();

        let focused = manager.prev_focus();
        assert_eq!(focused, Some(id1));

        // Wraps around
        let focused = manager.prev_focus();
        assert_eq!(focused, Some(id2));
    }

    #[test]
    fn test_focus_manager_next_focus_empty() {
        let mut manager = FocusManager::new();
        assert_eq!(manager.next_focus(), None);
    }

    #[test]
    fn test_focus_manager_prev_focus_empty() {
        let mut manager = FocusManager::new();
        assert_eq!(manager.prev_focus(), None);
    }

    #[test]
    fn test_focus_manager_handle_navigation_tab() {
        let mut manager = FocusManager::new();
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();

        manager.register(id1);
        manager.register(id2);

        let event = KeyEvent::new(KeyCode::Tab, crossterm::event::KeyModifiers::empty());
        assert_eq!(manager.handle_navigation(event), EventPropagation::Stop);
        assert_eq!(manager.get_focused(), Some(id1));
    }

    #[test]
    fn test_focus_manager_handle_navigation_shift_tab() {
        let mut manager = FocusManager::new();
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();

        manager.register(id1);
        manager.register(id2);
        manager.set_focus(id2).unwrap();

        let event = KeyEvent::new(KeyCode::BackTab, crossterm::event::KeyModifiers::SHIFT);
        assert_eq!(manager.handle_navigation(event), EventPropagation::Stop);
        assert_eq!(manager.get_focused(), Some(id1));
    }

    #[test]
    fn test_focus_manager_handle_navigation_other_key() {
        let mut manager = FocusManager::new();
        let event = KeyEvent::new(KeyCode::Char('a'), crossterm::event::KeyModifiers::empty());
        assert_eq!(manager.handle_navigation(event), EventPropagation::Continue);
    }

    #[test]
    fn test_focus_error_display() {
        let id = ComponentId::new();
        let error = FocusError::ComponentNotRegistered(id);
        let msg = error.to_string();
        assert!(msg.contains("Component"));
        assert!(msg.contains("not registered"));
    }

    #[test]
    fn test_focus_error_debug() {
        let id = ComponentId::new();
        let error = FocusError::ComponentNotRegistered(id);
        let debug = format!("{:?}", error);
        assert!(debug.contains("ComponentNotRegistered"));
    }
}
