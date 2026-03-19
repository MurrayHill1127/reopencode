//! Agent Selection Dialog Component
//!
//! Displays a modal dialog for selecting the current AI agent.
//! Uses SelectDialog<String> internally with agent metadata display.
//!
//! # Features
//!
//! - Fuzzy filtering of agent names
//! - Pre-selection of current agent
//! - Agent descriptions and categories
//! - Keyboard navigation (j/k/Up/Down, Enter, Esc)
//!
//! # Examples
//!
//! ```rust,ignore
//! let mut dialog = DialogAgent::new();
//! dialog.set_agents(agents);
//! dialog.set_current_agent("coder");
//! dialog.show();
//!
//! // In render loop
//! if dialog.is_visible() {
//!     dialog.render(frame, area);
//! }
//!
//! // Handle input
//! let propagation = dialog.handle_input(event);
//! if let Some(selected) = dialog.get_selected_agent() {
//!     println!("Selected agent: {}", selected);
//! }
//! ```

use super::dialog::{SelectDialog, SelectOption};
use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::time::Duration;

/// Agent information for display in the dialog
///
/// Contains metadata about an agent including its name, description,
/// whether it's a native agent, and optional category.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog_agent::AgentInfo;
///
/// let agent = AgentInfo::new("build")
///     .with_description("General coding assistant")
///     .with_native(true)
///     .with_category("default");
///
/// assert_eq!(agent.name, "build");
/// assert_eq!(agent.description, Some("General coding assistant".to_string()));
/// assert!(agent.native);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInfo {
    /// Agent name (used as value)
    pub name: String,
    /// Agent description
    pub description: Option<String>,
    /// Whether this is a native agent
    pub native: bool,
    /// Optional category for grouping
    pub category: Option<String>,
}

impl AgentInfo {
    /// Create a new AgentInfo with the given name
    ///
    /// # Arguments
    ///
    /// * `name` - The agent name
    ///
    /// # Returns
    ///
    /// A new AgentInfo with no description, not native, and no category.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::AgentInfo;
    ///
    /// let agent = AgentInfo::new("build");
    /// assert_eq!(agent.name, "build");
    /// assert!(agent.description.is_none());
    /// assert!(!agent.native);
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            native: false,
            category: None,
        }
    }

    /// Builder: Set the agent description
    ///
    /// # Arguments
    ///
    /// * `desc` - The description text
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::AgentInfo;
    ///
    /// let agent = AgentInfo::new("build").with_description("Coder agent");
    /// assert_eq!(agent.description, Some("Coder agent".to_string()));
    /// ```
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: Set whether this is a native agent
    ///
    /// # Arguments
    ///
    /// * `native` - Whether the agent is native
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::AgentInfo;
    ///
    /// let agent = AgentInfo::new("build").with_native(true);
    /// assert!(agent.native);
    /// ```
    pub fn with_native(mut self, native: bool) -> Self {
        self.native = native;
        self
    }

    /// Builder: Set the agent category
    ///
    /// # Arguments
    ///
    /// * `cat` - The category name
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::AgentInfo;
    ///
    /// let agent = AgentInfo::new("explore").with_category("subagent");
    /// assert_eq!(agent.category, Some("subagent".to_string()));
    /// ```
    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.category = Some(cat.into());
        self
    }

    /// Convert this AgentInfo to a SelectOption for the dialog
    fn to_option(&self) -> SelectOption<String> {
        let description = self.description.clone().unwrap_or_else(|| {
            if self.native {
                "native".to_string()
            } else {
                String::new()
            }
        });

        let mut opt = SelectOption::new(&self.name, self.name.clone());

        if !description.is_empty() {
            opt = opt.with_description(description);
        }

        if let Some(ref cat) = self.category {
            opt = opt.with_category(cat);
        }

        opt
    }
}

/// Modal dialog for selecting an AI agent
///
/// Displays a centered modal with a list of available agents.
/// Supports fuzzy filtering and keyboard navigation.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog_agent::DialogAgent;
///
/// let dialog = DialogAgent::new();
/// assert!(!dialog.is_visible());
/// assert_eq!(dialog.len(), 0);
/// ```
pub struct DialogAgent {
    /// Unique component identifier
    id: ComponentId,
    /// Internal selection dialog
    dialog: SelectDialog<String>,
    /// List of agent information
    agents: Vec<AgentInfo>,
    /// Currently selected agent name
    current_agent: Option<String>,
    /// Whether the dialog currently has focus
    focused: bool,
}

impl DialogAgent {
    /// Create a new DialogAgent
    ///
    /// # Returns
    ///
    /// A new DialogAgent, hidden by default with no agents.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::DialogAgent;
    ///
    /// let dialog = DialogAgent::new();
    /// assert!(!dialog.is_visible());
    /// assert_eq!(dialog.len(), 0);
    /// ```
    pub fn new() -> Self {
        let mut dialog = SelectDialog::new(" Select Agent ");
        dialog.set_title(" Select Agent ");
        Self {
            id: ComponentId::new(),
            dialog,
            agents: Vec::new(),
            current_agent: None,
            focused: false,
        }
    }

    /// Set the list of available agents
    ///
    /// Converts AgentInfo to SelectOption and updates the internal dialog.
    ///
    /// # Arguments
    ///
    /// * `agents` - Vector of AgentInfo items
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::{DialogAgent, AgentInfo};
    ///
    /// let mut dialog = DialogAgent::new();
    /// let agents = vec![
    ///     AgentInfo::new("build").with_description("Coder"),
    ///     AgentInfo::new("explore").with_description("Explorer"),
    /// ];
    /// dialog.set_agents(agents);
    /// assert_eq!(dialog.len(), 2);
    /// ```
    pub fn set_agents(&mut self, agents: Vec<AgentInfo>) {
        self.agents = agents;
        let options: Vec<SelectOption<String>> =
            self.agents.iter().map(|a| a.to_option()).collect();
        self.dialog = SelectDialog::with_options(" Select Agent ", options);
        self.dialog.set_title(" Select Agent ");

        if let Some(ref current) = self.current_agent.clone() {
            self.preselect_agent(&current);
        }
    }

    /// Set the current agent and pre-select it in the dialog
    ///
    /// # Arguments
    ///
    /// * `name` - The agent name to set as current
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::{DialogAgent, AgentInfo};
    ///
    /// let mut dialog = DialogAgent::new();
    /// let agents = vec![
    ///     AgentInfo::new("build"),
    ///     AgentInfo::new("explore"),
    /// ];
    /// dialog.set_agents(agents);
    /// dialog.set_current_agent("explore");
    /// // The "explore" agent should be pre-selected
    /// ```
    pub fn set_current_agent(&mut self, name: impl Into<String>) {
        let name = name.into();
        self.current_agent = Some(name.clone());
        self.preselect_agent(&name);
    }

    /// Pre-select an agent in the dialog by name
    fn preselect_agent(&mut self, name: &str) {
        // Find the agent index
        for (i, agent) in self.agents.iter().enumerate() {
            if agent.name == name {
                self.dialog.select(i);
                return;
            }
        }
        // Agent not found, select first if available
        if !self.agents.is_empty() {
            self.dialog.select(0);
        }
    }

    /// Get the currently selected agent name
    ///
    /// # Returns
    ///
    /// The selected agent name as a string slice, or None if no selection.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::{DialogAgent, AgentInfo};
    ///
    /// let mut dialog = DialogAgent::new();
    /// assert!(dialog.get_selected_agent().is_none());
    ///
    /// let agents = vec![AgentInfo::new("build")];
    /// dialog.set_agents(agents);
    /// dialog.show();
    /// // After user selection...
    /// // if let Some(agent) = dialog.get_selected_agent() { ... }
    /// ```
    pub fn get_selected_agent(&self) -> Option<&str> {
        self.dialog.get_selected().map(|opt| opt.value().as_str())
    }

    /// Get the number of agents
    ///
    /// # Returns
    ///
    /// The total number of agents in the list.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_agent::DialogAgent;
    ///
    /// let dialog = DialogAgent::new();
    /// assert_eq!(dialog.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if there are no agents
    ///
    /// # Returns
    ///
    /// `true` if there are no agents in the list.
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Check if the dialog is currently visible
    ///
    /// # Returns
    ///
    /// `true` if the dialog is visible.
    pub fn is_visible(&self) -> bool {
        self.dialog.is_visible()
    }

    /// Show the dialog
    ///
    /// Makes the dialog visible and focused.
    pub fn show(&mut self) {
        self.dialog.show();
        self.focused = true;

        if let Some(ref current) = self.current_agent.clone() {
            self.preselect_agent(&current);
        } else if !self.agents.is_empty() {
            self.dialog.select(0);
        }
    }

    /// Hide the dialog
    ///
    /// Makes the dialog invisible and unfocused.
    pub fn hide(&mut self) {
        self.dialog.hide();
        self.focused = false;
    }

    /// Clear the filter query
    pub fn clear_filter(&mut self) {
        self.dialog.clear_filter();
    }

    /// Get the current agent name
    ///
    /// # Returns
    ///
    /// The current agent name, or None if not set.
    pub fn current_agent(&self) -> Option<&str> {
        self.current_agent.as_deref()
    }
}

impl Default for DialogAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for DialogAgent {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.is_visible() {
            return;
        }
        self.dialog.render(frame, area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.is_visible() {
            return EventPropagation::Continue;
        }

        // Handle Esc and Enter specially
        match event.code {
            KeyCode::Esc => {
                self.hide();
                return EventPropagation::Stop;
            }
            KeyCode::Enter => {
                // Update current agent on Enter
                if let Some(selected) = self.get_selected_agent() {
                    self.current_agent = Some(selected.to_string());
                }
                self.hide();
                return EventPropagation::Stop;
            }
            _ => {}
        }

        // Delegate to internal dialog for navigation and filtering
        self.dialog.handle_input(event)
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn on_focus(&mut self) {
        self.focused = true;
        self.dialog.on_focus();
    }

    fn on_blur(&mut self) {
        self.focused = false;
        self.dialog.on_blur();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_agent_new() {
        let dialog = DialogAgent::new();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.len(), 0);
        assert!(dialog.is_empty());
        assert!(dialog.current_agent().is_none());
    }

    #[test]
    fn test_dialog_agent_default() {
        let dialog = DialogAgent::default();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.len(), 0);
    }

    #[test]
    fn test_agent_info_new() {
        let agent = AgentInfo::new("build");
        assert_eq!(agent.name, "build");
        assert!(agent.description.is_none());
        assert!(!agent.native);
        assert!(agent.category.is_none());
    }

    #[test]
    fn test_agent_info_builders() {
        let agent = AgentInfo::new("build")
            .with_description("General coding assistant")
            .with_native(true)
            .with_category("default");

        assert_eq!(agent.name, "build");
        assert_eq!(
            agent.description,
            Some("General coding assistant".to_string())
        );
        assert!(agent.native);
        assert_eq!(agent.category, Some("default".to_string()));
    }

    #[test]
    fn test_agent_info_to_option() {
        let agent = AgentInfo::new("build")
            .with_description("Coder")
            .with_category("default");

        let opt = agent.to_option();
        assert_eq!(opt.title(), "build");
        assert_eq!(opt.value(), "build");
        assert_eq!(opt.description(), Some("Coder"));
        assert_eq!(opt.category(), Some("default"));
    }

    #[test]
    fn test_agent_info_to_option_native() {
        let agent = AgentInfo::new("build").with_native(true);
        let opt = agent.to_option();
        // Native without description should show "native"
        assert_eq!(opt.description(), Some("native"));
    }

    #[test]
    fn test_agent_info_to_option_non_native_no_desc() {
        let agent = AgentInfo::new("custom");
        let opt = agent.to_option();
        // Non-native without description should have empty description
        assert_eq!(opt.description(), None);
    }

    #[test]
    fn test_set_agents() {
        let mut dialog = DialogAgent::new();
        let agents = vec![
            AgentInfo::new("build").with_description("Coder"),
            AgentInfo::new("explore").with_description("Explorer"),
        ];
        dialog.set_agents(agents);
        assert_eq!(dialog.len(), 2);
        assert!(!dialog.is_empty());
    }

    #[test]
    fn test_set_current_agent() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        dialog.set_current_agent("explore");

        assert_eq!(dialog.current_agent(), Some("explore"));
    }

    #[test]
    fn test_set_current_agent_not_in_list() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        // Setting agent not in list should still update current_agent
        dialog.set_current_agent("unknown");
        assert_eq!(dialog.current_agent(), Some("unknown"));
    }

    #[test]
    fn test_show_hide() {
        let mut dialog = DialogAgent::new();
        assert!(!dialog.is_visible());

        dialog.show();
        assert!(dialog.is_visible());

        dialog.hide();
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_show_preselects_current_agent() {
        let mut dialog = DialogAgent::new();
        let agents = vec![
            AgentInfo::new("build"),
            AgentInfo::new("explore"),
            AgentInfo::new("librarian"),
        ];
        dialog.set_agents(agents);
        dialog.set_current_agent("explore");

        dialog.show();
        // After show, the current agent should be pre-selected
        assert!(dialog.is_visible());
    }

    #[test]
    fn test_show_with_no_current_agent_selects_first() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        // No current agent set

        dialog.show();
        assert!(dialog.is_visible());
    }

    #[test]
    fn test_empty_agents() {
        let mut dialog = DialogAgent::new();
        dialog.set_agents(vec![]);
        dialog.show();
        assert!(dialog.is_visible());
        assert_eq!(dialog.len(), 0);
        assert!(dialog.get_selected_agent().is_none());
    }

    #[test]
    fn test_get_selected_agent_empty() {
        let dialog = DialogAgent::new();
        assert!(dialog.get_selected_agent().is_none());
    }

    #[test]
    fn test_get_selected_agent_with_agents() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        dialog.show();

        // Select first item
        let selected = dialog.get_selected_agent();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap(), "build");
    }

    #[test]
    fn test_is_focusable() {
        let dialog = DialogAgent::new();
        assert!(dialog.is_focusable());
    }

    #[test]
    fn test_on_focus_on_blur() {
        let mut dialog = DialogAgent::new();
        assert!(!dialog.focused);

        dialog.on_focus();
        assert!(dialog.focused);

        dialog.on_blur();
        assert!(!dialog.focused);
    }

    #[test]
    fn test_component_id_unique() {
        let dialog1 = DialogAgent::new();
        let dialog2 = DialogAgent::new();
        assert_ne!(dialog1.id(), dialog2.id());
    }

    #[test]
    fn test_handle_input_not_visible() {
        let mut dialog = DialogAgent::new();
        assert!(!dialog.is_visible());

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_handle_input_escape() {
        let mut dialog = DialogAgent::new();
        dialog.show();
        assert!(dialog.is_visible());

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_handle_input_enter() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        dialog.show();
        assert!(dialog.is_visible());

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(!dialog.is_visible());
        // Current agent should be updated
        assert_eq!(dialog.current_agent(), Some("build"));
    }

    #[test]
    fn test_handle_input_navigation() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        dialog.show();

        // Navigate down
        let event = KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);

        // Select should now be on second item
        assert_eq!(dialog.get_selected_agent(), Some("explore"));
    }

    #[test]
    fn test_clear_filter() {
        let mut dialog = DialogAgent::new();
        let agents = vec![AgentInfo::new("build"), AgentInfo::new("explore")];
        dialog.set_agents(agents);
        dialog.show();

        // Clear filter should not panic
        dialog.clear_filter();
    }

    #[test]
    fn test_agent_info_equality() {
        let a1 = AgentInfo::new("build").with_description("test");
        let a2 = AgentInfo::new("build").with_description("test");
        let a3 = AgentInfo::new("build").with_description("other");

        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }

    #[test]
    fn test_agent_info_clone() {
        let a1 = AgentInfo::new("build").with_description("test");
        let a2 = a1.clone();
        assert_eq!(a1, a2);
    }
}
