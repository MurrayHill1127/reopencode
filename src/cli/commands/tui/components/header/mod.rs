//! Header Component
//!
//! Displays session header with title, workspace info, and context statistics.
//! Supports both normal sessions and subagent sessions with navigation hints.
//!
//! # Normal Session
//! - Title with `#` prefix
//! - Workspace info (local or with type)
//! - Context info: tokens, percentage, cost
//!
//! # Subagent Session
//! - "Subagent session" label
//! - Workspace info
//! - Context info
//! - Navigation buttons: Parent, Prev, Next
//!
//! # Narrow Mode
//! When width < 80 characters, layouts stack vertically.

use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Duration;

use super::{Component, ComponentId, ContextInfo, EventPropagation};
use crate::cli::commands::tui::theme::ThemeContext;

/// Minimum width for horizontal layout
const NARROW_THRESHOLD: u16 = 80;

/// Hover state for navigation buttons (subagent mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverButton {
    #[default]
    None,
    Parent,
    Prev,
    Next,
}

/// Header component for TUI
///
/// Displays session title, workspace information, and context statistics.
/// Supports two modes: normal session and subagent session.
pub struct Header {
    /// Unique component identifier
    id: ComponentId,
    /// Theme context for styling
    theme: ThemeContext,
    /// Session title to display
    session_title: String,
    /// Context information (tokens, percentage, cost)
    context: ContextInfo,
    /// Workspace ID (None = local)
    workspace_id: Option<String>,
    /// Workspace type (e.g., "shared", "remote")
    workspace_type: Option<String>,
    /// Parent session ID (Some = subagent session)
    parent_id: Option<String>,
    /// Current hover state for navigation buttons
    hover: HoverButton,
    /// Width for narrow mode detection
    width: u16,
}

impl Header {
    /// Create a new Header component
    ///
    /// # Arguments
    ///
    /// * `theme` - Theme context for styling
    ///
    /// # Returns
    ///
    /// A new `Header` instance with default values.
    pub fn new(theme: ThemeContext) -> Self {
        Self {
            id: ComponentId::new(),
            theme,
            session_title: String::new(),
            context: ContextInfo::default(),
            workspace_id: None,
            workspace_type: None,
            parent_id: None,
            hover: HoverButton::None,
            width: 80,
        }
    }

    /// Set the session title
    pub fn set_session_title(&mut self, title: String) {
        self.session_title = title;
    }

    /// Get the session title
    pub fn session_title(&self) -> &str {
        &self.session_title
    }

    /// Set context information
    pub fn set_context(&mut self, context: ContextInfo) {
        self.context = context;
    }

    /// Get context information
    pub fn context(&self) -> &ContextInfo {
        &self.context
    }

    /// Set workspace information
    ///
    /// # Arguments
    ///
    /// * `id` - Workspace ID (None for local)
    /// * `workspace_type` - Workspace type (e.g., "shared")
    pub fn set_workspace(&mut self, id: Option<String>, workspace_type: Option<String>) {
        self.workspace_id = id;
        self.workspace_type = workspace_type;
    }

    /// Set parent session ID (enables subagent mode)
    pub fn set_parent_id(&mut self, id: Option<String>) {
        self.parent_id = id;
    }

    /// Get parent session ID
    pub fn parent_id(&self) -> Option<&str> {
        self.parent_id.as_deref()
    }

    /// Set the component width (for narrow mode detection)
    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    /// Set hover state for navigation buttons
    pub fn set_hover(&mut self, hover: HoverButton) {
        self.hover = hover;
    }

    /// Check if in narrow mode
    fn is_narrow(&self) -> bool {
        self.width < NARROW_THRESHOLD
    }

    /// Check if this is a subagent session
    fn is_subagent(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Format workspace string for display
    fn format_workspace(&self) -> String {
        match (&self.workspace_id, &self.workspace_type) {
            (None, _) => "Workspace local".to_string(),
            (Some(id), None) => format!("Workspace {}", id),
            (Some(id), Some(ty)) => format!("Workspace {} ({})", id, ty),
        }
    }

    /// Format context string: "tokens percentage%"
    fn format_context(&self) -> String {
        let tokens = self.context.format_tokens();
        let pct = self.context.format_percentage();
        format!("{} {}", tokens, pct)
    }

    /// Render normal session header
    fn render_normal(&self, frame: &mut Frame, area: Rect) {
        let narrow = self.is_narrow();
        let direction = if narrow {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };

        let chunks = Layout::default()
            .direction(direction)
            .constraints(if narrow {
                vec![Constraint::Length(2), Constraint::Length(1)]
            } else {
                vec![Constraint::Min(20), Constraint::Length(30)]
            })
            .split(area);

        // Left side: Title + Workspace
        let left_lines = if narrow {
            vec![
                Line::from(vec![
                    Span::styled("# ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        self.session_title.clone(),
                        Style::default()
                            .fg(self.theme.text())
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    self.format_workspace(),
                    Style::default().fg(self.theme.text_muted()),
                )),
            ]
        } else {
            vec![Line::from(vec![
                Span::styled("# ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    self.session_title.clone(),
                    Style::default()
                        .fg(self.theme.text())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    self.format_workspace(),
                    Style::default().fg(self.theme.text_muted()),
                ),
            ])]
        };

        let left_para = Paragraph::new(left_lines);
        frame.render_widget(left_para, chunks[0]);

        // Right side: Context info
        if !narrow {
            let right_line = Line::from(vec![
                Span::styled(
                    self.format_context(),
                    Style::default().fg(self.theme.text_muted()),
                ),
                Span::raw("  "),
                Span::styled(
                    self.context.format_cost(),
                    Style::default().fg(self.theme.success()),
                ),
            ]);
            let right_para = Paragraph::new(right_line);
            frame.render_widget(right_para, chunks[1]);
        } else {
            // Narrow mode: context on second line
            let context_line = Line::from(vec![
                Span::styled(
                    self.format_context(),
                    Style::default().fg(self.theme.text_muted()),
                ),
                Span::raw("  "),
                Span::styled(
                    self.context.format_cost(),
                    Style::default().fg(self.theme.success()),
                ),
            ]);
            // Overwrite the second chunk with context
            let right_para = Paragraph::new(context_line);
            frame.render_widget(right_para, chunks[1]);
        }
    }

    /// Render subagent session header
    fn render_subagent(&self, frame: &mut Frame, area: Rect) {
        let narrow = self.is_narrow();

        // Main layout: content on top, buttons on bottom
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(1)])
            .split(area);

        // Content area
        let content_direction = if narrow {
            Direction::Vertical
        } else {
            Direction::Horizontal
        };

        let content_chunks = Layout::default()
            .direction(content_direction)
            .constraints(if narrow {
                vec![Constraint::Length(1), Constraint::Length(1)]
            } else {
                vec![Constraint::Min(20), Constraint::Length(30)]
            })
            .split(main_chunks[0]);

        // Left: "Subagent session" + workspace
        let left_lines = if narrow {
            vec![
                Line::from(Span::styled(
                    "Subagent session",
                    Style::default()
                        .fg(self.theme.text())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    self.format_workspace(),
                    Style::default().fg(self.theme.text_muted()),
                )),
            ]
        } else {
            vec![Line::from(vec![
                Span::styled(
                    "Subagent session",
                    Style::default()
                        .fg(self.theme.text())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    self.format_workspace(),
                    Style::default().fg(self.theme.text_muted()),
                ),
            ])]
        };

        let left_para = Paragraph::new(left_lines);
        frame.render_widget(left_para, content_chunks[0]);

        // Right: Context info
        let right_line = Line::from(vec![
            Span::styled(
                self.format_context(),
                Style::default().fg(self.theme.text_muted()),
            ),
            Span::raw("  "),
            Span::styled(
                self.context.format_cost(),
                Style::default().fg(self.theme.success()),
            ),
        ]);
        let right_para = Paragraph::new(right_line);
        frame.render_widget(right_para, content_chunks[1]);

        // Bottom: Navigation buttons
        let button_area = main_chunks[1];
        let buttons = self.render_buttons();
        let buttons_para = Paragraph::new(buttons);
        frame.render_widget(buttons_para, button_area);
    }

    /// Render navigation buttons for subagent mode
    fn render_buttons(&self) -> Vec<Line<'static>> {
        let mut spans = Vec::new();

        // Parent button
        let parent_bg = if self.hover == HoverButton::Parent {
            self.theme.secondary()
        } else {
            self.theme.background()
        };
        spans.push(Span::styled(
            "Parent",
            Style::default().fg(self.theme.text()).bg(parent_bg),
        ));
        spans.push(Span::raw(" "));

        // Prev button
        let prev_bg = if self.hover == HoverButton::Prev {
            self.theme.secondary()
        } else {
            self.theme.background()
        };
        spans.push(Span::styled(
            "Prev",
            Style::default().fg(self.theme.text()).bg(prev_bg),
        ));
        spans.push(Span::raw(" "));

        // Next button
        let next_bg = if self.hover == HoverButton::Next {
            self.theme.secondary()
        } else {
            self.theme.background()
        };
        spans.push(Span::styled(
            "Next",
            Style::default().fg(self.theme.text()).bg(next_bg),
        ));

        vec![Line::from(spans)]
    }
}

impl Component for Header {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if self.is_subagent() {
            self.render_subagent(frame, area);
        } else {
            self.render_normal(frame, area);
        }
    }

    fn update(&mut self, _delta: Duration) {
        // No timer-based updates needed
    }

    fn handle_input(&mut self, _event: KeyEvent) -> EventPropagation {
        EventPropagation::Continue
    }

    fn is_focusable(&self) -> bool {
        false
    }
}

impl Default for Header {
    fn default() -> Self {
        Self::new(ThemeContext::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new() {
        let header = Header::new(ThemeContext::default());
        assert!(!header.is_subagent());
        assert!(!header.is_narrow());
        assert!(header.session_title.is_empty());
    }

    #[test]
    fn test_header_default() {
        let header = Header::default();
        assert!(!header.is_subagent());
    }

    #[test]
    fn test_header_set_session_title() {
        let mut header = Header::default();
        header.set_session_title("Test Session".to_string());
        assert_eq!(header.session_title(), "Test Session");
    }

    #[test]
    fn test_header_set_context() {
        let mut header = Header::default();
        let context = ContextInfo::new(1500, Some(45), 0.002);
        header.set_context(context);
        assert_eq!(header.context().tokens, 1500);
    }

    #[test]
    fn test_header_set_workspace() {
        let mut header = Header::default();

        // Local workspace
        assert_eq!(header.format_workspace(), "Workspace local");

        // Workspace with ID only
        header.set_workspace(Some("ws-1".to_string()), None);
        assert_eq!(header.format_workspace(), "Workspace ws-1");

        // Workspace with ID and type
        header.set_workspace(Some("ws-1".to_string()), Some("shared".to_string()));
        assert_eq!(header.format_workspace(), "Workspace ws-1 (shared)");
    }

    #[test]
    fn test_header_set_parent_id() {
        let mut header = Header::default();
        assert!(!header.is_subagent());

        header.set_parent_id(Some("parent-123".to_string()));
        assert!(header.is_subagent());
        assert_eq!(header.parent_id(), Some("parent-123"));

        header.set_parent_id(None);
        assert!(!header.is_subagent());
    }

    #[test]
    fn test_header_narrow_mode() {
        let mut header = Header::default();
        assert!(!header.is_narrow());

        header.set_width(60);
        assert!(header.is_narrow());

        header.set_width(100);
        assert!(!header.is_narrow());
    }

    #[test]
    fn test_header_hover_state() {
        let mut header = Header::default();
        assert_eq!(header.hover, HoverButton::None);

        header.set_hover(HoverButton::Parent);
        assert_eq!(header.hover, HoverButton::Parent);

        header.set_hover(HoverButton::Next);
        assert_eq!(header.hover, HoverButton::Next);
    }

    #[test]
    fn test_header_format_context() {
        let mut header = Header::default();
        let context = ContextInfo::new(1500, Some(45), 0.002);
        header.set_context(context);
        assert_eq!(header.format_context(), "1.5K 45%");
    }

    #[test]
    fn test_header_is_focusable() {
        let header = Header::default();
        assert!(!header.is_focusable());
    }

    #[test]
    fn test_hover_button_default() {
        let hover = HoverButton::default();
        assert_eq!(hover, HoverButton::None);
    }
}
