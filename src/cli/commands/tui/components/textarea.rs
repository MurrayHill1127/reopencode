//! TextArea Component - wraps tui-textarea crate

use super::{Component, ComponentId, EventPropagation};
use ratatui::{layout::Rect, style::{Color, Modifier, Style}, widgets::{Block, Borders}, Frame};
use crossterm::event::KeyEvent;
use tui_textarea::{TextArea as TuiTextArea, Input, Scrolling};
use std::time::Duration;

pub struct TextArea {
    id: ComponentId,
    textarea: TuiTextArea<'static>,
    title: String,
    focused: bool,
    placeholder: String,
    read_only: bool,
}

impl TextArea {
    pub fn new() -> Self {
        let textarea = TuiTextArea::default();
        Self {
            id: ComponentId::new(),
            textarea,
            title: String::from("Input"),
            focused: false,
            placeholder: String::new(),
            read_only: false,
        }
    }

    pub fn with_title(title: impl Into<String>) -> Self {
        let mut this = Self::new();
        this.title = title.into();
        this
    }

    pub fn with_text(text: impl Into<String>) -> Self {
        let mut this = Self::new();
        this.set_text(text);
        this
    }

    pub fn text(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let lines: Vec<String> = text.into().lines().map(String::from).collect();
        self.textarea = TuiTextArea::new(lines);
    }

    pub fn title(&self) -> &str { &self.title }
    pub fn set_title(&mut self, title: impl Into<String>) { self.title = title.into(); }
    pub fn placeholder(&self) -> &str { &self.placeholder }
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) { self.placeholder = placeholder.into(); }
    pub fn is_read_only(&self) -> bool { self.read_only }
    pub fn set_read_only(&mut self, read_only: bool) { self.read_only = read_only; }
    pub fn cursor_position(&self) -> (usize, usize) { self.textarea.cursor() }
    pub fn is_empty(&self) -> bool {
        self.textarea.lines().is_empty() ||
        (self.textarea.lines().len() == 1 && self.textarea.lines()[0].is_empty())
    }
    pub fn line_count(&self) -> usize { self.textarea.lines().len() }
    pub fn clear(&mut self) { self.textarea = TuiTextArea::default(); }
}

impl Default for TextArea {
    fn default() -> Self { Self::new() }
}

impl Component for TextArea {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let block = if self.focused {
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
        };

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Render textarea content manually
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;
        
        let lines: Vec<Line> = self.textarea.lines().iter().map(|s| Line::from(Span::raw(s.clone()))).collect();
        let paragraph = Paragraph::new(lines).block(Block::default());
        frame.render_widget(paragraph, inner);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        if !self.focused {
            return EventPropagation::Continue;
        }

        if self.read_only {
            match (event.code, event.modifiers) {
                (KeyCode::Up, _) | (KeyCode::Down, _) |
                (KeyCode::Left, _) | (KeyCode::Right, _) |
                (KeyCode::Home, _) | (KeyCode::End, _) |
                (KeyCode::PageUp, _) | (KeyCode::PageDown, _) => {}
                _ => return EventPropagation::Stop,
            }
        }

        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
            return EventPropagation::Continue;
        }

        let input: Input = event.into();
        self.textarea.input(input);
        EventPropagation::Stop
    }

    fn update(&mut self, _delta: Duration) {}
    fn is_focusable(&self) -> bool { true }
    fn on_focus(&mut self) { self.focused = true; }
    fn on_blur(&mut self) { self.focused = false; }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_textarea_new() {
        let textarea = TextArea::new();
        assert_eq!(textarea.text(), "");
    }

    #[test]
    fn test_textarea_component_id() {
        let t1 = TextArea::new();
        let t2 = TextArea::new();
        assert_ne!(t1.id(), t2.id());
    }
}
