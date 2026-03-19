//! MCP Status Panel Component
//!
//! A TUI component that displays the status of MCP (Model Context Protocol) servers.
//! Shows connected, failed, disabled, and needs-auth servers with visual indicators.
//!
//! # Features (TODO)
//!
//! - Display server status with color-coded indicators
//! - Show tool count for connected servers
//! - Expandable view for detailed server info
//! - Periodic polling for status updates
//! - Footer with keyboard shortcuts
//!
//! # Status Colors
//!
//! - Connected: Green
//! - Failed: Red
//! - Disabled: Gray
//! - NeedsAuth: Yellow

mod mock;
mod tests;

use super::{Component, ComponentId, EventPropagation};
use crate::mcp::types::McpStatus;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};
use std::collections::HashMap;
use std::time::Duration;

/// MCP server status entry for display
#[derive(Debug, Clone)]
pub struct McpServerEntry {
    /// Server name
    pub name: String,
    /// Current status
    pub status: McpStatus,
    /// Number of available tools (if connected)
    pub tool_count: usize,
}

/// MCP Status Panel component
///
/// Displays a list of MCP servers with their connection status.
/// Supports expanded view for detailed information and periodic
/// polling for status updates.
///
/// # Examples
///
/// ```rust,ignore
/// let panel = McpStatusPanel::new();
/// // Render in TUI loop
/// panel.render(frame, area);
/// ```
pub struct McpStatusPanel {
    /// Unique component identifier
    id: ComponentId,
    /// List of MCP server entries
    servers: Vec<McpServerEntry>,
    /// Currently selected server index
    selected: usize,
    /// Whether the panel is in expanded view
    expanded: bool,
    /// Whether the panel is focused
    focused: bool,
}

impl McpStatusPanel {
    /// Create a new MCP status panel
    ///
    /// # Returns
    ///
    /// A new `McpStatusPanel` with empty server list.
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            servers: Vec::new(),
            selected: 0,
            expanded: false,
            focused: false,
        }
    }

    /// Set the server list from status map
    ///
    /// # Arguments
    ///
    /// * `statuses` - HashMap of server names to their status
    pub fn set_statuses(&mut self, statuses: HashMap<String, McpStatus>) {
        self.servers = statuses
            .into_iter()
            .map(|(name, status)| McpServerEntry {
                name,
                status,
                tool_count: 0,
            })
            .collect();
        self.servers.sort_by(|a, b| a.name.cmp(&b.name));
    }

    /// Set tool counts for each server
    ///
    /// # Arguments
    ///
    /// * `tools` - HashMap of server names to their tool lists
    pub fn set_tool_counts(&mut self, tools: HashMap<String, usize>) {
        for entry in &mut self.servers {
            if let Some(&count) = tools.get(&entry.name) {
                entry.tool_count = count;
            }
        }
    }

    /// Get the current server list
    pub fn servers(&self) -> &[McpServerEntry] {
        &self.servers
    }

    /// Get the currently selected index
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Check if the panel is in expanded view
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Toggle expanded view
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Select the next server
    pub fn select_next(&mut self) {
        if !self.servers.is_empty() {
            self.selected = (self.selected + 1) % self.servers.len();
        }
    }

    /// Select the previous server
    pub fn select_prev(&mut self) {
        if !self.servers.is_empty() {
            self.selected = if self.selected == 0 {
                self.servers.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    /// Get status color for display
    ///
    /// # Arguments
    ///
    /// * `status` - The MCP status to get color for
    ///
    /// # Returns
    ///
    /// A `ratatui::style::Color` for the status.
    pub fn status_color(status: &McpStatus) -> ratatui::style::Color {
        match status {
            McpStatus::Connected => ratatui::style::Color::Green,
            McpStatus::Failed { .. } => ratatui::style::Color::Red,
            McpStatus::Disabled => ratatui::style::Color::Gray,
            McpStatus::NeedsAuth => ratatui::style::Color::Yellow,
            McpStatus::NeedsClientRegistration { .. } => ratatui::style::Color::Magenta,
        }
    }

    /// Get status display text
    ///
    /// # Arguments
    ///
    /// * `status` - The MCP status to get text for
    ///
    /// # Returns
    ///
    /// A string representation of the status.
    pub fn status_text(status: &McpStatus) -> &'static str {
        match status {
            McpStatus::Connected => "Connected",
            McpStatus::Failed { .. } => "Failed",
            McpStatus::Disabled => "Disabled",
            McpStatus::NeedsAuth => "Needs Auth",
            McpStatus::NeedsClientRegistration { .. } => "Needs Registration",
        }
    }

    /// Get footer text for keyboard shortcuts
    pub fn footer_text() -> &'static str {
        "↑/↓: Navigate | Enter: Expand | Esc: Close"
    }
}

impl Component for McpStatusPanel {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, _frame: &mut Frame, _area: Rect) {
        // TODO: Implement rendering
        // This is intentionally left empty for TDD - tests should fail
        todo!("McpStatusPanel::render not implemented yet")
    }

    fn handle_input(&mut self, _event: KeyEvent) -> EventPropagation {
        // TODO: Implement input handling
        todo!("McpStatusPanel::handle_input not implemented yet")
    }

    fn update(&mut self, _delta: Duration) {
        // TODO: Implement polling updates
        // This is intentionally empty for TDD
    }

    fn is_focusable(&self) -> bool {
        true
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

impl Default for McpStatusPanel {
    fn default() -> Self {
        Self::new()
    }
}
