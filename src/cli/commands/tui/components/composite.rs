//! Component Composition System
//!
//! Provides utilities for composing multiple components into layouts
//! and managing event routing between components.

use super::{
    Component, ComponentAction, ComponentEvent, ComponentId, EventPropagation, FocusManager,
};
use anyhow::Result;
use ratatui::{layout::Rect, Frame};

/// Pump events through component chain, stopping when consumed.
///
/// Iterates through components in order, passing the event to each.
/// Returns `Stop` as soon as any component consumes the event.
pub fn event_pump(
    components: &mut [&mut dyn Component],
    event: &ComponentEvent,
) -> EventPropagation {
    for component in components {
        if component.handle_event(event) == EventPropagation::Stop {
            return EventPropagation::Stop;
        }
    }
    EventPropagation::Continue
}

/// Pump events with focus priority.
///
/// First delivers the event to the focused component if it matches the focused_id.
/// If the focused component doesn't consume it, falls back to broadcasting to all components.
pub fn event_pump_with_focus(
    components: &mut [&mut dyn Component],
    focused_id: &ComponentId,
    event: &ComponentEvent,
) -> EventPropagation {
    // First, try focused component
    for component in components.iter_mut() {
        if component.id() == *focused_id && component.focused() {
            if component.handle_event(event) == EventPropagation::Stop {
                return EventPropagation::Stop;
            }
        }
    }
    // Then, try other components
    event_pump(components, event)
}

/// Render all components in their allocated areas.
///
/// Components and areas must have matching lengths.
pub fn render_all(
    components: &mut [&mut dyn Component],
    frame: &mut Frame,
    areas: &[Rect],
) -> Result<()> {
    if components.len() != areas.len() {
        anyhow::bail!(
            "Components ({}) and areas ({}) count mismatch",
            components.len(),
            areas.len()
        );
    }

    for (component, area) in components.iter_mut().zip(areas.iter()) {
        component.render(frame, *area);
    }
    Ok(())
}

/// Update all components with a ComponentAction.
///
/// Dispatches actions like FocusNext, FocusPrev, etc. to components.
pub fn update_all(components: &mut [&mut dyn Component], action: &ComponentAction) {
    for _component in components {
        match action {
            ComponentAction::FocusNext | ComponentAction::FocusPrev => {
                // Focus navigation handled by FocusManager
            }
            ComponentAction::Render => {}
            ComponentAction::Quit => {}
            ComponentAction::Submit(_) => {}
            ComponentAction::ShowToast { .. } => {}
            ComponentAction::CloseDialog => {}
            ComponentAction::Custom(_) => {}
        }
    }
}

/// Registry for managing multiple components with focus support.
pub struct ComponentRegistry {
    components: Vec<Box<dyn Component>>,
    focus_manager: FocusManager,
}

impl ComponentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            focus_manager: FocusManager::new(),
        }
    }

    /// Register a component in the registry.
    ///
    /// If the component is focusable, it's added to the focus ring.
    pub fn register(&mut self, component: Box<dyn Component>) {
        let id = component.id();
        let is_focusable = component.is_focusable();
        self.components.push(component);
        if is_focusable {
            self.focus_manager.register(id);
        }
    }

    /// Remove a component by ID.
    ///
    /// Returns true if the component was found and removed.
    pub fn unregister(&mut self, id: &ComponentId) -> bool {
        if let Some(pos) = self.components.iter().position(|c| c.id() == *id) {
            self.components.remove(pos);
            self.focus_manager.unregister(*id);
            true
        } else {
            false
        }
    }

    /// Handle an event, routing to focused component first.
    pub fn handle_event(&mut self, event: &ComponentEvent) -> EventPropagation {
        if let Some(focused_id) = self.focus_manager.get_focused() {
            // First, try focused component
            for component in &mut self.components {
                if component.id() == focused_id && component.focused() {
                    if component.handle_event(event) == EventPropagation::Stop {
                        return EventPropagation::Stop;
                    }
                }
            }
        }
        // Then, try all components
        for component in &mut self.components {
            if component.handle_event(event) == EventPropagation::Stop {
                return EventPropagation::Stop;
            }
        }
        EventPropagation::Continue
    }

    /// Dispatch an action to all components.
    pub fn update_all_components(&mut self, action: &ComponentAction) {
        for component in &mut self.components {
            let _ = action;
            // Actions are handled by the registry, not individual components
        }
    }

    /// Render all components to the frame.
    pub fn render_components(&mut self, frame: &mut Frame, areas: &[Rect]) -> Result<()> {
        if self.components.len() != areas.len() {
            anyhow::bail!(
                "Components ({}) and areas ({}) count mismatch",
                self.components.len(),
                areas.len()
            );
        }
        for (component, area) in self.components.iter_mut().zip(areas.iter()) {
            component.render(frame, *area);
        }
        Ok(())
    }

    /// Move focus to the next component in the focus ring.
    pub fn focus_next(&mut self) {
        self.focus_manager.next_focus();
        self.sync_focus_state();
    }

    /// Move focus to the previous component in the focus ring.
    pub fn focus_prev(&mut self) {
        self.focus_manager.prev_focus();
        self.sync_focus_state();
    }

    /// Get the currently focused component ID.
    pub fn get_focused(&self) -> Option<ComponentId> {
        self.focus_manager.get_focused()
    }

    /// Sync focus state between FocusManager and components.
    fn sync_focus_state(&mut self) {
        let focused_id = self.focus_manager.get_focused();
        for component in &mut self.components {
            let should_be_focused = focused_id == Some(component.id());
            let currently_focused = component.focused();
            if should_be_focused && !currently_focused {
                component.on_focus();
            } else if !should_be_focused && currently_focused {
                component.on_blur();
            }
        }
    }

    /// Get the number of registered components.
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::commands::tui::components::Component;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    struct MockComponent {
        id: ComponentId,
        focused: bool,
        is_focusable: bool,
    }

    impl MockComponent {
        fn new() -> Self {
            Self {
                id: ComponentId::new(),
                focused: false,
                is_focusable: true,
            }
        }

        fn non_focusable() -> Self {
            Self {
                id: ComponentId::new(),
                focused: false,
                is_focusable: false,
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

        fn is_focusable(&self) -> bool {
            self.is_focusable
        }

        fn focused(&self) -> bool {
            self.focused
        }

        fn on_focus(&mut self) {
            self.focused = true;
        }

        fn on_blur(&mut self) {
            self.focused = false;
        }
    }

    #[test]
    fn test_event_pump_stops_on_consume() {
        let mut comp1 = MockComponent::new();
        let mut comp2 = MockComponent::new();
        let event = ComponentEvent::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));

        let result = event_pump(&mut [&mut comp1, &mut comp2], &event);
        assert_eq!(result, EventPropagation::Stop);
    }

    #[test]
    fn test_event_pump_with_focus_priority() {
        let mut comp1 = MockComponent::new();
        let mut comp2 = MockComponent::new();
        let focused_id = comp1.id();
        comp1.on_focus();

        let event = ComponentEvent::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        let result = event_pump_with_focus(&mut [&mut comp1, &mut comp2], &focused_id, &event);
        assert_eq!(result, EventPropagation::Stop);
    }

    #[test]
    fn test_registry_new() {
        let registry = ComponentRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = ComponentRegistry::new();
        registry.register(Box::new(MockComponent::new()));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = ComponentRegistry::new();
        let comp = MockComponent::new();
        let id = comp.id();
        registry.register(Box::new(comp));
        assert!(registry.unregister(&id));
        assert!(!registry.unregister(&id));
    }

    #[test]
    fn test_registry_focus_navigation() {
        let mut registry = ComponentRegistry::new();
        let comp1 = MockComponent::new();
        let comp2 = MockComponent::new();
        let id1 = comp1.id();
        let id2 = comp2.id();

        registry.register(Box::new(comp1));
        registry.register(Box::new(comp2));

        registry.focus_next();
        assert_eq!(registry.get_focused(), Some(id1));

        registry.focus_next();
        assert_eq!(registry.get_focused(), Some(id2));

        registry.focus_prev();
        assert_eq!(registry.get_focused(), Some(id1));
    }

    #[test]
    fn test_registry_non_focusable_component() {
        let mut registry = ComponentRegistry::new();
        let comp = MockComponent::non_focusable();

        registry.register(Box::new(comp));

        registry.focus_next();
        assert_eq!(registry.get_focused(), None);
    }

    #[test]
    fn test_render_all_success() {
        let mut comp1 = MockComponent::new();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let areas = [Rect::default()];

        terminal
            .draw(|frame| {
                let result = render_all(&mut [&mut comp1], frame, &areas);
                assert!(result.is_ok());
            })
            .unwrap();
    }

    #[test]
    fn test_render_all_mismatch() {
        let mut comp1 = MockComponent::new();
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let areas = [Rect::default(), Rect::default()];
                let result = render_all(&mut [&mut comp1], frame, &areas);
                assert!(result.is_err());
            })
            .unwrap();
    }
}
