//! Status Dialog Component
//!
//! A modal dialog that displays the status of various system components:
//! - MCP Servers with connection status
//! - LSP Servers
//! - Formatters
//! - Plugins
//!
//! # Features
//!
//! - Color-coded status indicators
//! - Scrollable content with scrollbar
//! - Modal overlay behavior
//! - ESC to close

use super::{Component, ComponentId, EventPropagation};
use crate::mcp::types::McpStatus;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::collections::HashMap;
use std::time::Duration;

/// Display line for the status dialog
#[derive(Debug, Clone)]
struct DisplayLine {
    /// Line content
    content: Line<'static>,
}

/// Status dialog component showing MCP, LSP, Formatters, and Plugins status
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::status_dialog::StatusDialog;
///
/// let mut dialog = StatusDialog::new();
/// dialog.show();
/// assert!(dialog.is_visible());
/// ```
pub struct StatusDialog {
    /// Unique component identifier
    id: ComponentId,
    /// Whether the dialog is currently visible
    visible: bool,
    /// Whether the dialog currently has focus
    focused: bool,
    /// MCP server statuses
    mcp_statuses: HashMap<String, McpStatus>,
    /// LSP server count
    lsp_count: usize,
    /// Scroll offset
    scroll_offset: usize,
    /// Total content height
    content_height: usize,
    /// Display lines cache
    display_lines: Vec<DisplayLine>,
}

impl StatusDialog {
    /// Create a new StatusDialog
    ///
    /// # Returns
    ///
    /// A new `StatusDialog`, hidden by default.
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            visible: false,
            focused: false,
            mcp_statuses: HashMap::new(),
            lsp_count: 0,
            scroll_offset: 0,
            content_height: 0,
            display_lines: Vec::new(),
        }
    }

    /// Set MCP server statuses
    ///
    /// # Arguments
    ///
    /// * `statuses` - HashMap of server names to their status
    pub fn set_mcp_statuses(&mut self, statuses: HashMap<String, McpStatus>) {
        self.mcp_statuses = statuses;
        self.rebuild_display_lines();
    }

    /// Set LSP server count
    ///
    /// # Arguments
    ///
    /// * `count` - Number of LSP servers
    pub fn set_lsp_count(&mut self, count: usize) {
        self.lsp_count = count;
        self.rebuild_display_lines();
    }

    /// Check if the dialog is currently visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
        self.scroll_offset = 0;
        self.rebuild_display_lines();
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Check if the dialog is focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Calculate the centered area for the dialog
    ///
    /// Dialog size: 60% width, 70% height, capped at 80x30
    fn centered_area(&self, frame_area: Rect) -> Rect {
        let width_percent = 60u16;
        let height_percent = 70u16;
        let max_width = 80u16;
        let max_height = 30u16;

        let width = ((frame_area.width as u32 * width_percent as u32 / 100) as u16).min(max_width);
        let height =
            ((frame_area.height as u32 * height_percent as u32 / 100) as u16).min(max_height);

        let x = frame_area.x + (frame_area.width.saturating_sub(width)) / 2;
        let y = frame_area.y + (frame_area.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }

    /// Get status color for MCP status
    fn status_color(status: &McpStatus) -> Color {
        match status {
            McpStatus::Connected => Color::Green,
            McpStatus::Failed { .. } => Color::Red,
            McpStatus::Disabled => Color::Gray,
            McpStatus::NeedsAuth => Color::Yellow,
            McpStatus::NeedsClientRegistration { .. } => Color::Magenta,
        }
    }

    /// Get status text for MCP status
    fn status_text(status: &McpStatus) -> String {
        match status {
            McpStatus::Connected => "Connected".to_string(),
            McpStatus::Failed { error } => format!("Failed: {}", error),
            McpStatus::Disabled => "Disabled in configuration".to_string(),
            McpStatus::NeedsAuth => "Needs authentication".to_string(),
            McpStatus::NeedsClientRegistration { error } => error.clone(),
        }
    }

    /// Rebuild display lines from current state
    fn rebuild_display_lines(&mut self) {
        self.display_lines.clear();

        // Header
        self.display_lines.push(DisplayLine {
            content: Line::from(Span::styled(
                "Status",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )),
        });

        self.display_lines.push(DisplayLine {
            content: Line::default(),
        });

        // MCP Servers section
        self.build_mcp_section();

        // LSP Servers section
        self.build_lsp_section();

        // Formatters section
        self.build_formatters_section();

        // Plugins section
        self.build_plugins_section();

        self.content_height = self.display_lines.len();
        self.clamp_scroll();
    }

    /// Build MCP servers section
    fn build_mcp_section(&mut self) {
        let mcp_count = self.mcp_statuses.len();

        if mcp_count == 0 {
            self.display_lines.push(DisplayLine {
                content: Line::from(Span::styled(
                    "No MCP Servers",
                    Style::default().fg(Color::White),
                )),
            });
        } else {
            self.display_lines.push(DisplayLine {
                content: Line::from(Span::styled(
                    format!("{} MCP Servers", mcp_count),
                    Style::default().fg(Color::White),
                )),
            });

            // Sort server names alphabetically
            let mut servers: Vec<_> = self.mcp_statuses.iter().collect();
            servers.sort_by(|a, b| a.0.cmp(b.0));

            for (name, status) in servers {
                let color = Self::status_color(status);
                let status_text = Self::status_text(status);

                self.display_lines.push(DisplayLine {
                    content: Line::from(vec![
                        Span::styled("  • ", Style::default().fg(color)),
                        Span::styled(name.clone(), Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" "),
                        Span::styled(status_text, Style::default().fg(Color::DarkGray)),
                    ]),
                });
            }
        }

        self.display_lines.push(DisplayLine {
            content: Line::default(),
        });
    }

    /// Build LSP servers section
    fn build_lsp_section(&mut self) {
        if self.lsp_count == 0 {
            return;
        }

        self.display_lines.push(DisplayLine {
            content: Line::from(Span::styled(
                format!("{} LSP Servers", self.lsp_count),
                Style::default().fg(Color::White),
            )),
        });

        self.display_lines.push(DisplayLine {
            content: Line::from(vec![
                Span::styled("  • ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{} active", self.lsp_count),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        });

        self.display_lines.push(DisplayLine {
            content: Line::default(),
        });
    }

    /// Build formatters section
    fn build_formatters_section(&mut self) {
        self.display_lines.push(DisplayLine {
            content: Line::from(Span::styled(
                "No Formatters",
                Style::default().fg(Color::White),
            )),
        });

        self.display_lines.push(DisplayLine {
            content: Line::default(),
        });
    }

    /// Build plugins section
    fn build_plugins_section(&mut self) {
        self.display_lines.push(DisplayLine {
            content: Line::from(Span::styled(
                "No Plugins",
                Style::default().fg(Color::White),
            )),
        });
    }

    /// Clamp scroll offset to valid range
    fn clamp_scroll(&mut self) {
        let max_scroll = self.content_height.saturating_sub(1);
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }

    /// Scroll up by one line
    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down by one line
    fn scroll_down(&mut self) {
        let max_scroll = self.content_height.saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Scroll up by one page
    fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by one page
    fn scroll_page_down(&mut self, page_size: usize) {
        let max_scroll = self.content_height.saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_scroll);
    }

    /// Scroll to top
    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to bottom
    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.content_height.saturating_sub(1);
    }
}

impl Component for StatusDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = self.centered_area(area);

        // Clear the area for modal overlay effect
        frame.render_widget(Clear, dialog_area);

        // Dialog block with border
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };

        let block = Block::default()
            .title(" Status - Press Esc to close ")
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner_area = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Calculate layout: content area + footer
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner_area);

        let content_area = layout[0];
        let footer_area = layout[1];

        // Render content
        let visible_height = content_area.height as usize;
        let mut lines_to_render = Vec::new();

        // Calculate which lines to show
        let end_idx = (self.scroll_offset + visible_height).min(self.display_lines.len());

        for line_idx in self.scroll_offset..end_idx {
            lines_to_render.push(self.display_lines[line_idx].content.clone());
        }

        // Pad with empty lines if needed
        while lines_to_render.len() < visible_height {
            lines_to_render.push(Line::default());
        }

        let content = Paragraph::new(lines_to_render);
        frame.render_widget(content, content_area);

        // Render scrollbar
        if self.content_height > visible_height {
            let scrollbar_area = Rect::new(
                content_area.x + content_area.width.saturating_sub(1),
                content_area.y,
                1,
                content_area.height,
            );

            let scrollbar_state = ScrollbarState::new(self.content_height)
                .position(self.scroll_offset)
                .viewport_content_length(visible_height);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state.clone());
        }

        // Render footer with navigation hints
        let footer_text = " ↑/↓ or j/k: scroll | PgUp/PgDn: page | Home/End: jump | Esc: close ";
        let footer = Paragraph::new(Line::from(Span::styled(
            footer_text,
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(footer, footer_area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.visible {
            return EventPropagation::Continue;
        }

        match event.code {
            KeyCode::Esc => {
                self.hide();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down();
            }
            KeyCode::PageUp => {
                self.scroll_page_up(10);
            }
            KeyCode::PageDown => {
                self.scroll_page_down(10);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll_to_top();
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll_to_bottom();
            }
            _ => {}
        }

        EventPropagation::Stop
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed
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

impl Default for StatusDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_dialog_new() {
        let dialog = StatusDialog::new();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_status_dialog_show_hide() {
        let mut dialog = StatusDialog::new();

        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_status_dialog_mcp_statuses() {
        let mut dialog = StatusDialog::new();
        let mut statuses = HashMap::new();
        statuses.insert("server1".to_string(), McpStatus::Connected);
        statuses.insert("server2".to_string(), McpStatus::Disabled);

        dialog.set_mcp_statuses(statuses);
        assert_eq!(dialog.mcp_statuses.len(), 2);
    }

    #[test]
    fn test_status_dialog_lsp_count() {
        let mut dialog = StatusDialog::new();
        dialog.set_lsp_count(5);
        assert_eq!(dialog.lsp_count, 5);
    }

    #[test]
    fn test_status_color_connected() {
        let status = McpStatus::Connected;
        assert_eq!(StatusDialog::status_color(&status), Color::Green);
    }

    #[test]
    fn test_status_color_failed() {
        let status = McpStatus::Failed {
            error: "test error".to_string(),
        };
        assert_eq!(StatusDialog::status_color(&status), Color::Red);
    }

    #[test]
    fn test_status_color_disabled() {
        let status = McpStatus::Disabled;
        assert_eq!(StatusDialog::status_color(&status), Color::Gray);
    }

    #[test]
    fn test_status_color_needs_auth() {
        let status = McpStatus::NeedsAuth;
        assert_eq!(StatusDialog::status_color(&status), Color::Yellow);
    }

    #[test]
    fn test_status_color_needs_registration() {
        let status = McpStatus::NeedsClientRegistration {
            error: "test".to_string(),
        };
        assert_eq!(StatusDialog::status_color(&status), Color::Magenta);
    }

    #[test]
    fn test_status_text_connected() {
        let status = McpStatus::Connected;
        assert_eq!(StatusDialog::status_text(&status), "Connected");
    }

    #[test]
    fn test_status_text_failed() {
        let status = McpStatus::Failed {
            error: "connection refused".to_string(),
        };
        assert_eq!(
            StatusDialog::status_text(&status),
            "Failed: connection refused"
        );
    }

    #[test]
    fn test_status_text_disabled() {
        let status = McpStatus::Disabled;
        assert_eq!(
            StatusDialog::status_text(&status),
            "Disabled in configuration"
        );
    }

    #[test]
    fn test_status_text_needs_auth() {
        let status = McpStatus::NeedsAuth;
        assert_eq!(StatusDialog::status_text(&status), "Needs authentication");
    }

    #[test]
    fn test_scroll_behavior() {
        let mut dialog = StatusDialog::new();
        let mut statuses = HashMap::new();

        // Add many servers to enable scrolling
        for i in 0..30 {
            statuses.insert(format!("server{}", i), McpStatus::Connected);
        }
        dialog.set_mcp_statuses(statuses);

        // Test scrolling
        dialog.scroll_down();
        assert_eq!(dialog.scroll_offset, 1);

        dialog.scroll_page_down(10);
        assert_eq!(dialog.scroll_offset, 11);

        dialog.scroll_to_top();
        assert_eq!(dialog.scroll_offset, 0);

        dialog.scroll_to_bottom();
        assert!(dialog.scroll_offset > 0);
    }
}
