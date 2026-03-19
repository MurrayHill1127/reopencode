//! Dialog Component
//!
//! A modal dialog component for user confirmations and alerts.
//! Captures focus when active and blocks interaction with underlying components.
//!
//! # Features
//!
//! - Modal behavior with focus capture
//! - Size variants (small, medium, large)
//! - Title and content
//! - Close callback support
//! - Keyboard dismissal (Escape key)
//! - Centered positioning
//!
//! # Examples
//!
//! ```rust,ignore
//! let mut dialog = Dialog::new("Delete File?")
//!     .with_content("Are you sure you want to delete this file?")
//!     .with_size(DialogSize::Small);
//!
//! // In render loop
//! if dialog.is_visible() {
//!     dialog.render(frame, area);
//! }
//!
//! // Handle escape
//! let propagation = dialog.handle_input(event);
//! if propagation == EventPropagation::Stop {
//!     // Dialog handled the event
//! }
//! ```

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use std::fmt;
use std::time::Duration;

/// Dialog size variants
///
/// Determines the relative size of the dialog compared to the terminal area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DialogSize {
    /// Small dialog (40% width, 30% height)
    Small,
    /// Medium dialog (60% width, 50% height) - default
    #[default]
    Medium,
    /// Large dialog (80% width, 70% height)
    Large,
}

impl DialogSize {
    /// Get the width percentage for this size variant
    pub fn width_percent(&self) -> u16 {
        match self {
            DialogSize::Small => 40,
            DialogSize::Medium => 60,
            DialogSize::Large => 80,
        }
    }

    /// Get the height percentage for this size variant
    pub fn height_percent(&self) -> u16 {
        match self {
            DialogSize::Small => 30,
            DialogSize::Medium => 50,
            DialogSize::Large => 70,
        }
    }
}

/// Modal dialog component
///
/// Displays a centered modal dialog with title and content.
/// Captures all input when active and can trigger a close callback.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog::{Dialog, DialogSize};
///
/// let dialog = Dialog::new("Title").with_content("Content here");
/// assert_eq!(dialog.title(), "Title");
///
/// let dialog = Dialog::new("Delete?")
///     .with_content("Are you sure?")
///     .with_size(DialogSize::Large);
/// ```
pub struct Dialog {
    /// Unique component identifier
    id: ComponentId,
    /// Dialog title
    title: String,
    /// Dialog content
    content: String,
    /// Size variant
    size: DialogSize,
    /// Whether the dialog is currently visible
    visible: bool,
    /// Whether the dialog currently has focus
    focused: bool,
}

impl Dialog {
    /// Create a new Dialog with the given title
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    ///
    /// # Returns
    ///
    /// A new Dialog with Medium size, hidden by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::Dialog;
    ///
    /// let dialog = Dialog::new("Warning");
    /// assert_eq!(dialog.title(), "Warning");
    /// assert!(!dialog.is_visible()); // Hidden by default
    /// ```
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: ComponentId::new(),
            title: title.into(),
            content: String::new(),
            size: DialogSize::Medium,
            visible: false,
            focused: false,
        }
    }

    /// Builder: Set the dialog content
    ///
    /// # Arguments
    ///
    /// * `content` - The dialog content
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::Dialog;
    ///
    /// let dialog = Dialog::new("Title").with_content("This is the content");
    /// assert_eq!(dialog.content(), "This is the content");
    /// ```
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Builder: Set the dialog size
    ///
    /// # Arguments
    ///
    /// * `size` - The size variant
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::{Dialog, DialogSize};
    ///
    /// let dialog = Dialog::new("Title").with_size(DialogSize::Large);
    /// assert_eq!(dialog.size(), DialogSize::Large);
    /// ```
    pub fn with_size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    /// Create a confirmation dialog (shown by default)
    ///
    /// # Arguments
    ///
    /// * `title` - The confirmation title (e.g., "Delete File?")
    /// * `content` - The confirmation message
    ///
    /// # Returns
    ///
    /// A new Dialog configured for user confirmation.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::Dialog;
    ///
    /// let dialog = Dialog::confirm("Delete File?", "Are you sure?");
    /// assert!(dialog.is_visible());
    /// ```
    pub fn confirm(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: ComponentId::new(),
            title: title.into(),
            content: content.into(),
            size: DialogSize::Small,
            visible: true,
            focused: true,
        }
    }

    /// Create an alert dialog (shown by default)
    ///
    /// # Arguments
    ///
    /// * `title` - The alert title
    /// * `content` - The alert message
    ///
    /// # Returns
    ///
    /// A new Dialog configured for alerts.
    pub fn alert(title: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: ComponentId::new(),
            title: title.into(),
            content: content.into(),
            size: DialogSize::Medium,
            visible: true,
            focused: true,
        }
    }

    /// Get the dialog title
    ///
    /// # Returns
    ///
    /// The title as a string slice.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the dialog title
    ///
    /// # Arguments
    ///
    /// * `title` - The new title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Get the dialog content
    ///
    /// # Returns
    ///
    /// The content as a string slice.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the dialog content
    ///
    /// # Arguments
    ///
    /// * `content` - The new content
    pub fn set_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
    }

    /// Get the dialog size variant
    ///
    /// # Returns
    ///
    /// The current DialogSize.
    pub fn size(&self) -> DialogSize {
        self.size
    }

    /// Set the dialog size variant
    ///
    /// # Arguments
    ///
    /// * `size` - The new size variant
    pub fn set_size(&mut self, size: DialogSize) {
        self.size = size;
    }

    /// Check if the dialog is currently visible
    ///
    /// # Returns
    ///
    /// `true` if the dialog is visible.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::Dialog;
    ///
    /// let mut dialog = Dialog::new("Title");
    /// assert!(!dialog.is_visible());
    /// dialog.show();
    /// assert!(dialog.is_visible());
    /// ```
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the dialog
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut dialog = Dialog::new("Title");
    /// dialog.hide();
    /// dialog.show();
    /// assert!(dialog.is_visible());
    /// ```
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
    }

    /// Hide the dialog
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut dialog = Dialog::new("Title");
    /// dialog.show();
    /// dialog.hide();
    /// assert!(!dialog.is_visible());
    /// ```
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

    /// Calculate the centered area for the dialog
    ///
    /// # Arguments
    ///
    /// * `frame_area` - The total available area
    ///
    /// # Returns
    ///
    /// A Rect representing the centered dialog area.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let dialog = Dialog::new("Title");
    /// let area = Rect::new(0, 0, 100, 50);
    /// let dialog_area = dialog.centered_area(area);
    /// ```
    pub fn centered_area(&self, frame_area: Rect) -> Rect {
        let width = (frame_area.width as f32 * self.size.width_percent() as f32 / 100.0) as u16;
        let height = (frame_area.height as f32 * self.size.height_percent() as f32 / 100.0) as u16;

        let x = frame_area.x + (frame_area.width.saturating_sub(width)) / 2;
        let y = frame_area.y + (frame_area.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }
}

impl Component for Dialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate centered area
        let dialog_area = self.centered_area(area);

        // Create semi-transparent background overlay
        let overlay = Block::default().style(Style::default().bg(Color::Black));
        frame.render_widget(overlay, area);

        // Create dialog block with bold border
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
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create paragraph with content
        let paragraph = Paragraph::new(Line::from(Span::raw(&self.content)))
            .block(block)
            .wrap(Wrap { trim: true });

        // Render the dialog
        frame.render_widget(paragraph, dialog_area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.visible {
            return EventPropagation::Continue;
        }

        // Escape key closes the dialog
        if event.code == KeyCode::Esc {
            self.hide();
            return EventPropagation::Stop;
        }

        // Any other key is consumed but doesn't close
        // This prevents underlying components from receiving input
        EventPropagation::Stop
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed for dialog
    }

    fn is_focusable(&self) -> bool {
        true // Dialog captures focus when visible
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new("Dialog")
    }
}

/// Generic option for selection dialogs
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog::SelectOption;
///
/// let option = SelectOption::new("Option 1", 1);
/// assert_eq!(option.title(), "Option 1");
///
/// let option = SelectOption::new("Disabled Option", 0)
///     .with_description("Cannot be selected")
///     .disabled();
/// assert!(option.is_disabled());
/// ```
pub struct SelectOption<T> {
    /// Display title
    title: String,
    /// Associated value
    value: T,
    /// Optional description
    description: Option<String>,
    /// Optional category for grouping
    category: Option<String>,
    /// Whether the option is disabled
    disabled: bool,
    /// Optional footer text
    footer: Option<String>,
}

impl<T> SelectOption<T> {
    /// Create a new SelectOption
    ///
    /// # Arguments
    ///
    /// * `title` - The display title
    /// * `value` - The associated value
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::SelectOption;
    ///
    /// let option = SelectOption::new("Save", "save.png");
    /// assert_eq!(option.title(), "Save");
    /// assert_eq!(option.value(), "save.png");
    /// ```
    pub fn new(title: impl Into<String>, value: T) -> Self {
        Self {
            title: title.into(),
            value,
            description: None,
            category: None,
            disabled: false,
            footer: None,
        }
    }

    /// Get the title
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get the description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the category
    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }

    /// Check if disabled
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Get the footer
    pub fn footer(&self) -> Option<&str> {
        self.footer.as_deref()
    }

    /// Builder: Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builder: Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Builder: Set footer
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Builder: Mark as disabled
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }
}

impl<T> fmt::Display for SelectOption<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title)
    }
}

/// Perform fuzzy matching between query and target
///
/// Case-insensitive matching where all query characters must appear
/// in order in the target string.
///
/// # Arguments
///
/// * `query` - The search query
/// * `target` - The string to match against
///
/// # Returns
///
/// `true` if all characters in query appear in order in target.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog::fuzzy_match;
///
/// assert!(fuzzy_match("save", "Save File"));
/// assert!(fuzzy_match("svf", "Save File"));
/// assert!(!fuzzy_match("xyz", "Save File"));
/// assert!(fuzzy_match("", "Any String")); // Empty query matches anything
/// ```
pub fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    let mut query_idx = 0;
    for target_char in target_lower {
        if query_idx < query_lower.len() && target_char == query_lower[query_idx] {
            query_idx += 1;
        }
    }

    query_idx == query_lower.len()
}

/// Filter options by fuzzy matching against title and category
///
/// Excludes disabled options from results.
///
/// # Arguments
///
/// * `options` - The list of options to filter
/// * `query` - The search query
///
/// # Returns
///
/// Vector of indices of matching options.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog::{SelectOption, filter_options};
///
/// let options = vec![
///     SelectOption::new("Save", 1),
///     SelectOption::new("Open", 2),
///     SelectOption::new("Save As", 3),
/// ];
///
/// let matches = filter_options(&options, "sv");
/// assert_eq!(matches, vec![0, 2]); // "Save" and "Save As"
/// ```
pub fn filter_options<T>(options: &[SelectOption<T>], query: &str) -> Vec<usize> {
    let query_lower = query.to_lowercase();

    options
        .iter()
        .enumerate()
        .filter(|(_, opt)| {
            if opt.is_disabled() {
                return false;
            }

            // Check title
            if fuzzy_match(&query_lower, &opt.title) {
                return true;
            }

            // Check category
            if let Some(cat) = &opt.category {
                if fuzzy_match(&query_lower, cat) {
                    return true;
                }
            }

            false
        })
        .map(|(idx, _)| idx)
        .collect()
}

/// Input Dialog Component
///
/// A modal dialog for text input with cursor navigation and placeholder support.
/// Captures focus when active and provides inline text editing.
///
/// # Features
///
/// - Modal behavior with focus capture
/// - Text input with cursor support
/// - Placeholder text when empty
/// - Cursor navigation (Left/Right arrows)
/// - Keyboard submission (Enter) and cancellation (Escape)
/// - Centered positioning
///
/// # Examples
///
/// ```rust,ignore
/// let mut dialog = InputDialog::new("Enter Name");
/// dialog.show();
///
/// // In render loop
/// if dialog.is_visible() {
///     dialog.render(frame, area);
/// }
///
/// // Handle input
/// let propagation = dialog.handle_input(event);
/// if propagation == EventPropagation::Stop {
///     // Dialog handled the event
/// }
///
/// // Get the entered value
/// let value = dialog.get_value();
/// ```
pub struct InputDialog {
    /// Unique component identifier
    id: ComponentId,
    /// Dialog title
    title: String,
    /// Optional description shown below title
    description: Option<String>,
    /// Placeholder text shown when input is empty
    placeholder: String,
    /// Current input value
    value: String,
    /// Cursor position (0-based, can be at end of string)
    cursor_position: usize,
    /// Whether the dialog is currently visible
    visible: bool,
    /// Whether the dialog currently has focus
    focused: bool,
}

impl InputDialog {
    /// Create a new InputDialog with the given title
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    ///
    /// # Returns
    ///
    /// A new InputDialog with empty placeholder and value, hidden by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::InputDialog;
    ///
    /// let dialog = InputDialog::new("Enter Name");
    /// assert_eq!(dialog.title(), "Enter Name");
    /// assert!(!dialog.is_visible());
    /// ```
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: ComponentId::new(),
            title: title.into(),
            description: None,
            placeholder: String::new(),
            value: String::new(),
            cursor_position: 0,
            visible: false,
            focused: false,
        }
    }

    /// Builder: Set the description text
    ///
    /// # Arguments
    ///
    /// * `desc` - The description to show below the title
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::InputDialog;
    ///
    /// let dialog = InputDialog::new("Name")
    ///     .with_description("Enter your full name");
    /// assert_eq!(dialog.description(), Some("Enter your full name"));
    /// ```
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder: Set the placeholder text
    ///
    /// # Arguments
    ///
    /// * `text` - The placeholder to show when input is empty
    ///
    /// # Returns
    ///
    /// Self for method chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::InputDialog;
    ///
    /// let dialog = InputDialog::new("Name")
    ///     .with_placeholder("e.g., John Doe");
    /// assert_eq!(dialog.placeholder(), "e.g., John Doe");
    /// ```
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Builder: Set the initial value
    ///
    /// # Arguments
    ///
    /// * `text` - The initial input value
    ///
    /// # Returns
    ///
    /// Self for method chaining. Cursor is positioned at the end.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::InputDialog;
    ///
    /// let dialog = InputDialog::new("Name")
    ///     .with_value("Default Name");
    /// assert_eq!(dialog.get_value(), "Default Name");
    /// ```
    pub fn with_value(mut self, text: impl Into<String>) -> Self {
        let value = text.into();
        self.cursor_position = value.len();
        self.value = value;
        self
    }

    /// Get the title
    ///
    /// # Returns
    ///
    /// The title as a string slice.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get the description
    ///
    /// # Returns
    ///
    /// The description as an optional string slice.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the placeholder text
    ///
    /// # Returns
    ///
    /// The placeholder as a string slice.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Get the current input value
    ///
    /// # Returns
    ///
    /// The current input text as a String.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::InputDialog;
    ///
    /// let mut dialog = InputDialog::new("Name");
    /// assert!(dialog.get_value().is_empty());
    /// ```
    pub fn get_value(&self) -> String {
        self.value.clone()
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
    /// Makes the dialog visible and focused.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut dialog = InputDialog::new("Name");
    /// dialog.show();
    /// assert!(dialog.is_visible());
    /// ```
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
    }

    /// Hide the dialog
    ///
    /// Makes the dialog invisible and unfocused.
    /// Does not clear the input value.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut dialog = InputDialog::new("Name");
    /// dialog.show();
    /// dialog.hide();
    /// assert!(!dialog.is_visible());
    /// ```
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

    /// Clear the input field
    ///
    /// Removes all text from the input field and resets cursor position to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::InputDialog;
    ///
    /// let mut dialog = InputDialog::new("Name");
    /// // After user types something...
    /// dialog.clear();
    /// assert!(dialog.get_value().is_empty());
    /// assert_eq!(dialog.cursor_position, 0);
    /// ```
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    /// Calculate the centered area for the dialog
    ///
    /// # Arguments
    ///
    /// * `area` - The total available area
    ///
    /// # Returns
    ///
    /// A Rect representing the centered dialog area.
    fn centered_area(&self, area: Rect) -> Rect {
        let width = area.width * 50 / 100; // 50% width
        let height = area.height * 30 / 100; // 30% height
        let x = area.x + (area.width - width) / 2;
        let y = area.y + (area.height - height) / 2;
        Rect::new(x, y, width, height)
    }
}

impl Component for InputDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = self.centered_area(area);

        // Clear the area and render semi-transparent overlay
        frame.render_widget(Clear, dialog_area);

        // Dialog block with border
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .title(format!(" {} ", self.title))
            .borders(Borders::ALL)
            .border_style(border_style);

        // Calculate inner area
        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Create layout: description (optional), input field, help text
        let mut constraints = vec![Constraint::Min(1)]; // Input field
        if self.description.is_some() {
            constraints.insert(0, Constraint::Length(1)); // Description
        }
        constraints.push(Constraint::Length(1)); // Help text

        let layout = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        let mut layout_idx = 0;

        // Render description if present
        if let Some(desc) = &self.description {
            let desc_paragraph = Paragraph::new(Line::from(Span::styled(
                desc.as_str(),
                Style::default().fg(Color::Gray),
            )));
            frame.render_widget(desc_paragraph, layout[layout_idx]);
            layout_idx += 1;
        }

        // Render input field with cursor indicator
        let display_text = if self.value.is_empty() {
            // Show placeholder in gray when empty
            Line::from(Span::styled(
                self.placeholder.as_str(),
                Style::default().fg(Color::DarkGray),
            ))
        } else {
            // Show value with cursor as underscore
            // Build the text with cursor indicator
            let mut text_with_cursor = String::new();
            for (i, ch) in self.value.chars().enumerate() {
                if i == self.cursor_position {
                    text_with_cursor.push('_');
                }
                text_with_cursor.push(ch);
            }
            // Cursor at end
            if self.cursor_position == self.value.len() {
                text_with_cursor.push('_');
            }

            Line::from(Span::styled(
                text_with_cursor,
                Style::default().fg(Color::White),
            ))
        };

        let input_paragraph = Paragraph::new(display_text)
            .style(Style::default())
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(input_paragraph, layout[layout_idx]);
        layout_idx += 1;

        // Help text
        let help_text = Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" submit | "),
            Span::styled(
                "Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" cancel"),
        ]);
        let help_paragraph = Paragraph::new(help_text);
        frame.render_widget(help_paragraph, layout[layout_idx]);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.visible {
            return EventPropagation::Continue;
        }

        match event.code {
            KeyCode::Enter => {
                // Submit - hide dialog
                self.hide();
                EventPropagation::Stop
            }
            KeyCode::Esc => {
                // Cancel - hide dialog
                self.hide();
                EventPropagation::Stop
            }
            KeyCode::Char(c) => {
                // Insert character at cursor position
                self.value.insert(self.cursor_position, c);
                self.cursor_position += 1;
                EventPropagation::Stop
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.value.remove(self.cursor_position);
                }
                EventPropagation::Stop
            }
            KeyCode::Left => {
                // Move cursor left
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                EventPropagation::Stop
            }
            KeyCode::Right => {
                // Move cursor right
                if self.cursor_position < self.value.len() {
                    self.cursor_position += 1;
                }
                EventPropagation::Stop
            }
            KeyCode::Home => {
                // Move cursor to start
                self.cursor_position = 0;
                EventPropagation::Stop
            }
            KeyCode::End => {
                // Move cursor to end
                self.cursor_position = self.value.len();
                EventPropagation::Stop
            }
            KeyCode::Delete => {
                if self.cursor_position < self.value.len() {
                    self.value.remove(self.cursor_position);
                }
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Default for InputDialog {
    fn default() -> Self {
        Self::new("Input")
    }
}

// =============================================================================
// ConfirmDialog Component
// =============================================================================

/// Active button in the confirmation dialog
///
/// Determines which button (Cancel or Confirm) is currently selected.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ActiveButton {
    /// Cancel button is active (default)
    #[default]
    Cancel,
    /// Confirm button is active
    Confirm,
}

impl ActiveButton {
    /// Toggle between Cancel and Confirm
    ///
    /// # Returns
    ///
    /// The opposite button variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::ActiveButton;
    ///
    /// assert_eq!(ActiveButton::Cancel.toggle(), ActiveButton::Confirm);
    /// assert_eq!(ActiveButton::Confirm.toggle(), ActiveButton::Cancel);
    /// ```
    pub fn toggle(self) -> Self {
        match self {
            ActiveButton::Cancel => ActiveButton::Confirm,
            ActiveButton::Confirm => ActiveButton::Cancel,
        }
    }

    /// Get the label for this button
    ///
    /// # Returns
    ///
    /// A string slice representing the button label.
    pub fn label(&self) -> &'static str {
        match self {
            ActiveButton::Cancel => "Cancel",
            ActiveButton::Confirm => "Confirm",
        }
    }
}

/// Confirmation dialog with two buttons (Cancel/Confirm)
///
/// A modal dialog that presents a confirmation message with two selectable buttons.
/// Users can toggle between buttons using Left/Right arrow keys and activate
/// the selected button with Enter.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::dialog::{ConfirmDialog, ActiveButton};
///
/// let mut dialog = ConfirmDialog::new("Delete File?", "Are you sure?");
/// assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
/// ```
pub struct ConfirmDialog {
    /// Unique component identifier
    id: ComponentId,
    /// Dialog title
    title: String,
    /// Confirmation message
    message: String,
    /// Currently active button
    active_button: ActiveButton,
    /// Whether the dialog is currently visible
    visible: bool,
    /// Whether the dialog currently has focus
    focused: bool,
}

impl ConfirmDialog {
    /// Create a new ConfirmDialog with the given title and message
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    /// * `message` - The confirmation message
    ///
    /// # Returns
    ///
    /// A new ConfirmDialog with Cancel as the default active button, hidden by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::{ConfirmDialog, ActiveButton};
    ///
    /// let dialog = ConfirmDialog::new("Delete File?", "Are you sure?");
    /// assert_eq!(dialog.title(), "Delete File?");
    /// assert_eq!(dialog.message(), "Are you sure?");
    /// assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
    /// assert!(!dialog.is_visible());
    /// ```
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: ComponentId::new(),
            title: title.into(),
            message: message.into(),
            active_button: ActiveButton::Cancel,
            visible: false,
            focused: false,
        }
    }

    /// Get the dialog title
    ///
    /// # Returns
    ///
    /// The title as a string slice.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the dialog title
    ///
    /// # Arguments
    ///
    /// * `title` - The new title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Get the confirmation message
    ///
    /// # Returns
    ///
    /// The message as a string slice.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Set the confirmation message
    ///
    /// # Arguments
    ///
    /// * `message` - The new message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get the currently active button
    ///
    /// # Returns
    ///
    /// The active ActiveButton variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::{ConfirmDialog, ActiveButton};
    ///
    /// let dialog = ConfirmDialog::new("Title", "Message");
    /// assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
    /// ```
    pub fn get_active_button(&self) -> ActiveButton {
        self.active_button
    }

    /// Set the active button
    ///
    /// # Arguments
    ///
    /// * `button` - The button to set as active
    pub fn set_active_button(&mut self, button: ActiveButton) {
        self.active_button = button;
    }

    /// Toggle the active button between Cancel and Confirm
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::{ConfirmDialog, ActiveButton};
    ///
    /// let mut dialog = ConfirmDialog::new("Title", "Message");
    /// assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
    /// dialog.toggle_button();
    /// assert_eq!(dialog.get_active_button(), ActiveButton::Confirm);
    /// ```
    pub fn toggle_button(&mut self) {
        self.active_button = self.active_button.toggle();
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
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::ConfirmDialog;
    ///
    /// let mut dialog = ConfirmDialog::new("Title", "Message");
    /// dialog.show();
    /// assert!(dialog.is_visible());
    /// assert!(dialog.is_focused());
    /// ```
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
    }

    /// Hide the dialog
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::dialog::ConfirmDialog;
    ///
    /// let mut dialog = ConfirmDialog::new("Title", "Message");
    /// dialog.show();
    /// dialog.hide();
    /// assert!(!dialog.is_visible());
    /// assert!(!dialog.is_focused());
    /// ```
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

    /// Calculate the centered area for the dialog (Small size)
    ///
    /// # Arguments
    ///
    /// * `frame_area` - The total available area
    ///
    /// # Returns
    ///
    /// A Rect representing the centered dialog area.
    fn centered_area(&self, frame_area: Rect) -> Rect {
        let width = (frame_area.width as f32 * 40.0 / 100.0) as u16;
        let height = (frame_area.height as f32 * 30.0 / 100.0) as u16;

        let x = frame_area.x + (frame_area.width.saturating_sub(width)) / 2;
        let y = frame_area.y + (frame_area.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }
}

impl Component for ConfirmDialog {
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
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create message paragraph
        let paragraph = Paragraph::new(Line::from(Span::raw(&self.message)))
            .block(block)
            .wrap(Wrap { trim: true });

        // Render the dialog
        frame.render_widget(paragraph, dialog_area);

        // Render buttons row at the bottom of the dialog
        use ratatui::layout::{Constraint, Direction, Layout};

        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(Rect::new(
                dialog_area.x,
                dialog_area.y + dialog_area.height.saturating_sub(3),
                dialog_area.width,
                3,
            ));

        // Render Cancel button
        let cancel_style = if self.active_button == ActiveButton::Cancel {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        };
        let cancel_button = Paragraph::new(Line::from(Span::raw(" Cancel ")))
            .style(cancel_style)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(cancel_button, button_layout[0]);

        // Render Confirm button
        let confirm_style = if self.active_button == ActiveButton::Confirm {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(Color::DarkGray).fg(Color::White)
        };
        let confirm_button = Paragraph::new(Line::from(Span::raw(" Confirm ")))
            .style(confirm_style)
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(confirm_button, button_layout[1]);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::KeyCode;

        if !self.visible {
            return EventPropagation::Continue;
        }

        match event.code {
            // Escape hides the dialog
            KeyCode::Esc => {
                self.hide();
                EventPropagation::Stop
            }
            // Left/Right arrows toggle active button
            KeyCode::Left | KeyCode::Right => {
                self.toggle_button();
                EventPropagation::Stop
            }
            // Enter activates the selected button (returns Stop for parent to check which button)
            KeyCode::Enter => EventPropagation::Stop,
            // Any other key is consumed but doesn't activate
            _ => EventPropagation::Stop,
        }
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed for confirm dialog
    }

    fn is_focusable(&self) -> bool {
        true // ConfirmDialog captures focus when visible
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self::new("Confirm", "Are you sure?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_new() {
        let dialog = Dialog::new("Title");
        assert_eq!(dialog.title(), "Title");
        assert_eq!(dialog.content(), "");
        assert_eq!(dialog.size(), DialogSize::Medium);
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_dialog_with_content() {
        let dialog = Dialog::new("Title").with_content("Content here");
        assert_eq!(dialog.title(), "Title");
        assert_eq!(dialog.content(), "Content here");
    }

    #[test]
    fn test_dialog_with_size() {
        let dialog = Dialog::new("Title").with_size(DialogSize::Large);
        assert_eq!(dialog.size(), DialogSize::Large);
    }

    #[test]
    fn test_dialog_builder_chain() {
        let dialog = Dialog::new("Title")
            .with_content("Content")
            .with_size(DialogSize::Small);
        assert_eq!(dialog.title(), "Title");
        assert_eq!(dialog.content(), "Content");
        assert_eq!(dialog.size(), DialogSize::Small);
    }

    #[test]
    fn test_dialog_confirm() {
        let dialog = Dialog::confirm("Delete?", "Are you sure?");
        assert_eq!(dialog.title(), "Delete?");
        assert_eq!(dialog.content(), "Are you sure?");
        assert_eq!(dialog.size(), DialogSize::Small);
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());
    }

    #[test]
    fn test_dialog_alert() {
        let dialog = Dialog::alert("Warning", "This is a warning");
        assert_eq!(dialog.title(), "Warning");
        assert_eq!(dialog.content(), "This is a warning");
        assert_eq!(dialog.size(), DialogSize::Medium);
        assert!(dialog.is_visible());
    }

    #[test]
    fn test_dialog_set_title() {
        let mut dialog = Dialog::new("Old");
        dialog.set_title("New");
        assert_eq!(dialog.title(), "New");
    }

    #[test]
    fn test_dialog_set_content() {
        let mut dialog = Dialog::new("Title");
        dialog.set_content("New content");
        assert_eq!(dialog.content(), "New content");
    }

    #[test]
    fn test_dialog_set_size() {
        let mut dialog = Dialog::new("Title");
        assert_eq!(dialog.size(), DialogSize::Medium);
        dialog.set_size(DialogSize::Large);
        assert_eq!(dialog.size(), DialogSize::Large);
    }

    #[test]
    fn test_dialog_show_hide() {
        let mut dialog = Dialog::new("Title");
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());

        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_dialog_centered_area_medium() {
        let dialog = Dialog::new("Title");
        let frame_area = Rect::new(0, 0, 100, 50);
        let dialog_area = dialog.centered_area(frame_area);

        // Medium size = 60% width, 50% height
        assert_eq!(dialog_area.width, 60);
        assert_eq!(dialog_area.height, 25);
        assert_eq!(dialog_area.x, 20); // (100 - 60) / 2
        assert_eq!(dialog_area.y, 12); // (50 - 25) / 2
    }

    #[test]
    fn test_dialog_centered_area_small() {
        let dialog = Dialog::new("Title").with_size(DialogSize::Small);
        let frame_area = Rect::new(0, 0, 100, 50);
        let dialog_area = dialog.centered_area(frame_area);

        // Small size = 40% width, 30% height
        assert_eq!(dialog_area.width, 40);
        assert_eq!(dialog_area.height, 15);
        assert_eq!(dialog_area.x, 30); // (100 - 40) / 2
        assert_eq!(dialog_area.y, 17); // (50 - 15) / 2
    }

    #[test]
    fn test_dialog_centered_area_large() {
        let dialog = Dialog::new("Title").with_size(DialogSize::Large);
        let frame_area = Rect::new(0, 0, 100, 50);
        let dialog_area = dialog.centered_area(frame_area);

        // Large size = 80% width, 70% height
        assert_eq!(dialog_area.width, 80);
        assert_eq!(dialog_area.height, 35);
        assert_eq!(dialog_area.x, 10); // (100 - 80) / 2
        assert_eq!(dialog_area.y, 7); // (50 - 35) / 2
    }

    #[test]
    fn test_dialog_component_id() {
        let dialog1 = Dialog::new("A");
        let dialog2 = Dialog::new("B");
        assert_ne!(dialog1.id(), dialog2.id());
    }

    #[test]
    fn test_dialog_is_focusable() {
        let dialog = Dialog::new("Title");
        assert!(dialog.is_focusable());
    }

    #[test]
    fn test_dialog_on_focus() {
        let mut dialog = Dialog::new("Title");
        assert!(!dialog.is_focused());
        dialog.on_focus();
        assert!(dialog.is_focused());
    }

    #[test]
    fn test_dialog_on_blur() {
        let mut dialog = Dialog::new("Title");
        dialog.on_focus();
        assert!(dialog.is_focused());
        dialog.on_blur();
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_dialog_handle_input_not_visible() {
        let mut dialog = Dialog::new("Title");
        // Hidden by default
        assert!(!dialog.is_visible());

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_dialog_handle_input_escape() {
        let mut dialog = Dialog::new("Title");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(!dialog.is_visible()); // Hide called
    }

    #[test]
    fn test_dialog_handle_input_other_key() {
        let mut dialog = Dialog::new("Title");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(dialog.is_visible()); // Still visible
    }

    #[test]
    fn test_dialog_size_variants() {
        assert_eq!(DialogSize::Small.width_percent(), 40);
        assert_eq!(DialogSize::Small.height_percent(), 30);
        assert_eq!(DialogSize::Medium.width_percent(), 60);
        assert_eq!(DialogSize::Medium.height_percent(), 50);
        assert_eq!(DialogSize::Large.width_percent(), 80);
        assert_eq!(DialogSize::Large.height_percent(), 70);
    }

    #[test]
    fn test_dialog_default() {
        let dialog = Dialog::default();
        assert_eq!(dialog.title(), "Dialog");
        assert_eq!(dialog.content(), "");
        assert_eq!(dialog.size(), DialogSize::Medium);
        assert!(!dialog.is_visible());
    }

    // === SelectOption tests ===

    #[test]
    fn test_select_option_new() {
        let option = SelectOption::new("Option 1", 1);
        assert_eq!(option.title(), "Option 1");
        assert_eq!(*option.value(), 1);
        assert!(!option.is_disabled());
    }

    #[test]
    fn test_select_option_with_description() {
        let option = SelectOption::new("Option", 1).with_description("A description");
        assert_eq!(option.description(), Some("A description"));
    }

    #[test]
    fn test_select_option_with_category() {
        let option = SelectOption::new("Option", 1).with_category("Group A");
        assert_eq!(option.category(), Some("Group A"));
    }

    #[test]
    fn test_select_option_with_footer() {
        let option = SelectOption::new("Option", 1).with_footer("Footer text");
        assert_eq!(option.footer(), Some("Footer text"));
    }

    #[test]
    fn test_select_option_disabled() {
        let option = SelectOption::new("Option", 1).disabled();
        assert!(option.is_disabled());
    }

    #[test]
    fn test_select_option_builder_chain() {
        let option = SelectOption::new("Save", "save.png")
            .with_description("Save current file")
            .with_category("File")
            .with_footer("Ctrl+S")
            .disabled();

        assert_eq!(option.title(), "Save");
        assert_eq!(*option.value(), "save.png");
        assert_eq!(option.description(), Some("Save current file"));
        assert_eq!(option.category(), Some("File"));
        assert_eq!(option.footer(), Some("Ctrl+S"));
        assert!(option.is_disabled());
    }

    #[test]
    fn test_select_option_display() {
        let option = SelectOption::new("Test Option", 1);
        assert_eq!(format!("{}", option), "Test Option");
    }

    // === fuzzy_match tests ===

    #[test]
    fn test_fuzzy_match_exact() {
        assert!(fuzzy_match("save", "Save"));
        assert!(fuzzy_match("SAVE", "Save"));
        assert!(fuzzy_match("save", "save"));
    }

    #[test]
    fn test_fuzzy_match_subsequence() {
        assert!(fuzzy_match("svf", "Save File"));
        assert!(fuzzy_match("sv", "Save"));
        assert!(fuzzy_match("ave", "Save"));
    }

    #[test]
    fn test_fuzzy_match_no_match() {
        assert!(!fuzzy_match("xyz", "Save File"));
        assert!(!fuzzy_match("abc", "Save"));
    }

    #[test]
    fn test_fuzzy_match_empty_query() {
        assert!(fuzzy_match("", "Any String"));
        assert!(fuzzy_match("", ""));
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("ABC", "abc"));
        assert!(fuzzy_match("abc", "ABC"));
        assert!(fuzzy_match("AbCdEf", "aBcDeF"));
    }

    #[test]
    fn test_fuzzy_match_non_sequential() {
        assert!(fuzzy_match("sf", "Save File"));
        assert!(fuzzy_match("sfe", "Save File Editor"));
    }

    // === filter_options tests ===

    #[test]
    fn test_filter_options_basic() {
        let options = vec![
            SelectOption::new("Save", 1),
            SelectOption::new("Open", 2),
            SelectOption::new("Save As", 3),
        ];

        let matches = filter_options(&options, "sv");
        assert_eq!(matches, vec![0, 2]);
    }

    #[test]
    fn test_filter_options_empty_query() {
        let options = vec![SelectOption::new("Save", 1), SelectOption::new("Open", 2)];

        let matches = filter_options(&options, "");
        assert_eq!(matches, vec![0, 1]);
    }

    #[test]
    fn test_filter_options_excludes_disabled() {
        let options = vec![
            SelectOption::new("Save", 1),
            SelectOption::new("Open", 2).disabled(),
            SelectOption::new("Save As", 3),
        ];

        let matches = filter_options(&options, "sv");
        assert_eq!(matches, vec![0, 2]);
    }

    #[test]
    fn test_filter_options_category_match() {
        let options = vec![
            SelectOption::new("Save", 1).with_category("File"),
            SelectOption::new("Open", 2).with_category("File"),
            SelectOption::new("Quit", 3).with_category("Edit"),
        ];

        let matches = filter_options(&options, "fl");
        assert_eq!(matches, vec![0, 1]);
    }

    #[test]
    fn test_filter_options_no_matches() {
        let options = vec![SelectOption::new("Save", 1), SelectOption::new("Open", 2)];

        let matches = filter_options(&options, "xyz");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_filter_options_mixed_disabled_and_non_match() {
        let options = vec![
            SelectOption::new("Save", 1),
            SelectOption::new("Open", 2).disabled(),
            SelectOption::new("Close", 3),
        ];

        let matches = filter_options(&options, "clo");
        assert_eq!(matches, vec![2]);
    }

    // =============================================================================
    // ActiveButton Tests
    // =============================================================================

    #[test]
    fn test_active_button_default() {
        let button = ActiveButton::default();
        assert_eq!(button, ActiveButton::Cancel);
    }

    #[test]
    fn test_active_button_toggle() {
        assert_eq!(ActiveButton::Cancel.toggle(), ActiveButton::Confirm);
        assert_eq!(ActiveButton::Confirm.toggle(), ActiveButton::Cancel);
    }

    #[test]
    fn test_active_button_label_cancel() {
        assert_eq!(ActiveButton::Cancel.label(), "Cancel");
    }

    #[test]
    fn test_active_button_label_confirm() {
        assert_eq!(ActiveButton::Confirm.label(), "Confirm");
    }

    #[test]
    fn test_active_button_debug() {
        let cancel = format!("{:?}", ActiveButton::Cancel);
        let confirm = format!("{:?}", ActiveButton::Confirm);
        assert_eq!(cancel, "Cancel");
        assert_eq!(confirm, "Confirm");
    }

    #[test]
    fn test_active_button_copy() {
        let button1 = ActiveButton::Cancel;
        let button2 = button1;
        assert_eq!(button1, button2);
    }

    #[test]
    fn test_active_button_eq() {
        assert_eq!(ActiveButton::Cancel, ActiveButton::Cancel);
        assert_ne!(ActiveButton::Cancel, ActiveButton::Confirm);
    }

    // =============================================================================
    // ConfirmDialog Tests
    // =============================================================================

    #[test]
    fn test_confirm_dialog_new() {
        let dialog = ConfirmDialog::new("Title", "Message");
        assert_eq!(dialog.title(), "Title");
        assert_eq!(dialog.message(), "Message");
        assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_confirm_dialog_default() {
        let dialog = ConfirmDialog::default();
        assert_eq!(dialog.title(), "Confirm");
        assert_eq!(dialog.message(), "Are you sure?");
        assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_confirm_dialog_set_title() {
        let mut dialog = ConfirmDialog::new("Old", "Message");
        dialog.set_title("New");
        assert_eq!(dialog.title(), "New");
    }

    #[test]
    fn test_confirm_dialog_set_message() {
        let mut dialog = ConfirmDialog::new("Title", "Old");
        dialog.set_message("New");
        assert_eq!(dialog.message(), "New");
    }

    #[test]
    fn test_confirm_dialog_get_active_button() {
        let dialog = ConfirmDialog::new("Title", "Message");
        assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
    }

    #[test]
    fn test_confirm_dialog_set_active_button() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.set_active_button(ActiveButton::Confirm);
        assert_eq!(dialog.get_active_button(), ActiveButton::Confirm);
    }

    #[test]
    fn test_confirm_dialog_toggle_button() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
        dialog.toggle_button();
        assert_eq!(dialog.get_active_button(), ActiveButton::Confirm);
        dialog.toggle_button();
        assert_eq!(dialog.get_active_button(), ActiveButton::Cancel);
    }

    #[test]
    fn test_confirm_dialog_show() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());
    }

    #[test]
    fn test_confirm_dialog_hide() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());
        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_confirm_dialog_is_focused() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        assert!(!dialog.is_focused());
        dialog.show();
        assert!(dialog.is_focused());
        dialog.hide();
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_confirm_dialog_component_id() {
        let dialog1 = ConfirmDialog::new("A", "Message");
        let dialog2 = ConfirmDialog::new("B", "Message");
        assert_ne!(dialog1.id(), dialog2.id());
    }

    #[test]
    fn test_confirm_dialog_is_focusable() {
        let dialog = ConfirmDialog::new("Title", "Message");
        assert!(dialog.is_focusable());
    }

    #[test]
    fn test_confirm_dialog_on_focus() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        assert!(!dialog.is_focused());
        dialog.on_focus();
        assert!(dialog.is_focused());
    }

    #[test]
    fn test_confirm_dialog_on_blur() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.on_focus();
        assert!(dialog.is_focused());
        dialog.on_blur();
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_confirm_dialog_centered_area() {
        let dialog = ConfirmDialog::new("Title", "Message");
        let frame_area = Rect::new(0, 0, 100, 50);
        let dialog_area = dialog.centered_area(frame_area);

        // Small size = 40% width, 30% height
        assert_eq!(dialog_area.width, 40);
        assert_eq!(dialog_area.height, 15);
        assert_eq!(dialog_area.x, 30); // (100 - 40) / 2
        assert_eq!(dialog_area.y, 17); // (50 - 15) / 2
    }

    #[test]
    fn test_confirm_dialog_handle_input_not_visible() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        // Hidden by default
        assert!(!dialog.is_visible());

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_confirm_dialog_handle_input_escape() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Esc,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(!dialog.is_visible()); // Hide called
    }

    #[test]
    fn test_confirm_dialog_handle_input_left_arrow() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.show();
        dialog.set_active_button(ActiveButton::Confirm);

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert_eq!(dialog.get_active_button(), ActiveButton::Cancel); // Toggled
        assert!(dialog.is_visible()); // Still visible
    }

    #[test]
    fn test_confirm_dialog_handle_input_right_arrow() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert_eq!(dialog.get_active_button(), ActiveButton::Confirm); // Toggled
        assert!(dialog.is_visible()); // Still visible
    }

    #[test]
    fn test_confirm_dialog_handle_input_enter() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(dialog.is_visible()); // Still visible (parent decides action)
    }

    #[test]
    fn test_confirm_dialog_handle_input_other_key() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::empty(),
        );
        assert_eq!(dialog.handle_input(event), EventPropagation::Stop);
        assert!(dialog.is_visible()); // Still visible
    }

    #[test]
    fn test_confirm_dialog_update() {
        let mut dialog = ConfirmDialog::new("Title", "Message");
        let delta = Duration::from_millis(100);
        dialog.update(delta);
        // Update should not change anything
        assert_eq!(dialog.title(), "Title");
        assert_eq!(dialog.message(), "Message");
    }

    // =============================================================================
    // InputDialog Tests
    // =============================================================================

    #[test]
    fn test_input_dialog_new() {
        let dialog = InputDialog::new("Enter Name");
        assert_eq!(dialog.title(), "Enter Name");
        assert_eq!(dialog.description(), None);
        assert_eq!(dialog.placeholder(), "");
        assert!(dialog.get_value().is_empty());
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_input_dialog_default() {
        let dialog = InputDialog::default();
        assert_eq!(dialog.title(), "Input");
        assert!(dialog.get_value().is_empty());
    }

    #[test]
    fn test_input_dialog_with_description() {
        let dialog = InputDialog::new("Name").with_description("Enter your full name");
        assert_eq!(dialog.description(), Some("Enter your full name"));
    }

    #[test]
    fn test_input_dialog_with_placeholder() {
        let dialog = InputDialog::new("Name").with_placeholder("e.g., John Doe");
        assert_eq!(dialog.placeholder(), "e.g., John Doe");
    }

    #[test]
    fn test_input_dialog_with_value() {
        let dialog = InputDialog::new("Name").with_value("Default Name");
        assert_eq!(dialog.get_value(), "Default Name");
        // Cursor should be at the end
        assert_eq!(dialog.cursor_position, 12); // "Default Name" length
    }

    #[test]
    fn test_input_dialog_builder_chain() {
        let dialog = InputDialog::new("Name")
            .with_description("Enter your name")
            .with_placeholder("John Doe")
            .with_value("Initial");
        assert_eq!(dialog.description(), Some("Enter your name"));
        assert_eq!(dialog.placeholder(), "John Doe");
        assert_eq!(dialog.get_value(), "Initial");
    }

    #[test]
    fn test_input_dialog_show_hide() {
        let mut dialog = InputDialog::new("Name");
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());

        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_input_dialog_clear() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('t')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('t')));

        assert_eq!(dialog.get_value(), "test");

        dialog.clear();
        assert!(dialog.get_value().is_empty());
        assert_eq!(dialog.cursor_position, 0);
    }

    #[test]
    fn test_input_dialog_text_input() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('H')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('l')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('l')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('o')));

        assert_eq!(dialog.get_value(), "Hello");
        assert_eq!(dialog.cursor_position, 5);
    }

    #[test]
    fn test_input_dialog_backspace() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('c')));
        assert_eq!(dialog.get_value(), "abc");

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Backspace));
        assert_eq!(dialog.get_value(), "ab");
        assert_eq!(dialog.cursor_position, 2);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Backspace));
        assert_eq!(dialog.get_value(), "a");

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Backspace));
        assert_eq!(dialog.get_value(), "");
        assert_eq!(dialog.cursor_position, 0);

        // Backspace at position 0 does nothing
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Backspace));
        assert_eq!(dialog.get_value(), "");
    }

    #[test]
    fn test_input_dialog_cursor_left() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('c')));

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Left));
        assert_eq!(dialog.cursor_position, 2);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('x')));
        assert_eq!(dialog.get_value(), "abxc");
        assert_eq!(dialog.cursor_position, 3);
    }

    #[test]
    fn test_input_dialog_cursor_left_at_start() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        let result = dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Left));
        assert_eq!(result, EventPropagation::Stop);
        assert_eq!(dialog.cursor_position, 0);
    }

    #[test]
    fn test_input_dialog_cursor_right() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Left));

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Right));
        assert_eq!(dialog.cursor_position, 2);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('x')));
        assert_eq!(dialog.get_value(), "abx");
    }

    #[test]
    fn test_input_dialog_cursor_right_at_end() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));

        let result = dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Right));
        assert_eq!(result, EventPropagation::Stop);
        assert_eq!(dialog.cursor_position, 2);
    }

    #[test]
    fn test_input_dialog_home_key() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('c')));
        assert_eq!(dialog.cursor_position, 3);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Home));
        assert_eq!(dialog.cursor_position, 0);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('x')));
        assert_eq!(dialog.get_value(), "xabc");
    }

    #[test]
    fn test_input_dialog_end_key() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Left));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Left));
        assert_eq!(dialog.cursor_position, 0);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::End));
        assert_eq!(dialog.cursor_position, 2);
    }

    #[test]
    fn test_input_dialog_delete_key() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('c')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Left));
        assert_eq!(dialog.cursor_position, 2);

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Delete));
        assert_eq!(dialog.get_value(), "ab");
        assert_eq!(dialog.cursor_position, 2);

        // Delete at end does nothing
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Delete));
        assert_eq!(dialog.get_value(), "ab");
    }

    #[test]
    fn test_input_dialog_enter_submits() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('T')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('t')));

        assert_eq!(dialog.get_value(), "Test");
        assert!(dialog.is_visible());

        let result = dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Enter));
        assert_eq!(result, EventPropagation::Stop);
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_value(), "Test"); // Value preserved
    }

    #[test]
    fn test_input_dialog_esc_cancels() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('t')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('t')));

        assert_eq!(dialog.get_value(), "test");
        assert!(dialog.is_visible());

        let result = dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Esc));
        assert_eq!(result, EventPropagation::Stop);
        assert!(!dialog.is_visible());
        assert_eq!(dialog.get_value(), "test"); // Value preserved
    }

    #[test]
    fn test_input_dialog_handle_input_not_visible() {
        let mut dialog = InputDialog::new("Name");
        assert!(!dialog.is_visible());

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::empty(),
        );
        let result = dialog.handle_input(event);
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_input_dialog_unhandled_key() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::F(1),
            crossterm::event::KeyModifiers::empty(),
        );
        let result = dialog.handle_input(event);
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_input_dialog_focus_transitions() {
        let mut dialog = InputDialog::new("Name");

        assert!(!dialog.is_focused());

        dialog.on_focus();
        assert!(dialog.is_focused());

        dialog.on_blur();
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_input_dialog_is_focusable() {
        let dialog = InputDialog::new("Name");
        assert!(dialog.is_focusable());
    }

    #[test]
    fn test_input_dialog_component_id_unique() {
        let dialog1 = InputDialog::new("A");
        let dialog2 = InputDialog::new("B");
        assert_ne!(dialog1.id(), dialog2.id());
    }

    #[test]
    fn test_input_dialog_render_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let dialog = InputDialog::new("Name");

        terminal
            .draw(|frame| {
                let area = frame.area();
                dialog.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_input_dialog_render_visible_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        terminal
            .draw(|frame| {
                let area = frame.area();
                dialog.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_input_dialog_update() {
        let mut dialog = InputDialog::new("Name");
        dialog.update(Duration::from_millis(16));
        // Should not panic
    }

    #[test]
    #[ignore = "TODO: fix multibyte character handling in InputDialog"]
    fn test_input_dialog_cursor_position_with_multibyte_chars() {
        let mut dialog = InputDialog::new("Name");
        dialog.show();

        // Insert emoji (multibyte character)
        // cursor_position tracks character count, not byte count
        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('😀')));
        assert_eq!(dialog.get_value(), "😀");
        assert_eq!(dialog.cursor_position, 1); // 1 character, not 4 bytes

        dialog.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('a')));
        assert_eq!(dialog.get_value(), "😀a");
        assert_eq!(dialog.cursor_position, 2); // 2 characters
    }
}
