//! TextArea Component — DeepSeek-TUI inspired composer with rounded border.
//!
//! Visual design:
//!
//! ```text
//! ╭─────────────────────────────────────────────────────╮
//! │ › Ask anything…                                     │
//! │                                                     │
//! ╰─────────────────────────────────────────────────────╯
//! ```
//!
//! - Border colour: ACCENT (purple) when streaming, BORDER_ACTIVE when focused, BORDER when idle
//! - `›` prompt prefix in PRIMARY (warm orange)
//! - Ctrl+J / Alt+Enter = insert newline
//! - Enter (no modifier) = submit (handled by parent via `take_submit`)

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use std::time::Duration;

// ── Palette ───────────────────────────────────────────────────────────────────

const C_TEXT: Color = Color::Rgb(238, 238, 238);
const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
const C_BG: Color = Color::Rgb(10, 10, 10);
const C_PRIMARY: Color = Color::Rgb(250, 178, 131);
const C_ACCENT: Color = Color::Rgb(157, 124, 216);
const C_BORDER: Color = Color::Rgb(72, 72, 72);
const C_BORDER_ACTIVE: Color = Color::Rgb(96, 96, 96);

// ── Component ─────────────────────────────────────────────────────────────────

pub struct TextArea {
    id: ComponentId,
    /// The live text buffer, lines stored without trailing newlines.
    lines: Vec<String>,
    /// Cursor position (row, col) — col is byte offset, not char index.
    cursor: (usize, usize),
    focused: bool,
    streaming: bool,
    placeholder: String,
    /// Whether a submit was requested this frame (cleared by take_submit).
    submit_pending: bool,
}

impl TextArea {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            lines: vec![String::new()],
            cursor: (0, 0),
            focused: false,
            streaming: false,
            placeholder: "Ask anything…".to_string(),
            submit_pending: false,
        }
    }

    // ── Public API ────────────────────────────────────────────────────────

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor = (0, 0);
        self.submit_pending = false;
    }

    pub fn set_placeholder(&mut self, p: impl Into<String>) {
        self.placeholder = p.into();
    }

    pub fn set_streaming(&mut self, s: bool) {
        self.streaming = s;
    }

    /// Returns the pending submit text and clears it, or None if none pending.
    pub fn take_submit(&mut self) -> Option<String> {
        if self.submit_pending {
            self.submit_pending = false;
            let text = self.text();
            if text.trim().is_empty() { return None; }
            self.clear();
            Some(text)
        } else {
            None
        }
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        self.cursor
    }

    /// Returns true when the first line starts with `/` (slash-command mode).
    pub fn starts_with_slash(&self) -> bool {
        self.lines.first().map(|l| l.starts_with('/')).unwrap_or(false)
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let s: String = text.into();
        self.lines = s.lines().map(String::from).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        let last = self.lines.len() - 1;
        self.cursor = (last, self.lines[last].len());
    }

    // ── Editing helpers ───────────────────────────────────────────────────

    fn insert_char(&mut self, c: char) {
        let (row, col) = self.cursor;
        self.lines[row].insert(col, c);
        self.cursor.1 += c.len_utf8();
    }

    fn insert_newline(&mut self) {
        let (row, col) = self.cursor;
        let tail = self.lines[row][col..].to_string();
        self.lines[row].truncate(col);
        self.lines.insert(row + 1, tail);
        self.cursor = (row + 1, 0);
    }

    fn backspace(&mut self) {
        let (row, col) = self.cursor;
        if col > 0 {
            // Find previous char boundary
            let s = &self.lines[row];
            let prev = s[..col]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.lines[row].remove(prev);
            self.cursor.1 = prev;
        } else if row > 0 {
            // Merge with previous line
            let cur_line = self.lines.remove(row);
            let prev_len = self.lines[row - 1].len();
            self.lines[row - 1].push_str(&cur_line);
            self.cursor = (row - 1, prev_len);
        }
    }

    fn delete_char(&mut self) {
        let (row, col) = self.cursor;
        if col < self.lines[row].len() {
            // Remove the char at col
            let c = self.lines[row][col..].chars().next().unwrap();
            self.lines[row].remove(col);
            let _ = c;
        } else if row + 1 < self.lines.len() {
            // Merge next line
            let next = self.lines.remove(row + 1);
            self.lines[row].push_str(&next);
        }
    }

    fn move_left(&mut self) {
        let (row, col) = self.cursor;
        if col > 0 {
            let prev = self.lines[row][..col]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.cursor.1 = prev;
        } else if row > 0 {
            let prev_row = row - 1;
            self.cursor = (prev_row, self.lines[prev_row].len());
        }
    }

    fn move_right(&mut self) {
        let (row, col) = self.cursor;
        if col < self.lines[row].len() {
            let next = self.lines[row][col..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| col + i)
                .unwrap_or(self.lines[row].len());
            self.cursor.1 = next;
        } else if row + 1 < self.lines.len() {
            self.cursor = (row + 1, 0);
        }
    }

    fn move_up(&mut self) {
        let (row, col) = self.cursor;
        if row > 0 {
            let new_col = col.min(self.lines[row - 1].len());
            self.cursor = (row - 1, new_col);
        }
    }

    fn move_down(&mut self) {
        let (row, col) = self.cursor;
        if row + 1 < self.lines.len() {
            let new_col = col.min(self.lines[row + 1].len());
            self.cursor = (row + 1, new_col);
        }
    }

    fn move_home(&mut self) { self.cursor.1 = 0; }

    fn move_end(&mut self) {
        let row = self.cursor.0;
        self.cursor.1 = self.lines[row].len();
    }

    fn kill_line(&mut self) {
        self.lines = vec![String::new()];
        self.cursor = (0, 0);
    }
}

impl Default for TextArea {
    fn default() -> Self { Self::new() }
}

// ── Component impl ────────────────────────────────────────────────────────────

impl Component for TextArea {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, f: &mut Frame, area: Rect) {
        let border_color = if self.streaming {
            C_ACCENT
        } else if self.focused {
            C_BORDER_ACTIVE
        } else {
            C_BORDER
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(C_BG));

        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height == 0 || inner.width == 0 { return; }

        if self.is_empty() && !self.focused {
            // Placeholder
            let line = Line::from(vec![
                Span::styled("› ", Style::default().fg(C_PRIMARY)),
                Span::styled(self.placeholder.clone(), Style::default().fg(C_TEXT_MUTED)),
            ]);
            f.render_widget(Paragraph::new(line), inner);
            return;
        }

        // Render text lines
        let mut rlines: Vec<Line<'static>> = Vec::new();
        for (i, line_str) in self.lines.iter().enumerate() {
            let prefix = if i == 0 {
                Span::styled("› ", Style::default().fg(C_PRIMARY))
            } else {
                Span::styled("  ", Style::default().fg(C_TEXT_MUTED))
            };
            let content = if line_str.is_empty() && i == 0 && self.lines.len() == 1 {
                Span::styled(String::new(), Style::default().fg(C_TEXT))
            } else {
                Span::styled(line_str.clone(), Style::default().fg(C_TEXT))
            };
            rlines.push(Line::from(vec![prefix, content]));
        }

        f.render_widget(
            Paragraph::new(rlines).wrap(Wrap { trim: false }),
            inner,
        );

        // Hardware cursor
        if self.focused && !self.streaming {
            let (crow, ccol) = self.cursor;
            // prefix is 2 wide ("› " or "  ")
            let cx = inner.x + 2 + ccol as u16;
            let cy = inner.y + crow as u16;
            if cy < inner.y + inner.height && cx < inner.x + inner.width {
                f.set_cursor_position(Position { x: cx, y: cy });
            }
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.focused || self.streaming {
            return EventPropagation::Continue;
        }

        // Ctrl+C passthrough → parent handles quit
        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            // Submit: Enter (no modifier) or Ctrl+M
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if !self.is_empty() {
                    self.submit_pending = true;
                }
                EventPropagation::Stop
            }
            // Newline: Ctrl+J, Alt+Enter, Shift+Enter
            (KeyCode::Char('j'), KeyModifiers::CONTROL)
            | (KeyCode::Enter, KeyModifiers::ALT)
            | (KeyCode::Enter, KeyModifiers::SHIFT) => {
                self.insert_newline();
                EventPropagation::Stop
            }
            // Navigation
            (KeyCode::Left, _)     => { self.move_left();  EventPropagation::Stop }
            (KeyCode::Right, _)    => { self.move_right(); EventPropagation::Stop }
            (KeyCode::Up, _)       => { self.move_up();    EventPropagation::Stop }
            (KeyCode::Down, _)     => { self.move_down();  EventPropagation::Stop }
            (KeyCode::Home, _)     => { self.move_home();  EventPropagation::Stop }
            (KeyCode::End, _)      => { self.move_end();   EventPropagation::Stop }
            // Ctrl+A = line start, Ctrl+E = line end
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => { self.move_home(); EventPropagation::Stop }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => { self.move_end();  EventPropagation::Stop }
            // Delete
            (KeyCode::Backspace, _) => { self.backspace();    EventPropagation::Stop }
            (KeyCode::Delete, _)    => { self.delete_char();  EventPropagation::Stop }
            // Ctrl+U = clear all
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => { self.kill_line(); EventPropagation::Stop }
            // Regular chars
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.insert_char(c);
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _: Duration) {}
    fn is_focusable(&self) -> bool { true }
    fn focused(&self) -> bool { self.focused }
    fn on_focus(&mut self) { self.focused = true; }
    fn on_blur(&mut self) { self.focused = false; }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_empty() {
        assert!(TextArea::new().is_empty());
    }

    #[test]
    fn insert_and_text() {
        let mut ta = TextArea::new();
        ta.insert_char('h');
        ta.insert_char('i');
        assert_eq!(ta.text(), "hi");
    }

    #[test]
    fn newline_splits_text() {
        let mut ta = TextArea::new();
        ta.insert_char('a');
        ta.insert_newline();
        ta.insert_char('b');
        assert_eq!(ta.text(), "a\nb");
    }

    #[test]
    fn backspace_removes_char() {
        let mut ta = TextArea::new();
        ta.insert_char('x');
        ta.backspace();
        assert!(ta.is_empty());
    }

    #[test]
    fn backspace_merges_lines() {
        let mut ta = TextArea::new();
        ta.insert_char('a');
        ta.insert_newline();
        ta.insert_char('b');
        ta.move_home();
        ta.backspace();
        assert_eq!(ta.text(), "ab");
    }

    #[test]
    fn take_submit_clears() {
        let mut ta = TextArea::new();
        ta.on_focus();
        ta.insert_char('x');
        ta.submit_pending = true;
        let result = ta.take_submit();
        assert_eq!(result, Some("x".to_string()));
        assert!(ta.is_empty());
    }

    #[test]
    fn kill_line_clears() {
        let mut ta = TextArea::new();
        ta.insert_char('a');
        ta.insert_char('b');
        ta.kill_line();
        assert!(ta.is_empty());
    }

    #[test]
    fn move_left_right() {
        let mut ta = TextArea::new();
        ta.insert_char('a');
        ta.insert_char('b');
        assert_eq!(ta.cursor, (0, 2));
        ta.move_left();
        assert_eq!(ta.cursor, (0, 1));
        ta.move_right();
        assert_eq!(ta.cursor, (0, 2));
    }

    #[test]
    fn component_ids_unique() {
        assert_ne!(TextArea::new().id(), TextArea::new().id());
    }
}
