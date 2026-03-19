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

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use super::{Component, ComponentId, EventPropagation};
use crate::mcp::types::McpStatus;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
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

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.servers.is_empty() {
            // Empty state
            let block = Block::default()
                .title("MCP Servers")
                .borders(Borders::ALL)
                .border_style(if self.focused {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default()
                });

            let text = Paragraph::new("No MCP servers configured").block(block);
            frame.render_widget(text, area);
            return;
        }

        // Build the block with title
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let block = Block::default()
            .title("MCP Servers")
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create list items for each server
        let items: Vec<ListItem> = self
            .servers
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let color = Self::status_color(&entry.status);
                let status_indicator = match &entry.status {
                    McpStatus::Connected => "●",
                    McpStatus::Failed { .. } => "✗",
                    McpStatus::Disabled => "○",
                    McpStatus::NeedsAuth => "⚠",
                    McpStatus::NeedsClientRegistration { .. } => "?",
                };

                let tool_info = if entry.tool_count > 0 {
                    format!(" ({} tools)", entry.tool_count)
                } else {
                    String::new()
                };

                let line = if self.expanded {
                    Line::from(vec![
                        Span::styled(status_indicator, Style::default().fg(color)),
                        Span::raw(" "),
                        Span::styled(&entry.name, Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(tool_info),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(status_indicator, Style::default().fg(color)),
                        Span::raw(" "),
                        Span::raw(&entry.name),
                    ])
                };

                let mut item = ListItem::new(line);
                if i == self.selected && self.focused {
                    item = item.style(Style::default().bg(Color::DarkGray));
                }
                item
            })
            .collect();

        let list = List::new(items).block(block);

        if self.focused {
            let mut state = ListState::default();
            state.select(Some(self.selected));
            frame.render_stateful_widget(list, area, &mut state);
        } else {
            frame.render_widget(list, area);
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        if !self.focused {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.select_prev();
                EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.select_next();
                EventPropagation::Stop
            }
            (KeyCode::Enter, _) => {
                self.toggle_expanded();
                EventPropagation::Stop
            }
            (KeyCode::Esc, _) => {
                if self.expanded {
                    self.expanded = false;
                }
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {
        // Placeholder for polling updates if needed
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
