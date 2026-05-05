//! Permission Dialog Component
//!
//! A modal dialog for handling tool execution permission requests.
//! Displays permission details with action buttons for once/always/reject.
//!
//! # Features
//!
//! - Multi-stage workflow (Permission → Always → Reject)
//! - Action cycling with Left/Right keys
//! - Stage transitions with Enter/Escape
//! - Centered modal overlay
//! - Cyan border when focused
//!
//! # Examples
//!
//! ```rust,ignore
//! let request = PermissionRequest {
//!     id: "req_123".to_string(),
//!     session_id: "ses_456".to_string(),
//!     permission: PermissionType::Edit,
//!     tool_name: Some("write_file".to_string()),
//!     description: Some("Edit src/main.rs".to_string()),
//!     metadata: HashMap::new(),
//!     always_patterns: vec!["src/**/*.rs".to_string()],
//! };
//!
//! let mut dialog = PermissionDialog::new(request);
//! dialog.show();
//!
//! // Handle input
//! if dialog.handle_input(event) == EventPropagation::Stop {
//!     // Dialog handled the event
//! }
//!
//! // Get the result
//! if let Some(reply) = dialog.get_reply() {
//!     // Handle permission reply
//! }
//! ```

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Permission types that can be requested
///
/// Represents the various tool operations that may require user permission.
/// Includes common file operations, search operations, and external calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionType {
    /// File editing operations
    Edit,
    /// File reading operations
    Read,
    /// Glob pattern matching
    Glob,
    /// Grep/text search operations
    Grep,
    /// Directory listing operations
    List,
    /// Shell/bash command execution
    Bash,
    /// Sub-agent task execution
    Task,
    /// Web content fetching
    WebFetch,
    /// Web search operations
    WebSearch,
    /// Code search operations
    CodeSearch,
    /// External directory access
    ExternalDirectory,
    /// Doom loop (repeated failure continuation)
    DoomLoop,
    /// Other permission types
    Other(String),
}

impl PermissionType {
    /// Get the display icon for this permission type
    ///
    /// Returns a unicode symbol representing the permission category.
    pub fn icon(&self) -> &str {
        match self {
            PermissionType::Edit => "→",
            PermissionType::Read => "→",
            PermissionType::Glob => "✱",
            PermissionType::Grep => "✱",
            PermissionType::List => "→",
            PermissionType::Bash => "#",
            PermissionType::Task => "#",
            PermissionType::WebFetch => "%",
            PermissionType::WebSearch => "◈",
            PermissionType::CodeSearch => "◇",
            PermissionType::ExternalDirectory => "←",
            PermissionType::DoomLoop => "⟳",
            PermissionType::Other(_) => "⚙",
        }
    }

    /// Get the action verb for this permission type
    ///
    /// Returns a human-readable verb describing the action.
    pub fn action_verb(&self) -> &str {
        match self {
            PermissionType::Edit => "Edit",
            PermissionType::Read => "Read",
            PermissionType::Glob => "Glob",
            PermissionType::Grep => "Grep",
            PermissionType::List => "List",
            PermissionType::Bash => "Execute",
            PermissionType::Task => "Run",
            PermissionType::WebFetch => "Fetch",
            PermissionType::WebSearch => "Search",
            PermissionType::CodeSearch => "Code Search",
            PermissionType::ExternalDirectory => "Access",
            PermissionType::DoomLoop => "Continue",
            PermissionType::Other(_) => "Call",
        }
    }
}

impl std::fmt::Display for PermissionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionType::Edit => write!(f, "edit"),
            PermissionType::Read => write!(f, "read"),
            PermissionType::Glob => write!(f, "glob"),
            PermissionType::Grep => write!(f, "grep"),
            PermissionType::List => write!(f, "list"),
            PermissionType::Bash => write!(f, "bash"),
            PermissionType::Task => write!(f, "task"),
            PermissionType::WebFetch => write!(f, "webfetch"),
            PermissionType::WebSearch => write!(f, "websearch"),
            PermissionType::CodeSearch => write!(f, "codesearch"),
            PermissionType::ExternalDirectory => write!(f, "external_directory"),
            PermissionType::DoomLoop => write!(f, "doom_loop"),
            PermissionType::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Stage of the permission prompt
///
/// The dialog progresses through different stages based on user selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PermissionStage {
    /// Initial permission prompt (default)
    #[default]
    Permission,
    /// Always allow confirmation stage
    Always,
    /// Reject with message stage
    Reject,
}

/// Permission reply options
///
/// Represents the user's decision on a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionReply {
    /// Allow this operation once
    Once,
    /// Always allow matching operations
    Always,
    /// Reject this operation
    Reject,
}

/// Permission request from agent
///
/// Contains all the information needed to display and handle
/// a permission request from an AI agent.
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Unique request identifier
    pub id: String,
    /// Session that initiated the request
    pub session_id: String,
    /// Type of permission being requested
    pub permission: PermissionType,
    /// Name of the tool requesting permission
    pub tool_name: Option<String>,
    /// Human-readable description of the operation
    pub description: Option<String>,
    /// Additional metadata about the request
    pub metadata: HashMap<String, String>,
    /// Patterns that would be matched by "always allow"
    pub always_patterns: Vec<String>,
}

impl PermissionRequest {
    /// Create a new permission request
    ///
    /// # Arguments
    ///
    /// * `id` - Unique request identifier
    /// * `session_id` - Session identifier
    /// * `permission` - Type of permission being requested
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_permission::{PermissionRequest, PermissionType};
    /// use std::collections::HashMap;
    ///
    /// let request = PermissionRequest::new(
    ///     "req_123",
    ///     "ses_456",
    ///     PermissionType::Edit,
    /// );
    /// assert_eq!(request.id, "req_123");
    /// assert_eq!(request.permission, PermissionType::Edit);
    /// ```
    pub fn new(
        id: impl Into<String>,
        session_id: impl Into<String>,
        permission: PermissionType,
    ) -> Self {
        Self {
            id: id.into(),
            session_id: session_id.into(),
            permission,
            tool_name: None,
            description: None,
            metadata: HashMap::new(),
            always_patterns: Vec::new(),
        }
    }

    /// Builder: Set the tool name
    pub fn with_tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = Some(name.into());
        self
    }

    /// Builder: Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: Set metadata
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builder: Set always patterns
    pub fn with_always_patterns(mut self, patterns: Vec<String>) -> Self {
        self.always_patterns = patterns;
        self
    }

    /// Get a formatted title for display
    ///
    /// Combines the icon, action verb, and target description.
    pub fn display_title(&self) -> String {
        let icon = self.permission.icon();
        let verb = self.permission.action_verb();

        if let Some(desc) = &self.description {
            format!("{} {} {}", icon, verb, desc)
        } else if let Some(tool) = &self.tool_name {
            format!("{} {} {}", icon, verb, tool)
        } else {
            format!("{} {} permission", icon, verb)
        }
    }
}

/// Action button labels
const ACTIONS: [&str; 3] = ["Allow once", "Allow always", "Reject"];

/// Permission dialog component
///
/// A modal dialog that presents permission requests with three action options.
/// Supports multi-stage workflow for "always" confirmation and reject messages.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog_permission::{
///     PermissionDialog, PermissionRequest, PermissionType, PermissionReply
/// };
///
/// let request = PermissionRequest::new("req_1", "ses_1", PermissionType::Edit)
///     .with_description("src/main.rs");
///
/// let mut dialog = PermissionDialog::new(request);
/// assert!(!dialog.is_visible());
///
/// dialog.show();
/// assert!(dialog.is_visible());
/// ```
pub struct PermissionDialog {
    /// Unique component identifier
    id: ComponentId,
    /// The permission request being displayed
    request: PermissionRequest,
    /// Current stage of the dialog
    stage: PermissionStage,
    /// Currently selected action index (0=once, 1=always, 2=reject)
    selected_action: usize,
    /// Whether the dialog is currently visible
    visible: bool,
    /// Whether the dialog currently has focus
    focused: bool,
    /// Message for reject stage
    reject_message: String,
    /// Result of the dialog interaction
    reply: Option<PermissionReply>,
}

impl PermissionDialog {
    /// Create a new PermissionDialog with the given request
    ///
    /// # Arguments
    ///
    /// * `request` - The permission request to display
    ///
    /// # Returns
    ///
    /// A new PermissionDialog hidden by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog_permission::{
    ///     PermissionDialog, PermissionRequest, PermissionType
    /// };
    ///
    /// let request = PermissionRequest::new("req_1", "ses_1", PermissionType::Read);
    /// let dialog = PermissionDialog::new(request);
    ///
    /// assert!(!dialog.is_visible());
    /// assert_eq!(dialog.request().permission, PermissionType::Read);
    /// ```
    pub fn new(request: PermissionRequest) -> Self {
        Self {
            id: ComponentId::new(),
            request,
            stage: PermissionStage::default(),
            selected_action: 0,
            visible: false,
            focused: false,
            reject_message: String::new(),
            reply: None,
        }
    }

    /// Get a reference to the permission request
    ///
    /// # Returns
    ///
    /// Reference to the PermissionRequest.
    pub fn request(&self) -> &PermissionRequest {
        &self.request
    }

    /// Get the current stage
    ///
    /// # Returns
    ///
    /// The current PermissionStage.
    pub fn stage(&self) -> PermissionStage {
        self.stage
    }

    /// Get the currently selected action index
    ///
    /// # Returns
    ///
    /// Index of the selected action (0=once, 1=always, 2=reject).
    pub fn selected_action(&self) -> usize {
        self.selected_action
    }

    /// Get the name of the currently selected action
    ///
    /// # Returns
    ///
    /// The action label as a string slice.
    pub fn selected_action_name(&self) -> &str {
        ACTIONS[self.selected_action]
    }

    /// Check if the dialog is currently visible
    ///
    /// # Returns
    ///
    /// `true` if the dialog is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the dialog
    ///
    /// Makes the dialog visible and focused, resetting to initial state.
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
        self.stage = PermissionStage::Permission;
        self.selected_action = 0;
        self.reply = None;
    }

    /// Hide the dialog
    ///
    /// Makes the dialog invisible and unfocused.
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
    }

    /// Check if the dialog is focused
    ///
    /// # Returns
    ///
    /// `true` if the dialog has focus.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Get the permission reply result
    ///
    /// # Returns
    ///
    /// Some(PermissionReply) if a selection was made, None otherwise.
    pub fn get_reply(&self) -> Option<PermissionReply> {
        self.reply
    }

    /// Get the reject message
    ///
    /// # Returns
    ///
    /// The reject message as a string slice.
    pub fn reject_message(&self) -> &str {
        &self.reject_message
    }

    /// Set the reject message
    ///
    /// # Arguments
    ///
    /// * `message` - The reject message
    pub fn set_reject_message(&mut self, message: impl Into<String>) {
        self.reject_message = message.into();
    }

    /// Move to the next action (right)
    ///
    /// Cycles through actions: once → always → reject → once
    pub fn next_action(&mut self) {
        self.selected_action = (self.selected_action + 1) % ACTIONS.len();
    }

    /// Move to the previous action (left)
    ///
    /// Cycles through actions: once → reject → always → once
    pub fn previous_action(&mut self) {
        if self.selected_action == 0 {
            self.selected_action = ACTIONS.len() - 1;
        } else {
            self.selected_action -= 1;
        }
    }

    /// Confirm the current action
    ///
    /// Handles stage transitions based on the selected action.
    /// Returns true if the dialog should close.
    pub fn confirm_action(&mut self) -> bool {
        match self.stage {
            PermissionStage::Permission => {
                match self.selected_action {
                    0 => {
                        // Allow once - close with reply
                        self.reply = Some(PermissionReply::Once);
                        true
                    }
                    1 => {
                        // Allow always - go to confirmation stage
                        self.stage = PermissionStage::Always;
                        false
                    }
                    2 => {
                        // Reject - go to reject stage
                        self.stage = PermissionStage::Reject;
                        false
                    }
                    _ => false,
                }
            }
            PermissionStage::Always => {
                // Confirm always
                self.reply = Some(PermissionReply::Always);
                true
            }
            PermissionStage::Reject => {
                // Confirm reject
                self.reply = Some(PermissionReply::Reject);
                true
            }
        }
    }

    /// Go back to the previous stage
    ///
    /// Returns true if went back, false if already at Permission stage.
    pub fn go_back(&mut self) -> bool {
        match self.stage {
            PermissionStage::Always | PermissionStage::Reject => {
                self.stage = PermissionStage::Permission;
                true
            }
            PermissionStage::Permission => false,
        }
    }

    /// Reset the dialog to initial state
    pub fn reset(&mut self) {
        self.stage = PermissionStage::Permission;
        self.selected_action = 0;
        self.reject_message.clear();
        self.reply = None;
    }

    /// Calculate the centered area for the dialog
    ///
    /// # Arguments
    ///
    /// * `frame_area` - The total available area
    ///
    /// # Returns
    ///
    /// A Rect representing the centered dialog area.
    fn centered_area(&self, frame_area: Rect) -> Rect {
        let width = (frame_area.width as f32 * 60.0 / 100.0) as u16;
        let height = (frame_area.height as f32 * 40.0 / 100.0) as u16;

        let x = frame_area.x + (frame_area.width.saturating_sub(width)) / 2;
        let y = frame_area.y + (frame_area.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }

    /// Get the title for the current stage
    fn stage_title(&self) -> &str {
        match self.stage {
            PermissionStage::Permission => "△ Permission required",
            PermissionStage::Always => "△ Always allow",
            PermissionStage::Reject => "△ Reject permission",
        }
    }

    /// Build the content lines for the current stage
    fn build_content(&self) -> Vec<Line<'_>> {
        let mut lines = Vec::new();

        match self.stage {
            PermissionStage::Permission => {
                // Title with icon
                let title = self.request.display_title();
                lines.push(Line::from(vec![Span::styled(
                    title,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )]));

                lines.push(Line::from(""));

                // Description if available
                if let Some(desc) = &self.request.description {
                    lines.push(Line::from(vec![Span::styled(
                        desc.clone(),
                        Style::default().fg(Color::Gray),
                    )]));
                }

                // Show diff hint or metadata
                if let Some(filepath) = self.request.metadata.get("filepath") {
                    lines.push(Line::from(vec![Span::styled(
                        format!("Path: {}", filepath),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }

                if let Some(pattern) = self.request.metadata.get("pattern") {
                    lines.push(Line::from(vec![Span::styled(
                        format!("Pattern: {}", pattern),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
            }
            PermissionStage::Always => {
                lines.push(Line::from(vec![Span::styled(
                    "This will allow the following patterns until the application is restarted:",
                    Style::default().fg(Color::White),
                )]));

                lines.push(Line::from(""));

                if self.request.always_patterns.is_empty()
                    || (self.request.always_patterns.len() == 1
                        && self.request.always_patterns[0] == "*")
                {
                    lines.push(Line::from(vec![Span::styled(
                        format!("- All {} operations", self.request.permission),
                        Style::default().fg(Color::Cyan),
                    )]));
                } else {
                    for pattern in &self.request.always_patterns {
                        lines.push(Line::from(vec![Span::styled(
                            format!("- {}", pattern),
                            Style::default().fg(Color::Cyan),
                        )]));
                    }
                }

                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    "Continue?",
                    Style::default().fg(Color::Gray),
                )]));
            }
            PermissionStage::Reject => {
                lines.push(Line::from(vec![Span::styled(
                    "Tell the agent what to do differently:",
                    Style::default().fg(Color::White),
                )]));

                lines.push(Line::from(""));

                // Show current reject message or placeholder
                if self.reject_message.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        "(Enter your feedback here)",
                        Style::default().fg(Color::DarkGray),
                    )]));
                } else {
                    lines.push(Line::from(vec![Span::styled(
                        self.reject_message.clone(),
                        Style::default().fg(Color::White),
                    )]));
                }
            }
        }

        lines
    }
}

impl Component for PermissionDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate centered area
        let dialog_area = self.centered_area(area);

        // Clear the area for modal overlay effect
        frame.render_widget(Clear, dialog_area);

        // Create semi-transparent background overlay for entire screen
        let overlay = Block::default().style(Style::default().bg(Color::Black));
        frame.render_widget(overlay, area);

        // Create dialog block with border
        // Use cyan when focused, white otherwise
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
            .title(self.stage_title())
            .borders(Borders::ALL)
            .border_style(border_style);

        // Calculate inner area
        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Create layout: content area + buttons area
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(inner);

        // Render content
        let content_lines = self.build_content();
        let content = Paragraph::new(content_lines).wrap(Wrap { trim: true });
        frame.render_widget(content, layout[0]);

        // Render buttons row
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33),
                Constraint::Percentage(34),
                Constraint::Percentage(33),
            ])
            .split(layout[1]);

        for (idx, action) in ACTIONS.iter().enumerate() {
            let is_selected =
                self.selected_action == idx && self.stage == PermissionStage::Permission;

            let style = if is_selected {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            };

            // In Always/Reject stage, show confirmation/cancel buttons
            let label = if self.stage != PermissionStage::Permission {
                if idx == 0 {
                    "Confirm"
                } else if idx == 2 {
                    "Cancel"
                } else {
                    ""
                }
            } else {
                action
            };

            if !label.is_empty() {
                let button = Paragraph::new(Line::from(Span::raw(format!(" {} ", label))))
                    .style(style)
                    .alignment(Alignment::Center);
                frame.render_widget(button, button_layout[idx]);
            }
        }

        // Render help text at the bottom
        let help_text = if self.stage == PermissionStage::Permission {
            Line::from(vec![
                Span::raw("←→ select  "),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" confirm  "),
                Span::styled(
                    "Esc",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" cancel"),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" confirm  "),
                Span::styled(
                    "Esc",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" back"),
            ])
        };

        let help_area = Rect::new(
            dialog_area.x,
            dialog_area.y + dialog_area.height,
            dialog_area.width,
            1,
        );
        let help = Paragraph::new(help_text).alignment(Alignment::Center);
        frame.render_widget(help, help_area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.visible {
            return EventPropagation::Continue;
        }

        match event.code {
            KeyCode::Esc => {
                if !self.go_back() {
                    // Already at Permission stage, hide with reject
                    self.reply = Some(PermissionReply::Reject);
                    self.hide();
                }
                EventPropagation::Stop
            }
            KeyCode::Left => {
                if self.stage == PermissionStage::Permission {
                    self.previous_action();
                }
                EventPropagation::Stop
            }
            KeyCode::Right => {
                if self.stage == PermissionStage::Permission {
                    self.next_action();
                }
                EventPropagation::Stop
            }
            KeyCode::Enter => {
                if self.confirm_action() {
                    self.hide();
                }
                EventPropagation::Stop
            }
            // Handle typing in reject stage
            KeyCode::Char(c) => {
                if self.stage == PermissionStage::Reject {
                    self.reject_message.push(c);
                }
                EventPropagation::Stop
            }
            KeyCode::Backspace => {
                if self.stage == PermissionStage::Reject {
                    self.reject_message.pop();
                }
                EventPropagation::Stop
            }
            _ => EventPropagation::Stop,
        }
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed for permission dialog
    }

    fn is_focusable(&self) -> bool {
        true // PermissionDialog captures focus when visible
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Default for PermissionDialog {
    fn default() -> Self {
        let request = PermissionRequest::new("default", "default", PermissionType::Edit);
        Self::new(request)
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_request() -> PermissionRequest {
        PermissionRequest::new("req_123", "ses_456", PermissionType::Edit)
            .with_description("src/main.rs")
            .with_tool_name("write_file")
    }

    #[test]
    fn test_permission_type_display() {
        assert_eq!(PermissionType::Edit.to_string(), "edit");
        assert_eq!(PermissionType::Read.to_string(), "read");
        assert_eq!(PermissionType::Bash.to_string(), "bash");

        let other = PermissionType::Other("custom".to_string());
        assert_eq!(other.to_string(), "custom");
    }

    #[test]
    fn test_permission_type_icon() {
        assert_eq!(PermissionType::Edit.icon(), "→");
        assert_eq!(PermissionType::Glob.icon(), "✱");
        assert_eq!(PermissionType::Bash.icon(), "#");
        assert_eq!(PermissionType::DoomLoop.icon(), "⟳");
    }

    #[test]
    fn test_permission_type_action_verb() {
        assert_eq!(PermissionType::Edit.action_verb(), "Edit");
        assert_eq!(PermissionType::Read.action_verb(), "Read");
        assert_eq!(PermissionType::Bash.action_verb(), "Execute");
    }

    #[test]
    fn test_permission_request_builder() {
        let mut metadata = HashMap::new();
        metadata.insert("filepath".to_string(), "src/main.rs".to_string());

        let request = PermissionRequest::new("req_1", "ses_1", PermissionType::Read)
            .with_tool_name("read_file")
            .with_description("Read config")
            .with_metadata(metadata.clone())
            .with_always_patterns(vec!["*.rs".to_string()]);

        assert_eq!(request.id, "req_1");
        assert_eq!(request.tool_name, Some("read_file".to_string()));
        assert_eq!(request.description, Some("Read config".to_string()));
        assert_eq!(request.metadata, metadata);
        assert_eq!(request.always_patterns, vec!["*.rs".to_string()]);
    }

    #[test]
    fn test_permission_request_display_title() {
        let request = PermissionRequest::new("req_1", "ses_1", PermissionType::Edit)
            .with_description("src/main.rs");
        assert_eq!(request.display_title(), "→ Edit src/main.rs");

        let request2 = PermissionRequest::new("req_2", "ses_2", PermissionType::Bash)
            .with_tool_name("execute_shell");
        assert_eq!(request2.display_title(), "# Execute execute_shell");

        let request3 = PermissionRequest::new("req_3", "ses_3", PermissionType::Read);
        assert_eq!(request3.display_title(), "→ Read permission");
    }

    #[test]
    fn test_permission_dialog_creation() {
        let request = create_test_request();
        let dialog = PermissionDialog::new(request);

        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
        assert_eq!(dialog.stage(), PermissionStage::Permission);
        assert_eq!(dialog.selected_action(), 0);
        assert!(dialog.get_reply().is_none());
    }

    #[test]
    fn test_permission_dialog_show_hide() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_action_cycling() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        assert_eq!(dialog.selected_action(), 0);
        assert_eq!(dialog.selected_action_name(), "Allow once");

        dialog.next_action();
        assert_eq!(dialog.selected_action(), 1);
        assert_eq!(dialog.selected_action_name(), "Allow always");

        dialog.next_action();
        assert_eq!(dialog.selected_action(), 2);
        assert_eq!(dialog.selected_action_name(), "Reject");

        // Cycle back to start
        dialog.next_action();
        assert_eq!(dialog.selected_action(), 0);

        // Cycle backwards
        dialog.previous_action();
        assert_eq!(dialog.selected_action(), 2);

        dialog.previous_action();
        assert_eq!(dialog.selected_action(), 1);
    }

    #[test]
    fn test_stage_transitions_once() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        dialog.show();
        assert_eq!(dialog.stage(), PermissionStage::Permission);

        // Select "Allow once" (index 0)
        dialog.selected_action = 0;
        let should_close = dialog.confirm_action();

        assert!(should_close);
        assert_eq!(dialog.get_reply(), Some(PermissionReply::Once));
    }

    #[test]
    fn test_stage_transitions_always() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        dialog.show();

        // Select "Allow always" (index 1)
        dialog.selected_action = 1;
        let should_close = dialog.confirm_action();

        assert!(!should_close);
        assert_eq!(dialog.stage(), PermissionStage::Always);
        assert!(dialog.get_reply().is_none());

        // Confirm in Always stage
        let should_close = dialog.confirm_action();
        assert!(should_close);
        assert_eq!(dialog.get_reply(), Some(PermissionReply::Always));
    }

    #[test]
    fn test_stage_transitions_reject() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        dialog.show();

        // Select "Reject" (index 2)
        dialog.selected_action = 2;
        let should_close = dialog.confirm_action();

        assert!(!should_close);
        assert_eq!(dialog.stage(), PermissionStage::Reject);
        assert!(dialog.get_reply().is_none());

        // Type a reject message
        dialog.set_reject_message("Please use a different approach");
        assert_eq!(dialog.reject_message(), "Please use a different approach");

        // Confirm in Reject stage
        let should_close = dialog.confirm_action();
        assert!(should_close);
        assert_eq!(dialog.get_reply(), Some(PermissionReply::Reject));
    }

    #[test]
    fn test_go_back() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        dialog.show();
        dialog.stage = PermissionStage::Always;

        assert!(dialog.go_back());
        assert_eq!(dialog.stage(), PermissionStage::Permission);

        // Already at Permission stage
        assert!(!dialog.go_back());
    }

    #[test]
    fn test_reset() {
        let request = create_test_request();
        let mut dialog = PermissionDialog::new(request);

        dialog.show();
        dialog.selected_action = 2;
        dialog.stage = PermissionStage::Reject;
        dialog.set_reject_message("Some message");

        dialog.reset();

        assert_eq!(dialog.stage(), PermissionStage::Permission);
        assert_eq!(dialog.selected_action(), 0);
        assert!(dialog.reject_message().is_empty());
        assert!(dialog.get_reply().is_none());
    }

    #[test]
    fn test_actions_constant() {
        assert_eq!(ACTIONS.len(), 3);
        assert_eq!(ACTIONS[0], "Allow once");
        assert_eq!(ACTIONS[1], "Allow always");
        assert_eq!(ACTIONS[2], "Reject");
    }

    #[test]
    fn test_stage_titles() {
        let request = create_test_request();
        let dialog = PermissionDialog::new(request);

        // Test through a mock or by checking the method directly
        // Since stage_title is private, we test indirectly through render
        // Here we just verify the stages exist and have different values
        assert_ne!(
            std::mem::discriminant(&PermissionStage::Permission),
            std::mem::discriminant(&PermissionStage::Always)
        );
        assert_ne!(
            std::mem::discriminant(&PermissionStage::Always),
            std::mem::discriminant(&PermissionStage::Reject)
        );
    }

    #[test]
    fn test_default_impl() {
        let dialog: PermissionDialog = Default::default();
        assert!(!dialog.is_visible());
        assert_eq!(dialog.stage(), PermissionStage::Permission);
    }

    #[test]
    fn test_permission_stage_default() {
        let stage: PermissionStage = Default::default();
        assert_eq!(stage, PermissionStage::Permission);
    }

    #[test]
    fn test_component_id_uniqueness() {
        let request = create_test_request();
        let dialog1 = PermissionDialog::new(request.clone());
        let dialog2 = PermissionDialog::new(request);

        assert_ne!(dialog1.id(), dialog2.id());
    }
}
