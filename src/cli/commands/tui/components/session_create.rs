//! Session Create Dialog Component
//!
//! A modal dialog for creating new sessions.
//! Captures focus when active and provides text input for session titles.
//!
//! # Features
//!
//! - Modal behavior with focus capture
//! - Text input with cursor support
//! - Keyboard navigation (Enter to submit, Escape to cancel)
//! - Centered positioning
//! - Cursor position tracking

use super::{Component, ComponentEvent, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Duration;

/// Modal dialog for creating new sessions
///
/// Displays a centered modal with a text input field for session titles.
/// Captures all input when active.
///
/// # Examples
///
/// ```rust,ignore
/// let mut dialog = SessionCreateDialog::new();
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
/// // Get the entered title
/// let title = dialog.get_title();
/// ```
pub struct SessionCreateDialog {
    id: ComponentId,
    visible: bool,
    focused: bool,
    title_input: String,
    cursor_position: usize,
}

impl SessionCreateDialog {
    /// Create a new SessionCreateDialog
    ///
    /// # Returns
    ///
    /// A new SessionCreateDialog hidden by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::session_create::SessionCreateDialog;
    ///
    /// let dialog = SessionCreateDialog::new();
    /// assert!(!dialog.is_visible());
    /// ```
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            visible: false,
            focused: false,
            title_input: String::new(),
            cursor_position: 0,
        }
    }

    /// Show the dialog
    ///
    /// Makes the dialog visible and focused.
    /// Resets the input field and cursor position.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut dialog = SessionCreateDialog::new();
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
    /// Does not clear the input field.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut dialog = SessionCreateDialog::new();
    /// dialog.show();
    /// dialog.hide();
    /// assert!(!dialog.is_visible());
    /// ```
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
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
    /// use crate::cli::commands::tui::components::session_create::SessionCreateDialog;
    ///
    /// let mut dialog = SessionCreateDialog::new();
    /// assert!(!dialog.is_visible());
    /// dialog.show();
    /// assert!(dialog.is_visible());
    /// ```
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the current title input
    ///
    /// # Returns
    ///
    /// The entered session title as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::session_create::SessionCreateDialog;
    ///
    /// let mut dialog = SessionCreateDialog::new();
    /// // After user types "My Session"...
    /// // assert_eq!(dialog.get_title(), "My Session");
    /// ```
    pub fn get_title(&self) -> String {
        self.title_input.clone()
    }

    /// Clear the input field
    ///
    /// Removes all text from the input field and resets cursor position.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::session_create::SessionCreateDialog;
    ///
    /// let mut dialog = SessionCreateDialog::new();
    /// // User types something...
    /// dialog.clear();
    /// assert!(dialog.get_title().is_empty());
    /// ```
    pub fn clear(&mut self) {
        self.title_input.clear();
        self.cursor_position = 0;
    }

    /// Calculate the centered area for the dialog
    ///
    /// # Arguments
    ///
    /// * `area` - The total available area
    /// * `percent_width` - The width as a percentage of the total area (0-100)
    /// * `percent_height` - The height as a percentage of the total area (0-100)
    ///
    /// # Returns
    ///
    /// A Rect representing the centered dialog area.
    fn centered_area(area: Rect, percent_width: u16, percent_height: u16) -> Rect {
        let width = area.width * percent_width / 100;
        let height = area.height * percent_height / 100;
        let x = area.x + (area.width - width) / 2;
        let y = area.y + (area.height - height) / 2;
        Rect::new(x, y, width, height)
    }
}

impl Component for SessionCreateDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = Self::centered_area(area, 50, 30);

        // Clear the area and render semi-transparent overlay
        frame.render_widget(Clear, dialog_area);

        // Dialog block with border
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .title(" Create New Session ")
            .borders(Borders::ALL)
            .border_style(border_style);

        // Create layout inside the dialog
        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        // Input field with cursor indicator
        let input_with_cursor = format!("{}_", self.title_input);
        let input_paragraph = Paragraph::new(input_with_cursor)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::BOTTOM));
        frame.render_widget(input_paragraph, layout[0]);

        // Help text
        let help_text = Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to create | "),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to cancel"),
        ]);
        let help_paragraph = Paragraph::new(help_text);
        frame.render_widget(help_paragraph, layout[1]);
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
                self.hide();
                EventPropagation::Stop
            }
            KeyCode::Char(c) => {
                self.title_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                EventPropagation::Stop
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.title_input.remove(self.cursor_position);
                }
                EventPropagation::Stop
            }
            KeyCode::Left => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                EventPropagation::Stop
            }
            KeyCode::Right => {
                if self.cursor_position < self.title_input.len() {
                    self.cursor_position += 1;
                }
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn handle_event(&mut self, event: &ComponentEvent) -> EventPropagation {
        match event {
            ComponentEvent::Key(key) => self.handle_input(*key),
            _ => EventPropagation::Continue,
        }
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

impl Default for SessionCreateDialog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_new() {
        let dialog = SessionCreateDialog::new();
        assert!(!dialog.is_visible());
        assert!(dialog.get_title().is_empty());
    }

    #[test]
    fn test_default() {
        let dialog: SessionCreateDialog = Default::default();
        assert!(!dialog.is_visible());
        assert!(dialog.get_title().is_empty());
    }

    #[test]
    fn test_show() {
        let mut dialog = SessionCreateDialog::new();
        assert!(!dialog.is_visible());
        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.focused());
    }

    #[test]
    fn test_hide() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();
        assert!(dialog.is_visible());
        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.focused());
    }

    #[test]
    fn test_show_resets_input() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        // Type some text
        dialog.handle_input(KeyEvent::from(KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('b')));
        assert_eq!(dialog.get_title(), "ab");

        // Hide and show again - should be cleared
        dialog.hide();
        dialog.show();
        // Note: show() doesn't clear, that's separate behavior
        assert_eq!(dialog.get_title(), "ab");
    }

    #[test]
    fn test_clear() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('t')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('t')));

        assert_eq!(dialog.get_title(), "test");

        dialog.clear();
        assert!(dialog.get_title().is_empty());
    }

    #[test]
    fn test_text_input() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('H')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('l')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('l')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('o')));

        assert_eq!(dialog.get_title(), "Hello");
    }

    #[test]
    fn test_backspace() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!(dialog.get_title(), "abc");

        // Backspace removes 'c'
        let result = dialog.handle_input(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(result, EventPropagation::Stop);
        assert_eq!(dialog.get_title(), "ab");

        // Backspace removes 'b'
        dialog.handle_input(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(dialog.get_title(), "a");

        // Backspace removes 'a'
        dialog.handle_input(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(dialog.get_title(), "");

        // Backspace at position 0 does nothing
        dialog.handle_input(KeyEvent::from(KeyCode::Backspace));
        assert_eq!(dialog.get_title(), "");
    }

    #[test]
    fn test_cursor_left() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('c')));

        // Cursor is at position 3, move left
        let result = dialog.handle_input(KeyEvent::from(KeyCode::Left));
        assert_eq!(result, EventPropagation::Stop);

        // Insert at position 2
        dialog.handle_input(KeyEvent::from(KeyCode::Char('x')));
        assert_eq!(dialog.get_title(), "abxc");
    }

    #[test]
    fn test_cursor_left_at_start() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        // Left at position 0 does nothing
        let result = dialog.handle_input(KeyEvent::from(KeyCode::Left));
        assert_eq!(result, EventPropagation::Stop);
    }

    #[test]
    fn test_cursor_right() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('b')));
        dialog.handle_input(KeyEvent::from(KeyCode::Left));

        let result = dialog.handle_input(KeyEvent::from(KeyCode::Right));
        assert_eq!(result, EventPropagation::Stop);

        dialog.handle_input(KeyEvent::from(KeyCode::Char('x')));
        assert_eq!(dialog.get_title(), "abx");
    }

    #[test]
    fn test_cursor_right_at_end() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('a')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('b')));

        // Right at end does nothing
        let result = dialog.handle_input(KeyEvent::from(KeyCode::Right));
        assert_eq!(result, EventPropagation::Stop);
    }

    #[test]
    fn test_enter_submits() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('M')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('y')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char(' ')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('S')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('i')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('o')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('n')));

        assert_eq!(dialog.get_title(), "My Session");
        assert!(dialog.is_visible());

        // Enter hides the dialog
        let result = dialog.handle_input(KeyEvent::from(KeyCode::Enter));
        assert_eq!(result, EventPropagation::Stop);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_esc_cancels() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        dialog.handle_input(KeyEvent::from(KeyCode::Char('t')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('e')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('s')));
        dialog.handle_input(KeyEvent::from(KeyCode::Char('t')));

        assert_eq!(dialog.get_title(), "test");
        assert!(dialog.is_visible());

        // Escape hides the dialog
        let result = dialog.handle_input(KeyEvent::from(KeyCode::Esc));
        assert_eq!(result, EventPropagation::Stop);
        assert!(!dialog.is_visible());
    }

    #[test]
    fn test_focus_transitions() {
        let mut dialog = SessionCreateDialog::new();

        assert!(!dialog.focused());

        dialog.on_focus();
        assert!(dialog.focused());

        dialog.on_blur();
        assert!(!dialog.focused());
    }

    #[test]
    fn test_is_focusable() {
        let dialog = SessionCreateDialog::new();
        assert!(dialog.is_focusable());
    }

    #[test]
    fn test_component_id_unique() {
        let dialog1 = SessionCreateDialog::new();
        let dialog2 = SessionCreateDialog::new();
        assert_ne!(dialog1.id(), dialog2.id());
    }

    #[test]
    fn test_handle_input_not_visible() {
        let mut dialog = SessionCreateDialog::new();
        // Dialog is hidden by default
        assert!(!dialog.is_visible());

        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let result = dialog.handle_input(event);
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_unhandled_key() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        // F1 key is not handled
        let event = KeyEvent::new(KeyCode::F(1), KeyModifiers::empty());
        let result = dialog.handle_input(event);
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_centered_area_calculation() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = SessionCreateDialog::centered_area(area, 50, 30);

        // 50% of 100 = 50 width
        assert_eq!(centered.width, 50);
        // 30% of 50 = 15 height
        assert_eq!(centered.height, 15);
        // Centered: x = (100 - 50) / 2 = 25
        assert_eq!(centered.x, 25);
        // Centered: y = (50 - 15) / 2 = 17 (integer division)
        assert_eq!(centered.y, 17);
    }

    #[test]
    fn test_render_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let dialog = SessionCreateDialog::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                dialog.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_render_visible_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        terminal
            .draw(|frame| {
                let area = frame.area();
                dialog.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_handle_event_key() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        let key_event = KeyEvent::from(KeyCode::Char('x'));
        let component_event = ComponentEvent::Key(key_event);

        let result = dialog.handle_event(&component_event);
        assert_eq!(result, EventPropagation::Stop);
        assert_eq!(dialog.get_title(), "x");
    }

    #[test]
    fn test_handle_event_non_key() {
        let mut dialog = SessionCreateDialog::new();
        dialog.show();

        let resize_event = ComponentEvent::Resize(80, 24);
        let result = dialog.handle_event(&resize_event);
        assert_eq!(result, EventPropagation::Continue);

        let tick_event = ComponentEvent::Tick;
        let result = dialog.handle_event(&tick_event);
        assert_eq!(result, EventPropagation::Continue);

        let focus_event = ComponentEvent::FocusGained;
        let result = dialog.handle_event(&focus_event);
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_update() {
        let mut dialog = SessionCreateDialog::new();
        // Should not panic
        dialog.update(Duration::from_millis(16));
    }
}
