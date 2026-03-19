//! Footer Component
//!
//! Displays footer information: directory path, welcome message, LSP/MCP/permissions indicators, /status hint.
//!
//! # Welcome Message Cycling
//! - Hidden for 10 seconds
//! - Visible for 5 seconds (shows "Get started /connect")
//! - Repeats
//! - Only shown when NOT connected (session_id is None)

use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::{Component, ComponentId, EventPropagation};
use crate::cli::commands::tui::theme::ThemeContext;
use crate::mcp::types::McpStatus;

/// Welcome state for timer cycling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WelcomeState {
    /// Welcome hidden, countdown to show
    Hidden(Duration),
    /// Welcome visible, countdown to hide
    Visible(Duration),
}

/// Timer constants matching TypeScript behavior
const WELCOME_HIDDEN_SECS: u64 = 10;
const WELCOME_VISIBLE_SECS: u64 = 5;

/// Footer component for TUI
pub struct Footer {
    id: ComponentId,
    /// Timer state for welcome message cycling
    welcome_state: WelcomeState,
    /// Theme context for colors
    theme: ThemeContext,
    /// Current directory path
    directory: String,
    /// MCP server statuses
    mcp_statuses: HashMap<String, McpStatus>,
    /// Number of LSP servers
    lsp_count: usize,
    /// Pending permission count
    pending_permissions: usize,
    /// Session ID (None = not connected)
    session_id: Option<String>,
}

impl Footer {
    /// Create a new Footer component
    pub fn new(theme: ThemeContext) -> Self {
        let current_dir = std::env::current_dir()
            .ok()
            .map(|p| {
                let path_str = p.to_string_lossy().to_string();
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy();
                    if path_str.starts_with(&*home_str) {
                        return path_str.replacen(&*home_str, "~", 1);
                    }
                }
                path_str
            })
            .unwrap_or_else(|| ".".to_string());

        Self {
            id: ComponentId::new(),
            welcome_state: WelcomeState::Hidden(Duration::from_secs(WELCOME_HIDDEN_SECS)),
            theme,
            directory: current_dir,
            mcp_statuses: HashMap::new(),
            lsp_count: 0,
            pending_permissions: 0,
            session_id: None,
        }
    }

    /// Check if connected to a session
    fn is_connected(&self) -> bool {
        self.session_id.is_some()
    }

    /// Check if welcome message should be shown
    fn show_welcome(&self) -> bool {
        !self.is_connected() && matches!(self.welcome_state, WelcomeState::Visible(_))
    }

    /// Set directory path
    pub fn set_directory(&mut self, dir: String) {
        self.directory = dir;
    }

    /// Set MCP statuses
    pub fn set_mcp_statuses(&mut self, statuses: HashMap<String, McpStatus>) {
        self.mcp_statuses = statuses;
    }

    /// Set LSP count
    pub fn set_lsp_count(&mut self, count: usize) {
        self.lsp_count = count;
    }

    /// Set pending permissions count
    pub fn set_pending_permissions(&mut self, count: usize) {
        self.pending_permissions = count;
    }

    /// Set session ID (None = not connected)
    pub fn set_session_id(&mut self, id: Option<String>) {
        self.session_id = id;
    }

    /// Get directory path
    pub fn directory(&self) -> &str {
        &self.directory
    }
}

impl Component for Footer {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        // Split area into left (directory) and right (indicators)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(10),    // Left: directory (flexible)
                Constraint::Length(60), // Right: indicators (fixed width)
            ])
            .split(area);

        // Left side: directory path
        let directory_para = Paragraph::new(self.directory.as_str())
            .style(Style::default().fg(self.theme.text_muted()));
        frame.render_widget(directory_para, chunks[0]);

        // Right side: welcome OR indicators
        let right_content = if self.show_welcome() {
            // Show welcome message
            Line::from(vec![
                Span::styled("Get started ", Style::default().fg(self.theme.text())),
                Span::styled("/connect", Style::default().fg(self.theme.text_muted())),
            ])
        } else if self.is_connected() {
            // Show indicators
            let mut spans = Vec::new();

            // Permissions indicator
            if self.pending_permissions > 0 {
                let perm_text = if self.pending_permissions == 1 {
                    format!("△ {} Permission", self.pending_permissions)
                } else {
                    format!("△ {} Permissions", self.pending_permissions)
                };
                spans.push(Span::styled(
                    perm_text,
                    Style::default().fg(self.theme.warning()),
                ));
                spans.push(Span::raw("  "));
            }

            // LSP indicator
            let lsp_color = if self.lsp_count > 0 {
                self.theme.success()
            } else {
                self.theme.text_muted()
            };
            spans.push(Span::styled(
                format!("• {} LSP", self.lsp_count),
                Style::default().fg(lsp_color),
            ));
            spans.push(Span::raw("  "));

            // MCP indicator
            let mcp_count = self.mcp_statuses.len();
            let all_ok = self
                .mcp_statuses
                .values()
                .all(|s| matches!(s, McpStatus::Connected));
            let mcp_color = if mcp_count == 0 {
                self.theme.text_muted()
            } else if all_ok {
                self.theme.success()
            } else {
                self.theme.error()
            };
            spans.push(Span::styled(
                format!("⊙ {} MCP", mcp_count),
                Style::default().fg(mcp_color),
            ));
            spans.push(Span::raw("  "));

            // /status hint
            spans.push(Span::styled(
                "/status",
                Style::default().fg(self.theme.text_muted()),
            ));

            Line::from(spans)
        } else {
            // Not connected, not showing welcome - empty
            Line::from(vec![])
        };

        let right_para = Paragraph::new(right_content);
        frame.render_widget(right_para, chunks[1]);
    }

    fn update(&mut self, delta: Duration) {
        // Only cycle welcome when NOT connected
        if self.is_connected() {
            return;
        }

        // Update timer state
        self.welcome_state = match self.welcome_state {
            WelcomeState::Hidden(remaining) => {
                let new_remaining = remaining.saturating_sub(delta);
                if new_remaining.is_zero() {
                    // Transition to visible
                    WelcomeState::Visible(Duration::from_secs(WELCOME_VISIBLE_SECS))
                } else {
                    WelcomeState::Hidden(new_remaining)
                }
            }
            WelcomeState::Visible(remaining) => {
                let new_remaining = remaining.saturating_sub(delta);
                if new_remaining.is_zero() {
                    // Transition to hidden
                    WelcomeState::Hidden(Duration::from_secs(WELCOME_HIDDEN_SECS))
                } else {
                    WelcomeState::Visible(new_remaining)
                }
            }
        };
    }

    fn handle_input(&mut self, _event: KeyEvent) -> EventPropagation {
        EventPropagation::Continue
    }

    fn is_focusable(&self) -> bool {
        false
    }
}

impl Default for Footer {
    fn default() -> Self {
        Self::new(ThemeContext::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footer_new() {
        let footer = Footer::new(ThemeContext::default());
        assert!(!footer.is_connected());
        assert!(!footer.show_welcome()); // Initially hidden
    }

    #[test]
    fn test_footer_connected_state() {
        let mut footer = Footer::new(ThemeContext::default());
        assert!(!footer.is_connected());

        footer.set_session_id(Some("test-session".to_string()));
        assert!(footer.is_connected());
        assert!(!footer.show_welcome()); // Never show welcome when connected
    }

    #[test]
    fn test_footer_welcome_cycle() {
        let mut footer = Footer::new(ThemeContext::default());

        // Initially hidden
        assert!(matches!(footer.welcome_state, WelcomeState::Hidden(_)));
        assert!(!footer.show_welcome());

        // After 10s, should be visible
        footer.update(Duration::from_secs(10));
        assert!(matches!(footer.welcome_state, WelcomeState::Visible(_)));
        assert!(footer.show_welcome());

        // After 5s more, should be hidden again
        footer.update(Duration::from_secs(5));
        assert!(matches!(footer.welcome_state, WelcomeState::Hidden(_)));
        assert!(!footer.show_welcome());
    }

    #[test]
    fn test_footer_no_welcome_when_connected() {
        let mut footer = Footer::new(ThemeContext::default());
        footer.set_session_id(Some("session".to_string()));

        // Timer should not advance when connected
        footer.update(Duration::from_secs(15));
        // State should remain at initial hidden
        assert!(matches!(footer.welcome_state, WelcomeState::Hidden(_)));
    }

    #[test]
    fn test_footer_setters() {
        let mut footer = Footer::new(ThemeContext::default());

        footer.set_directory("/test/path".to_string());
        assert_eq!(footer.directory(), "/test/path");

        footer.set_lsp_count(3);
        assert_eq!(footer.lsp_count, 3);

        footer.set_pending_permissions(2);
        assert_eq!(footer.pending_permissions, 2);

        let mut mcp = HashMap::new();
        mcp.insert("server1".to_string(), McpStatus::Connected);
        footer.set_mcp_statuses(mcp);
        assert_eq!(footer.mcp_statuses.len(), 1);
    }
}
