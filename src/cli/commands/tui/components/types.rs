//! Component Types
//!
//! This module provides core types for the component system including
//! unique component identifiers and event propagation control.

use std::sync::atomic::{AtomicU64, Ordering};

// ==================== Component Identity ====================

/// Unique identifier for components
///
/// Each component instance gets a unique ID generated from an atomic counter.
/// IDs are guaranteed to be unique across the lifetime of the application.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::ComponentId;
///
/// let id1 = ComponentId::new();
/// let id2 = ComponentId::new();
/// assert_ne!(id1, id2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentId(u64);

/// Atomic counter for generating unique component IDs
static COMPONENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

impl ComponentId {
    /// Create a new unique component ID
    ///
    /// # Returns
    ///
    /// A new `ComponentId` with a value guaranteed to be unique
    /// across all previously created IDs.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::ComponentId;
    ///
    /// let id = ComponentId::new();
    /// ```
    pub fn new() -> Self {
        Self(COMPONENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the underlying numeric value
    ///
    /// # Returns
    ///
    /// The `u64` value of this component ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::ComponentId;
    ///
    /// let id = ComponentId::new();
    /// let value: u64 = id.value();
    /// ```
    pub fn value(&self) -> u64 {
        self.0
    }

    /// Create a component ID from a specific value
    ///
    /// This is primarily useful for testing or when you need
    /// to create a specific ID for mocking purposes.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric value for the ID
    ///
    /// # Returns
    ///
    /// A new `ComponentId` with the specified value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::ComponentId;
    ///
    /// let id = ComponentId::from_value(42);
    /// assert_eq!(id.value(), 42);
    /// ```
    pub fn from_value(value: u64) -> Self {
        Self(value)
    }
}

impl Default for ComponentId {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Event Propagation ====================

/// Controls how events propagate through the component tree.
///
/// When a component handles an event, it returns one of these values
/// to indicate whether the event should continue to parent components
/// or be consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventPropagation {
    /// Continue propagating the event to parent components.
    /// Use this when the component handled the event but wants
    /// parents to also see it (e.g., for logging, analytics).
    #[default]
    Continue,

    /// Stop event propagation - the event has been fully handled.
    /// Parent components will not receive this event.
    /// Use this when the component has consumed the event
    /// (e.g., text input consuming character keys).
    Stop,
}

impl EventPropagation {
    /// Check if this is Stop state
    pub fn is_stop(&self) -> bool {
        matches!(self, Self::Stop)
    }

    /// Check if this is Continue state
    pub fn is_continue(&self) -> bool {
        matches!(self, Self::Continue)
    }
}

// ==================== Focus State ====================

/// Focus state of a component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusState {
    /// Component is focused
    Focused,
    /// Component is not focused
    #[default]
    Unfocused,
}

impl FocusState {
    /// Check if this is Focused state
    pub fn is_focused(&self) -> bool {
        matches!(self, Self::Focused)
    }

    /// Check if this is Unfocused state
    pub fn is_unfocused(&self) -> bool {
        matches!(self, Self::Unfocused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_id_uniqueness() {
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_component_id_value() {
        let id = ComponentId::from_value(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn test_component_id_ordering() {
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_component_id_default() {
        let id1 = ComponentId::default();
        let id2 = ComponentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_component_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        let id1 = ComponentId::new();
        let id2 = ComponentId::new();
        set.insert(id1);
        set.insert(id2);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_event_propagation_stop() {
        let prop = EventPropagation::Stop;
        assert!(prop.is_stop());
        assert!(!prop.is_continue());
    }

    #[test]
    fn test_event_propagation_continue() {
        let prop = EventPropagation::Continue;
        assert!(!prop.is_stop());
        assert!(prop.is_continue());
    }

    #[test]
    fn test_event_propagation_clone_copy() {
        let prop1 = EventPropagation::Stop;
        let prop2 = prop1; // Copy
        let prop3 = prop1.clone(); // Clone
        assert_eq!(prop1, prop2);
        assert_eq!(prop1, prop3);
    }

    #[test]
    fn test_event_propagation_default() {
        let prop = EventPropagation::default();
        assert_eq!(prop, EventPropagation::Continue);
    }

    #[test]
    fn test_focus_state_focused() {
        let state = FocusState::Focused;
        assert!(state.is_focused());
        assert!(!state.is_unfocused());
    }

    #[test]
    fn test_focus_state_unfocused() {
        let state = FocusState::Unfocused;
        assert!(!state.is_focused());
        assert!(state.is_unfocused());
    }

    #[test]
    fn test_focus_state_default() {
        let state = FocusState::default();
        assert_eq!(state, FocusState::Unfocused);
    }

    #[test]
    fn test_focus_state_clone_copy() {
        let state1 = FocusState::Focused;
        let state2 = state1; // Copy
        let state3 = state1.clone(); // Clone
        assert_eq!(state1, state2);
        assert_eq!(state1, state3);
    }
}
