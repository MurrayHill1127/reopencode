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
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
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
}
