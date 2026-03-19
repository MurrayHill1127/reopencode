//! Toast Component
//!
//! A toast notification component with auto-dismiss functionality.
//! Displays temporary messages with visual variants (info, success, warning, error).
//!
//! # Features
//!
//! - Auto-dismiss after configurable duration
//! - Visual variants with distinct colors
//! - Manual dismiss on any key press
//! - Elapsed time tracking for auto-dismiss
//! - Position anchoring at top-right
//!
//! # Examples
//!
//! ```rust,ignore
//! let mut toast = Toast::success("Operation completed!")
//!     .with_duration(Duration::from_secs(5));
//!
//! toast.show();
//!
//! // In update loop
//! toast.tick(delta); // Auto-dismisses after duration
//!
//! // Or manually dismiss
//! toast.hide();
//! ```

use super::{Component, ComponentId, EventPropagation, ToastVariant};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::time::Duration;

/// Default toast display duration (5 seconds)
const DEFAULT_DURATION: Duration = Duration::from_secs(5);

/// Toast notification component
///
/// Displays a temporary message that auto-dismisses after a duration.
/// Commonly used for status notifications and user feedback.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::toast::Toast;
/// use std::time::Duration;
///
/// let toast = Toast::info("Loading...");
/// assert_eq!(toast.message(), "Loading...");
///
/// let mut toast = Toast::error("Failed!")
///     .with_duration(Duration::from_secs(3));
/// ```
pub struct Toast {
    /// Unique component identifier
    id: ComponentId,
    /// Optional title displayed in bold
    title: Option<String>,
    /// Toast message content
    message: String,
    /// Visual style variant
    variant: ToastVariant,
    /// Auto-dismiss duration
    duration: Duration,
    /// Elapsed time since shown
    elapsed: Duration,
    /// Whether the toast is currently visible
    visible: bool,
}

impl Toast {
    /// Create a new Toast with the given message
    ///
    /// # Arguments
    ///
    /// * `message` - The toast message content
    ///
    /// # Returns
    ///
    /// A new Toast with Info variant and default duration.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            id: ComponentId::new(),
            title: None,
            message: message.into(),
            variant: ToastVariant::Info,
            duration: DEFAULT_DURATION,
            elapsed: Duration::ZERO,
            visible: false,
        }
    }

    /// Set the toast title (builder pattern)
    ///
    /// # Arguments
    ///
    /// * `title` - The title to display in bold
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the toast variant (builder pattern)
    ///
    /// # Arguments
    ///
    /// * `variant` - The visual style variant
    pub fn with_variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the auto-dismiss duration (builder pattern)
    ///
    /// # Arguments
    ///
    /// * `duration` - Time before auto-dismiss
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Create a new Toast with Info variant
    ///
    /// # Arguments
    ///
    /// * `message` - The toast message content
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message).with_variant(ToastVariant::Info)
    }

    /// Create a new Toast with Success variant
    ///
    /// # Arguments
    ///
    /// * `message` - The toast message content
    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message).with_variant(ToastVariant::Success)
    }

    /// Create a new Toast with Warning variant
    ///
    /// # Arguments
    ///
    /// * `message` - The toast message content
    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message).with_variant(ToastVariant::Warning)
    }

    /// Create a new Toast with Error variant
    ///
    /// # Arguments
    ///
    /// * `message` - The toast message content
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message).with_variant(ToastVariant::Error)
    }

    /// Show the toast (make it visible)
    ///
    /// Resets elapsed time and sets visibility to true.
    pub fn show(&mut self) {
        self.elapsed = Duration::ZERO;
        self.visible = true;
    }

    /// Hide the toast (dismiss it)
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Advance elapsed time by delta
    ///
    /// # Arguments
    ///
    /// * `delta` - Time elapsed since last tick
    pub fn tick(&mut self, delta: Duration) {
        if self.visible {
            self.elapsed += delta;
        }
    }

    /// Check if the toast should be auto-dismissed
    ///
    /// # Returns
    ///
    /// `true` if the elapsed time exceeds the configured duration.
    pub fn should_dismiss(&self) -> bool {
        self.visible && self.elapsed >= self.duration
    }

    /// Check if the toast is currently visible
    ///
    /// # Returns
    ///
    /// `true` if the toast is visible and not dismissed.
    pub fn is_visible(&self) -> bool {
        self.visible && !self.should_dismiss()
    }

    /// Get the toast message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Set the toast message
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Get the toast title
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set the toast title
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = Some(title.into());
    }

    /// Get the toast variant
    pub fn variant(&self) -> ToastVariant {
        self.variant
    }

    /// Set the toast variant
    pub fn set_variant(&mut self, variant: ToastVariant) {
        self.variant = variant;
    }

    /// Get the auto-dismiss duration
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Set the auto-dismiss duration
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Get the elapsed time
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Get the border color for the current variant
    fn border_color(&self) -> Color {
        match self.variant {
            ToastVariant::Info => Color::Blue,
            ToastVariant::Success => Color::Green,
            ToastVariant::Warning => Color::Yellow,
            ToastVariant::Error => Color::Red,
        }
    }
}

impl Component for Toast {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Calculate toast dimensions
        let max_width = 60u16;
        let width = area.width.min(max_width).max(20);
        let height = if self.title.is_some() { 4u16 } else { 3u16 };

        // Position at top-right
        let toast_area = Rect {
            x: area.x + area.width.saturating_sub(width).saturating_sub(2),
            y: area.y + 2,
            width,
            height,
        };

        // Create border with variant color (left border only)
        let border_style = Style::default()
            .fg(self.border_color())
            .add_modifier(Modifier::BOLD);

        let block = Block::default()
            .borders(Borders::LEFT)
            .border_style(border_style);

        // Build content lines
        let lines: Vec<Line> = if let Some(ref title) = self.title {
            vec![
                Line::from(Span::styled(
                    title.as_str(),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(self.message.as_str()),
            ]
        } else {
            vec![Line::from(self.message.as_str())]
        };

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });

        frame.render_widget(paragraph, toast_area);
    }

    fn handle_input(&mut self, _event: KeyEvent) -> EventPropagation {
        // Any key press dismisses the toast
        if self.visible {
            self.hide();
            EventPropagation::Stop
        } else {
            EventPropagation::Continue
        }
    }

    fn update(&mut self, delta: Duration) {
        self.tick(delta);

        // Auto-dismiss when duration elapsed
        if self.should_dismiss() {
            self.hide();
        }
    }

    fn is_focusable(&self) -> bool {
        false
    }
}

impl Default for Toast {
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toast_new() {
        let toast = Toast::new("Test message");
        assert_eq!(toast.message(), "Test message");
        assert_eq!(toast.variant(), ToastVariant::Info);
        assert_eq!(toast.duration(), DEFAULT_DURATION);
        assert!(!toast.visible);
        assert_eq!(toast.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_toast_info() {
        let toast = Toast::info("Info message");
        assert_eq!(toast.variant(), ToastVariant::Info);
    }

    #[test]
    fn test_toast_success() {
        let toast = Toast::success("Success!");
        assert_eq!(toast.variant(), ToastVariant::Success);
    }

    #[test]
    fn test_toast_warning() {
        let toast = Toast::warning("Warning!");
        assert_eq!(toast.variant(), ToastVariant::Warning);
    }

    #[test]
    fn test_toast_error() {
        let toast = Toast::error("Error!");
        assert_eq!(toast.variant(), ToastVariant::Error);
    }

    #[test]
    fn test_toast_with_title() {
        let toast = Toast::info("Message").with_title("Title");
        assert_eq!(toast.title(), Some("Title"));
    }

    #[test]
    fn test_toast_with_variant() {
        let toast = Toast::new("Message").with_variant(ToastVariant::Error);
        assert_eq!(toast.variant(), ToastVariant::Error);
    }

    #[test]
    fn test_toast_with_duration() {
        let duration = Duration::from_secs(10);
        let toast = Toast::info("Message").with_duration(duration);
        assert_eq!(toast.duration(), duration);
    }

    #[test]
    fn test_toast_show_hide() {
        let mut toast = Toast::info("Test");
        assert!(!toast.visible);
        assert!(!toast.is_visible());

        toast.show();
        assert!(toast.visible);
        assert!(toast.is_visible());

        toast.hide();
        assert!(!toast.visible);
        assert!(!toast.is_visible());
    }

    #[test]
    fn test_toast_tick() {
        let mut toast = Toast::info("Test");
        toast.show();

        assert_eq!(toast.elapsed(), Duration::ZERO);
        toast.tick(Duration::from_millis(100));
        assert_eq!(toast.elapsed(), Duration::from_millis(100));
    }

    #[test]
    fn test_toast_tick_when_hidden() {
        let mut toast = Toast::info("Test");
        // Not shown, tick should not increment elapsed
        toast.tick(Duration::from_millis(100));
        assert_eq!(toast.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_toast_should_dismiss() {
        let mut toast = Toast::info("Test").with_duration(Duration::from_millis(100));
        toast.show();

        assert!(!toast.should_dismiss());

        toast.tick(Duration::from_millis(50));
        assert!(!toast.should_dismiss());

        toast.tick(Duration::from_millis(60));
        assert!(toast.should_dismiss());
    }

    #[test]
    fn test_toast_is_visible() {
        let mut toast = Toast::info("Test").with_duration(Duration::from_millis(100));
        assert!(!toast.is_visible());

        toast.show();
        assert!(toast.is_visible());

        toast.tick(Duration::from_millis(100));
        assert!(!toast.is_visible()); // Should be dismissed
    }

    #[test]
    fn test_toast_update_auto_dismiss() {
        let mut toast = Toast::info("Test").with_duration(Duration::from_millis(100));
        toast.show();

        toast.update(Duration::from_millis(150));
        assert!(!toast.visible); // Should be hidden after update
    }

    #[test]
    fn test_toast_handle_input_dismisses() {
        let mut toast = Toast::info("Test");
        toast.show();

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let result = toast.handle_input(event);

        assert_eq!(result, EventPropagation::Stop);
        assert!(!toast.visible);
    }

    #[test]
    fn test_toast_handle_input_when_hidden() {
        let mut toast = Toast::info("Test");
        // Not shown

        let event = KeyEvent::new(
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyModifiers::NONE,
        );
        let result = toast.handle_input(event);

        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_toast_border_color() {
        assert_eq!(Toast::info("Test").border_color(), Color::Blue);
        assert_eq!(Toast::success("Test").border_color(), Color::Green);
        assert_eq!(Toast::warning("Test").border_color(), Color::Yellow);
        assert_eq!(Toast::error("Test").border_color(), Color::Red);
    }

    #[test]
    fn test_toast_not_focusable() {
        let toast = Toast::info("Test");
        assert!(!toast.is_focusable());
    }

    #[test]
    fn test_toast_component_id_unique() {
        let toast1 = Toast::info("A");
        let toast2 = Toast::info("B");
        assert_ne!(toast1.id(), toast2.id());
    }

    #[test]
    fn test_toast_setters() {
        let mut toast = Toast::info("Old");
        toast.set_message("New message");
        assert_eq!(toast.message(), "New message");

        toast.set_title("New title");
        assert_eq!(toast.title(), Some("New title"));

        toast.set_variant(ToastVariant::Error);
        assert_eq!(toast.variant(), ToastVariant::Error);

        toast.set_duration(Duration::from_secs(30));
        assert_eq!(toast.duration(), Duration::from_secs(30));
    }

    #[test]
    fn test_toast_default() {
        let toast = Toast::default();
        assert_eq!(toast.message(), "");
        assert_eq!(toast.variant(), ToastVariant::Info);
    }

    #[test]
    fn test_toast_builder_chain() {
        let toast = Toast::new("Message")
            .with_title("Title")
            .with_variant(ToastVariant::Warning)
            .with_duration(Duration::from_secs(3));

        assert_eq!(toast.message(), "Message");
        assert_eq!(toast.title(), Some("Title"));
        assert_eq!(toast.variant(), ToastVariant::Warning);
        assert_eq!(toast.duration(), Duration::from_secs(3));
    }
}
