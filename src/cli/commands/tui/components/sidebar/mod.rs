//! Sidebar Component
//!
//! A collapsible sidebar component that displays session information including:
//! - Context info (tokens, percentage, cost)
//! - MCP servers status
//! - LSP servers status
//! - Todo items
//! - File diffs
//!
//! # Features
//!
//! - Collapsible sections with expand/collapse indicators
//! - Clickable section headers to toggle visibility
//! - Color-coded status indicators
//! - Integration with theme system

pub mod context;
pub mod sections;

pub use context::ContextInfo;
pub use sections::{DiffInfo, LspServerInfo, McpServerInfo, SidebarSection, TodoItem};

use super::{Component, ComponentId, EventPropagation};
use crate::cli::commands::tui::theme::ThemeContext;
use crate::mcp::types::McpStatus;
use crate::VERSION;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use sections::SectionState;
use std::collections::HashMap;
use std::time::Duration;

/// Sidebar component displaying session information
///
/// Shows various sections of information in a collapsible sidebar
/// that can be toggled with keyboard shortcuts.
///
/// # Examples
///
/// ```rust,ignore
/// let sidebar = Sidebar::new(theme);
/// // Render when expanded
/// if sidebar.is_expanded() {
///     sidebar.render(frame, area);
/// }
/// ```
pub struct Sidebar {
    /// Unique component identifier
    id: ComponentId,
    /// Whether the sidebar is expanded
    expanded: bool,
    /// Sidebar width in characters
    width: u16,
    /// Currently active/focused section
    active_section: SidebarSection,
    /// Context information (tokens, cost, etc.)
    context: ContextInfo,
    /// List of MCP servers
    mcp_servers: Vec<McpServerInfo>,
    /// List of LSP servers
    lsp_servers: Vec<LspServerInfo>,
    /// List of todo items
    todos: Vec<TodoItem>,
    /// List of file diffs
    diffs: Vec<DiffInfo>,
    /// State of each section (expanded/collapsed)
    section_states: HashMap<SidebarSection, SectionState>,
    /// Theme context for styling
    theme: ThemeContext,
    /// Session title to display at the top
    session_title: String,
    /// Whether LSP is enabled (affects empty state message)
    lsp_enabled: bool,
}

impl Sidebar {
    /// Create a new sidebar component
    ///
    /// # Arguments
    ///
    /// * `theme` - Theme context for styling
    ///
    /// # Returns
    ///
    /// A new `Sidebar` instance with default settings.
    pub fn new(theme: ThemeContext) -> Self {
        let mut section_states = HashMap::new();
        for section in SidebarSection::all() {
            // Default: all sections expanded
            section_states.insert(section, SectionState::expanded(0));
        }

        Self {
            id: ComponentId::new(),
            expanded: false,
            width: 40,
            active_section: SidebarSection::Context,
            context: ContextInfo::default(),
            mcp_servers: Vec::new(),
            lsp_servers: Vec::new(),
            todos: Vec::new(),
            diffs: Vec::new(),
            section_states,
            theme,
            session_title: "New Session".to_string(),
            lsp_enabled: true,
        }
    }

    /// Set the sidebar width
    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    /// Get the sidebar width
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Check if the sidebar is expanded
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Toggle sidebar visibility
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Expand the sidebar
    pub fn expand(&mut self) {
        self.expanded = true;
    }

    /// Collapse the sidebar
    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    /// Set context information
    pub fn set_context(&mut self, context: ContextInfo) {
        self.context = context;
    }

    /// Get mutable reference to context
    pub fn context_mut(&mut self) -> &mut ContextInfo {
        &mut self.context
    }

    /// Set MCP servers list
    pub fn set_mcp_servers(&mut self, servers: Vec<McpServerInfo>) {
        self.mcp_servers = servers;
        if let Some(state) = self.section_states.get_mut(&SidebarSection::Mcp) {
            state.has_content = !self.mcp_servers.is_empty();
        }
    }

    /// Set LSP servers list
    pub fn set_lsp_servers(&mut self, servers: Vec<LspServerInfo>) {
        self.lsp_servers = servers;
        if let Some(state) = self.section_states.get_mut(&SidebarSection::Lsp) {
            state.has_content = !self.lsp_servers.is_empty();
        }
    }

    /// Set todo items list
    pub fn set_todos(&mut self, todos: Vec<TodoItem>) {
        self.todos = todos;
        if let Some(state) = self.section_states.get_mut(&SidebarSection::Todo) {
            state.has_content = !self.todos.is_empty();
        }
    }

    /// Set file diffs list
    pub fn set_diffs(&mut self, diffs: Vec<DiffInfo>) {
        self.diffs = diffs;
        if let Some(state) = self.section_states.get_mut(&SidebarSection::Diff) {
            state.has_content = !self.diffs.is_empty();
        }
    }

    /// Set the session title
    pub fn set_session_title(&mut self, title: String) {
        self.session_title = title;
    }

    /// Set whether LSP is enabled (affects empty state message)
    pub fn set_lsp_enabled(&mut self, enabled: bool) {
        self.lsp_enabled = enabled;
    }

    /// Toggle a section's expanded state
    pub fn toggle_section(&mut self, section: SidebarSection) {
        if let Some(state) = self.section_states.get_mut(&section) {
            state.toggle();
        }
    }

    /// Get the section state
    pub fn get_section_state(&self, section: SidebarSection) -> Option<&SectionState> {
        self.section_states.get(&section)
    }

    /// Render the context section
    fn render_context_section(&self, block: Block, area: Rect, frame: &mut Frame) {
        let mut lines = vec![Line::from(vec![Span::styled(
            self.session_title.clone(),
            Style::default()
                .fg(self.theme.text())
                .add_modifier(Modifier::BOLD),
        )])];

        lines.push(Line::from(vec![
            Span::styled("Tokens: ", Style::default().fg(self.theme.text_muted())),
            Span::styled(
                self.context.format_tokens(),
                Style::default().fg(self.theme.text()),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Usage:  ", Style::default().fg(self.theme.text_muted())),
            Span::styled(
                self.context.format_percentage(),
                Style::default().fg(self.theme.text()),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Cost:   ", Style::default().fg(self.theme.text_muted())),
            Span::styled(
                self.context.format_cost(),
                Style::default().fg(self.theme.success()),
            ),
        ]));

        let paragraph = Paragraph::new(lines).block(block);
        frame.render_widget(paragraph, area);
    }

    /// Count MCP servers by status for summary display
    fn count_mcp_statuses(&self) -> (usize, usize) {
        let active = self
            .mcp_servers
            .iter()
            .filter(|s| matches!(s.status, McpStatus::Connected))
            .count();
        let errors = self
            .mcp_servers
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    McpStatus::Failed { .. }
                        | McpStatus::NeedsAuth
                        | McpStatus::NeedsClientRegistration { .. }
                )
            })
            .count();
        (active, errors)
    }

    /// Render the footer with version info
    fn render_footer(&self, area: Rect, frame: &mut Frame) {
        let footer_text = Line::from(vec![
            Span::styled("•", Style::default().fg(self.theme.success())),
            Span::raw(" "),
            Span::styled("Open", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                "Code",
                Style::default()
                    .fg(self.theme.text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(VERSION, Style::default().fg(self.theme.text_muted())),
        ]);

        let paragraph = Paragraph::new(footer_text);
        frame.render_widget(paragraph, area);
    }

    /// Render a collapsible section with items
    fn render_section<T, F>(
        &self,
        section: SidebarSection,
        items: &[T],
        mut render_item: F,
        _block: Block,
        area: Rect,
        frame: &mut Frame,
    ) where
        F: FnMut(&T, usize) -> Line<'static>,
    {
        let state = self.section_states.get(&section).unwrap();

        // Build header with potential summary
        let mut header_spans = if state.has_content {
            vec![
                Span::raw(state.indicator()),
                Span::raw(" "),
                Span::raw(section.title()),
            ]
        } else {
            vec![Span::raw("  "), Span::raw(section.title())]
        };

        // Add MCP summary when collapsed and has >2 items
        if section == SidebarSection::Mcp && !state.expanded && items.len() > 2 {
            let (active, errors) = self.count_mcp_statuses();
            if active > 0 || errors > 0 {
                let mut summary = format!(" ({active} active");
                if errors > 0 {
                    summary.push_str(&format!(
                        ", {errors} error{}",
                        if errors > 1 { "s" } else { "" }
                    ));
                }
                summary.push(')');
                header_spans.push(Span::styled(
                    summary,
                    Style::default().fg(self.theme.text_muted()),
                ));
            }
        }

        let header_style = if section == self.active_section {
            Style::default()
                .fg(self.theme.primary())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text())
        };

        // Apply header style to all spans
        let header_line = Line::from(
            header_spans
                .into_iter()
                .map(|s| Span::styled(s.content, header_style))
                .collect::<Vec<_>>(),
        );

        let header_block = Block::default()
            .title(header_line)
            .borders(Borders::TOP)
            .border_style(Style::default().fg(self.theme.border()));

        // Calculate header height
        let header_height = 1u16;

        // Render header
        let header_area = Rect::new(area.x, area.y, area.width, header_height);
        frame.render_widget(header_block.clone(), header_area);

        // Render content if expanded
        if state.expanded {
            if items.is_empty() {
                // Handle empty state for LSP section
                if section == SidebarSection::Lsp {
                    let empty_message = if !self.lsp_enabled {
                        "LSPs have been disabled in settings"
                    } else {
                        "LSPs will activate as files are read"
                    };
                    let content_area = Rect::new(
                        area.x,
                        area.y + header_height,
                        area.width,
                        area.height.saturating_sub(header_height),
                    );
                    let content = vec![Line::from(Span::styled(
                        empty_message,
                        Style::default().fg(self.theme.text_muted()),
                    ))];
                    let content_block = Block::default().padding(ratatui::widgets::Padding {
                        top: 0,
                        right: 1,
                        bottom: 0,
                        left: 2,
                    });
                    let paragraph = Paragraph::new(content).block(content_block);
                    frame.render_widget(paragraph, content_area);
                }
            } else {
                let content_area = Rect::new(
                    area.x,
                    area.y + header_height,
                    area.width,
                    area.height.saturating_sub(header_height),
                );

                let content: Vec<Line> = items
                    .iter()
                    .enumerate()
                    .map(|(idx, item)| render_item(item, idx))
                    .collect();

                let content_block = Block::default().padding(ratatui::widgets::Padding {
                    top: 0,
                    right: 1,
                    bottom: 0,
                    left: 2,
                });

                let paragraph = Paragraph::new(content).block(content_block);
                frame.render_widget(paragraph, content_area);
            }
        }
    }
}

impl Component for Sidebar {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.expanded {
            return;
        }

        let block = Block::default()
            .title("Session Info")
            .title_style(Style::default().fg(self.theme.primary()))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme.border()));

        // Render context section
        let context_area = Rect::new(area.x, area.y, area.width, 5.min(area.height));
        self.render_context_section(block.clone(), context_area, frame);

        // Calculate vertical position for subsequent sections
        let mut current_y = area.y + 5;
        let section_height = 4u16;

        // Render MCP section
        if current_y + section_height <= area.y + area.height {
            let mcp_servers = self.mcp_servers.clone();
            let mcp_area = Rect::new(area.x, current_y, area.width, section_height);
            let text_muted = self.theme.text_muted();
            self.render_section(
                SidebarSection::Mcp,
                &mcp_servers,
                move |server, _idx| {
                    Line::from(vec![
                        Span::styled(
                            server.status_indicator(),
                            Style::default().fg(server.status_color()),
                        ),
                        Span::raw(" "),
                        Span::raw(server.name.clone()),
                        Span::styled(server.format_tools(), Style::default().fg(text_muted)),
                    ])
                },
                block.clone(),
                mcp_area,
                frame,
            );
            current_y += section_height;
        }

        // Render LSP section
        if current_y + section_height <= area.y + area.height {
            let lsp_servers = self.lsp_servers.clone();
            let lsp_area = Rect::new(area.x, current_y, area.width, section_height);
            let text_muted = self.theme.text_muted();
            self.render_section(
                SidebarSection::Lsp,
                &lsp_servers,
                move |server, _idx| {
                    Line::from(vec![
                        Span::styled(
                            server.status_indicator(),
                            Style::default().fg(server.status_color()),
                        ),
                        Span::raw(" "),
                        Span::raw(server.name.clone()),
                        Span::styled(server.format_languages(), Style::default().fg(text_muted)),
                    ])
                },
                block.clone(),
                lsp_area,
                frame,
            );
            current_y += section_height;
        }

        // Render Todo section
        if current_y + section_height <= area.y + area.height {
            let todos = self.todos.clone();
            let todo_area = Rect::new(area.x, current_y, area.width, section_height);
            self.render_section(
                SidebarSection::Todo,
                &todos,
                move |todo, _idx| {
                    Line::from(vec![
                        Span::styled(todo.indicator(), Style::default().fg(todo.status_color())),
                        Span::raw(" "),
                        Span::raw(todo.content.clone()),
                    ])
                },
                block.clone(),
                todo_area,
                frame,
            );
            current_y += section_height;
        }

        // Render Diff section
        if current_y + section_height <= area.y + area.height {
            let diffs = self.diffs.clone();
            let diff_area = Rect::new(area.x, current_y, area.width, section_height);
            self.render_section(
                SidebarSection::Diff,
                &diffs,
                move |diff, _idx| {
                    Line::from(vec![
                        Span::raw(diff.path.clone()),
                        Span::raw(" "),
                        Span::styled(
                            format!("+{}", diff.additions),
                            Style::default().fg(DiffInfo::additions_color()),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            format!("-{}", diff.deletions),
                            Style::default().fg(DiffInfo::deletions_color()),
                        ),
                    ])
                },
                block,
                diff_area,
                frame,
            );
            current_y += section_height;
        }

        // Render footer with version at the bottom
        if current_y < area.y + area.height {
            let footer_height = 1u16;
            let footer_area = Rect::new(
                area.x,
                area.y + area.height - footer_height,
                area.width,
                footer_height,
            );
            self.render_footer(footer_area, frame);
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        if !self.expanded {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            // Toggle current section
            (KeyCode::Enter, _) => {
                self.toggle_section(self.active_section);
                EventPropagation::Stop
            }
            // Navigate sections
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.navigate_section(-1);
                EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.navigate_section(1);
                EventPropagation::Stop
            }
            // Close sidebar
            (KeyCode::Esc, _) => {
                self.collapse();
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {
        // Placeholder for future updates (e.g., polling)
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn focused(&self) -> bool {
        false // Sidebar is not focusable in the traditional sense
    }

    fn on_focus(&mut self) {
        // No-op
    }

    fn on_blur(&mut self) {
        // No-op
    }
}

impl Sidebar {
    /// Navigate to previous or next section
    fn navigate_section(&mut self, direction: i8) {
        let sections = SidebarSection::all();
        let current_idx = sections
            .iter()
            .position(|&s| s == self.active_section)
            .unwrap_or(0);

        let new_idx = if direction < 0 {
            if current_idx == 0 {
                sections.len() - 1
            } else {
                current_idx - 1
            }
        } else {
            (current_idx + 1) % sections.len()
        };

        self.active_section = sections[new_idx];
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new(ThemeContext::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::types::McpStatus;

    #[test]
    fn test_sidebar_new() {
        let sidebar = Sidebar::new(ThemeContext::default());
        assert!(!sidebar.is_expanded());
        assert_eq!(sidebar.width(), 40);
    }

    #[test]
    fn test_sidebar_toggle() {
        let mut sidebar = Sidebar::default();
        assert!(!sidebar.is_expanded());

        sidebar.toggle();
        assert!(sidebar.is_expanded());

        sidebar.toggle();
        assert!(!sidebar.is_expanded());
    }

    #[test]
    fn test_sidebar_expand_collapse() {
        let mut sidebar = Sidebar::default();
        sidebar.expand();
        assert!(sidebar.is_expanded());

        sidebar.collapse();
        assert!(!sidebar.is_expanded());
    }

    #[test]
    fn test_sidebar_set_context() {
        let mut sidebar = Sidebar::default();
        let context = ContextInfo::new(1000, Some(50), 0.001);
        sidebar.set_context(context.clone());
        assert_eq!(sidebar.context.tokens, context.tokens);
    }

    #[test]
    fn test_sidebar_section_toggle() {
        let mut sidebar = Sidebar::default();
        let section = SidebarSection::Mcp;

        // Initially expanded
        assert!(sidebar.get_section_state(section).unwrap().expanded);

        sidebar.toggle_section(section);
        assert!(!sidebar.get_section_state(section).unwrap().expanded);

        sidebar.toggle_section(section);
        assert!(sidebar.get_section_state(section).unwrap().expanded);
    }

    #[test]
    fn test_sidebar_navigate_section() {
        let mut sidebar = Sidebar::default();
        assert_eq!(sidebar.active_section, SidebarSection::Context);

        sidebar.navigate_section(1);
        assert_eq!(sidebar.active_section, SidebarSection::Mcp);

        sidebar.navigate_section(-1);
        assert_eq!(sidebar.active_section, SidebarSection::Context);
    }

    #[test]
    fn test_sidebar_set_mcp_servers() {
        let mut sidebar = Sidebar::default();
        let servers = vec![McpServerInfo::new(
            "test".to_string(),
            McpStatus::Connected,
            5,
        )];
        sidebar.set_mcp_servers(servers);
        assert_eq!(sidebar.mcp_servers.len(), 1);
    }

    #[test]
    fn test_sidebar_default() {
        let sidebar = Sidebar::default();
        assert!(!sidebar.is_expanded());
        assert_eq!(sidebar.width(), 40);
        assert!(sidebar.mcp_servers.is_empty());
        assert!(sidebar.lsp_servers.is_empty());
        assert!(sidebar.todos.is_empty());
        assert!(sidebar.diffs.is_empty());
    }

    #[test]
    fn test_sidebar_set_session_title() {
        let mut sidebar = Sidebar::default();
        assert_eq!(sidebar.session_title, "New Session");

        sidebar.set_session_title("My Test Session".to_string());
        assert_eq!(sidebar.session_title, "My Test Session");
    }

    #[test]
    fn test_sidebar_set_lsp_enabled() {
        let mut sidebar = Sidebar::default();
        assert!(sidebar.lsp_enabled);

        sidebar.set_lsp_enabled(false);
        assert!(!sidebar.lsp_enabled);

        sidebar.set_lsp_enabled(true);
        assert!(sidebar.lsp_enabled);
    }

    #[test]
    fn test_sidebar_count_mcp_statuses() {
        let mut sidebar = Sidebar::default();

        // No servers
        let (active, errors) = sidebar.count_mcp_statuses();
        assert_eq!(active, 0);
        assert_eq!(errors, 0);

        // Add various servers
        let servers = vec![
            McpServerInfo::new("server1".to_string(), McpStatus::Connected, 5),
            McpServerInfo::new("server2".to_string(), McpStatus::Connected, 3),
            McpServerInfo::new(
                "server3".to_string(),
                McpStatus::Failed {
                    error: "test".to_string(),
                },
                0,
            ),
            McpServerInfo::new("server4".to_string(), McpStatus::NeedsAuth, 0),
            McpServerInfo::new("server5".to_string(), McpStatus::Disabled, 0),
        ];
        sidebar.set_mcp_servers(servers);

        let (active, errors) = sidebar.count_mcp_statuses();
        assert_eq!(active, 2);
        assert_eq!(errors, 2); // Failed and NeedsAuth
    }

    #[test]
    fn test_sidebar_count_mcp_statuses_with_client_registration_error() {
        let mut sidebar = Sidebar::default();

        let servers = vec![McpServerInfo::new(
            "server1".to_string(),
            McpStatus::NeedsClientRegistration {
                error: "auth required".to_string(),
            },
            0,
        )];
        sidebar.set_mcp_servers(servers);

        let (active, errors) = sidebar.count_mcp_statuses();
        assert_eq!(active, 0);
        assert_eq!(errors, 1);
    }
}
