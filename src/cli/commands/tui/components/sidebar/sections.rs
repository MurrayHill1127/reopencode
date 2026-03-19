//! Sidebar Sections
//!
//! This module provides data structures and rendering for collapsible
//! sidebar sections including MCP servers, LSP servers, todos, and diffs.

use crate::mcp::types::McpStatus;
use crate::storage::schema::TodoStatus;
use ratatui::style::Color;

/// Sidebar section types
///
/// Each section can be expanded or collapsed independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarSection {
    /// Context information section (tokens, cost, percentage)
    Context,
    /// MCP servers section
    Mcp,
    /// LSP servers section
    Lsp,
    /// Todo items section
    Todo,
    /// File diffs section
    Diff,
}

impl SidebarSection {
    /// Get the display title for this section
    pub fn title(&self) -> &'static str {
        match self {
            Self::Context => "Context",
            Self::Mcp => "MCP Servers",
            Self::Lsp => "LSP Servers",
            Self::Todo => "Todos",
            Self::Diff => "File Diffs",
        }
    }

    /// Get all sections in display order
    pub fn all() -> Vec<Self> {
        vec![Self::Context, Self::Mcp, Self::Lsp, Self::Todo, Self::Diff]
    }
}

/// State for a collapsible sidebar section
#[derive(Debug, Clone)]
pub struct SectionState {
    /// Whether the section is expanded
    pub expanded: bool,
    /// Whether the section has content to show
    pub has_content: bool,
    /// Number of items in the section
    pub item_count: usize,
}

impl SectionState {
    /// Create a new section state
    ///
    /// # Arguments
    ///
    /// * `expanded` - Initial expanded state
    /// * `has_content` - Whether the section has content
    /// * `item_count` - Number of items in the section
    ///
    /// # Returns
    ///
    /// A new `SectionState` instance.
    pub fn new(expanded: bool, has_content: bool, item_count: usize) -> Self {
        Self {
            expanded,
            has_content,
            item_count,
        }
    }

    /// Create a collapsed section with content
    pub fn collapsed(item_count: usize) -> Self {
        Self {
            expanded: false,
            has_content: true,
            item_count,
        }
    }

    /// Create an expanded section with content
    pub fn expanded(item_count: usize) -> Self {
        Self {
            expanded: true,
            has_content: true,
            item_count,
        }
    }

    /// Toggle expanded state
    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Check if the section can be toggled
    ///
    /// # Returns
    ///
    /// `true` if the section has more than 2 items and can be toggled.
    pub fn can_toggle(&self) -> bool {
        self.item_count > 2
    }

    /// Get the expand/collapse indicator
    ///
    /// # Returns
    ///
    /// "" (empty string) if 2 or fewer items.
    /// "▼" if expanded and more than 2 items.
    /// "▶" if collapsed and more than 2 items.
    pub fn indicator(&self) -> &'static str {
        if self.item_count <= 2 {
            ""
        } else if self.expanded {
            "▼"
        } else {
            "▶"
        }
    }
}

impl Default for SectionState {
    fn default() -> Self {
        Self::collapsed(0)
    }
}

/// MCP server information for display
#[derive(Debug, Clone)]
pub struct McpServerInfo {
    /// Server name
    pub name: String,
    /// Server status
    pub status: McpStatus,
    /// Number of tools available (if connected)
    pub tool_count: usize,
}

impl McpServerInfo {
    /// Create new MCP server info
    pub fn new(name: String, status: McpStatus, tool_count: usize) -> Self {
        Self {
            name,
            status,
            tool_count,
        }
    }

    /// Get status indicator symbol
    pub fn status_indicator(&self) -> &'static str {
        match &self.status {
            McpStatus::Connected => "●",
            McpStatus::Failed { .. } => "✗",
            McpStatus::Disabled => "○",
            McpStatus::NeedsAuth => "⚠",
            McpStatus::NeedsClientRegistration { .. } => "?",
        }
    }

    /// Get status color
    pub fn status_color(&self) -> Color {
        match &self.status {
            McpStatus::Connected => Color::Green,
            McpStatus::Failed { .. } => Color::Red,
            McpStatus::Disabled => Color::Gray,
            McpStatus::NeedsAuth => Color::Yellow,
            McpStatus::NeedsClientRegistration { .. } => Color::Magenta,
        }
    }

    /// Format tool count for display
    pub fn format_tools(&self) -> String {
        if self.tool_count > 0 {
            format!(" ({} tools)", self.tool_count)
        } else {
            String::new()
        }
    }
}

/// LSP server information for display
#[derive(Debug, Clone)]
pub struct LspServerInfo {
    /// Server name/ID
    pub name: String,
    /// Whether the server is active
    pub active: bool,
    /// Language(s) supported
    pub languages: Vec<String>,
}

impl LspServerInfo {
    /// Create new LSP server info
    pub fn new(name: String, active: bool, languages: Vec<String>) -> Self {
        Self {
            name,
            active,
            languages,
        }
    }

    /// Get status indicator
    pub fn status_indicator(&self) -> &'static str {
        if self.active {
            "●"
        } else {
            "○"
        }
    }

    /// Get status color
    pub fn status_color(&self) -> Color {
        if self.active {
            Color::Green
        } else {
            Color::Gray
        }
    }

    /// Format languages for display
    pub fn format_languages(&self) -> String {
        if self.languages.is_empty() {
            String::new()
        } else {
            format!(" ({})", self.languages.join(", "))
        }
    }
}

/// Todo item information
#[derive(Debug, Clone)]
pub struct TodoItem {
    /// Todo description
    pub content: String,
    /// Todo status
    pub status: TodoStatus,
}

impl TodoItem {
    /// Create new todo item
    pub fn new(content: String, status: TodoStatus) -> Self {
        Self { content, status }
    }

    /// Get status indicator
    pub fn indicator(&self) -> &'static str {
        match self.status {
            TodoStatus::Completed => "[✓]",
            TodoStatus::InProgress => "[•]",
            TodoStatus::Pending | TodoStatus::Cancelled => "[ ]",
        }
    }

    /// Get status color
    pub fn status_color(&self) -> Color {
        match self.status {
            TodoStatus::Completed => Color::Green,
            TodoStatus::InProgress => Color::Yellow,
            TodoStatus::Pending | TodoStatus::Cancelled => Color::Gray,
        }
    }
}

/// File diff information
#[derive(Debug, Clone)]
pub struct DiffInfo {
    /// File path
    pub path: String,
    /// Number of lines added
    pub additions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

impl DiffInfo {
    /// Create new diff info
    pub fn new(path: String, additions: usize, deletions: usize) -> Self {
        Self {
            path,
            additions,
            deletions,
        }
    }

    /// Format changes for display
    pub fn format_changes(&self) -> String {
        format!("+{} -{}", self.additions, self.deletions)
    }

    /// Get additions color
    pub fn additions_color() -> Color {
        Color::Green
    }

    /// Get deletions color
    pub fn deletions_color() -> Color {
        Color::Red
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidebar_section_title() {
        assert_eq!(SidebarSection::Context.title(), "Context");
        assert_eq!(SidebarSection::Mcp.title(), "MCP Servers");
        assert_eq!(SidebarSection::Lsp.title(), "LSP Servers");
        assert_eq!(SidebarSection::Todo.title(), "Todos");
        assert_eq!(SidebarSection::Diff.title(), "File Diffs");
    }

    #[test]
    fn test_sidebar_section_all() {
        let all = SidebarSection::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], SidebarSection::Context);
        assert_eq!(all[1], SidebarSection::Mcp);
    }

    #[test]
    fn test_section_state_toggle() {
        let mut state = SectionState::collapsed(3);
        assert!(!state.expanded);
        assert!(state.can_toggle());
        assert_eq!(state.indicator(), "▶");

        state.toggle();
        assert!(state.expanded);
        assert_eq!(state.indicator(), "▼");
    }

    #[test]
    fn test_section_state_default() {
        let state = SectionState::default();
        assert!(!state.expanded);
        assert!(state.has_content);
        assert_eq!(state.item_count, 0);
    }

    #[test]
    fn test_mcp_server_info() {
        let info = McpServerInfo::new("test-server".to_string(), McpStatus::Connected, 5);
        assert_eq!(info.status_indicator(), "●");
        assert_eq!(info.status_color(), Color::Green);
        assert_eq!(info.format_tools(), " (5 tools)");
    }

    #[test]
    fn test_lsp_server_info() {
        let info = LspServerInfo::new("rust-analyzer".to_string(), true, vec!["rust".to_string()]);
        assert_eq!(info.status_indicator(), "●");
        assert_eq!(info.status_color(), Color::Green);
        assert_eq!(info.format_languages(), " (rust)");
    }

    #[test]
    fn test_todo_item() {
        let todo = TodoItem::new("Fix bug".to_string(), TodoStatus::Pending);
        assert_eq!(todo.indicator(), "[ ]");
        assert_eq!(todo.status_color(), Color::Gray);

        let completed = TodoItem::new("Done task".to_string(), TodoStatus::Completed);
        assert_eq!(completed.indicator(), "[✓]");
        assert_eq!(completed.status_color(), Color::Green);
    }

    #[test]
    fn test_diff_info() {
        let diff = DiffInfo::new("src/main.rs".to_string(), 10, 3);
        assert_eq!(diff.format_changes(), "+10 -3");
        assert_eq!(DiffInfo::additions_color(), Color::Green);
        assert_eq!(DiffInfo::deletions_color(), Color::Red);
    }

    #[test]
    fn test_todo_item_in_progress() {
        let todo = TodoItem::new("Working on it".to_string(), TodoStatus::InProgress);
        assert_eq!(todo.indicator(), "[•]");
        assert_eq!(todo.status_color(), Color::Yellow);
    }

    #[test]
    fn test_section_indicator_two_items() {
        let collapsed = SectionState::collapsed(2);
        assert!(!collapsed.can_toggle());
        assert_eq!(collapsed.indicator(), "");

        let expanded = SectionState::expanded(2);
        assert!(!expanded.can_toggle());
        assert_eq!(expanded.indicator(), "");
    }

    #[test]
    fn test_section_indicator_three_items() {
        let collapsed = SectionState::collapsed(3);
        assert!(collapsed.can_toggle());
        assert_eq!(collapsed.indicator(), "▶");

        let expanded = SectionState::expanded(3);
        assert!(expanded.can_toggle());
        assert_eq!(expanded.indicator(), "▼");
    }
}
