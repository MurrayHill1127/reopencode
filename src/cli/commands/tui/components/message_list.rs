//! MessageList — scrollable conversation view

use super::{Component, ComponentId, EventPropagation};
use crate::cli::commands::tui::transcript::{MessageRole, PartType, ToolStatus, TranscriptMessage};
use crate::cli::commands::tui::transcript_renderer::TranscriptRenderer;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering::Relaxed};
use std::time::Duration;

pub struct MessageList {
    id: ComponentId,
    messages: Vec<TranscriptMessage>,
    /// Current scroll row (updated in both render and handle_input).
    scroll_offset: AtomicU16,
    /// Total rendered lines from last render() — used to clamp scroll.
    last_total_lines: AtomicU16,
    /// Visible height from last render() — used for page scroll.
    last_visible_height: AtomicU16,
    /// When true, view snaps to the bottom on each render.
    follow_bottom: AtomicBool,
    focused: bool,
    renderer: TranscriptRenderer,
}

impl MessageList {
    pub fn new(messages: Vec<TranscriptMessage>) -> Self {
        Self {
            id: ComponentId::new(),
            messages,
            scroll_offset: AtomicU16::new(0),
            last_total_lines: AtomicU16::new(0),
            last_visible_height: AtomicU16::new(1),
            follow_bottom: AtomicBool::new(true),
            focused: false,
            renderer: TranscriptRenderer::new(),
        }
    }

    pub fn with_title(messages: Vec<TranscriptMessage>, _title: impl Into<String>) -> Self {
        Self::new(messages)
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.scroll_offset.store(0, Relaxed);
        self.follow_bottom.store(true, Relaxed);
    }

    /// Append a message and snap to the bottom.
    pub fn push(&mut self, message: TranscriptMessage) {
        self.messages.push(message);
        self.follow_bottom.store(true, Relaxed);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom.store(true, Relaxed);
    }

    /// Append `text` to the last message's first non-synthetic text part.
    /// If no such part exists, adds a new one.
    pub fn append_last_text(&mut self, text: &str) {
        use crate::cli::commands::tui::transcript::PartType;
        if let Some(msg) = self.messages.last_mut() {
            for part in &mut msg.parts {
                if let PartType::Text { text: t, synthetic } = part {
                    if !*synthetic {
                        t.push_str(text);
                        return;
                    }
                }
            }
            msg.parts.push(PartType::Text {
                text: text.to_string(),
                synthetic: false,
            });
        }
    }

    /// Update the `duration_ms` field of the last assistant message.
    pub fn update_last_duration(&mut self, duration_ms: Option<u64>) {
        use crate::cli::commands::tui::transcript::MessageRole;
        if let Some(msg) = self.messages.last_mut() {
            if let MessageRole::Assistant { duration_ms: ref mut d, .. } = msg.role {
                *d = duration_ms;
            }
        }
    }

    fn scroll_up_by(&self, lines: u16) {
        self.follow_bottom.store(false, Relaxed);
        let cur = self.scroll_offset.load(Relaxed);
        self.scroll_offset.store(cur.saturating_sub(lines), Relaxed);
    }

    fn scroll_down_by(&self, lines: u16) {
        self.follow_bottom.store(false, Relaxed);
        let total = self.last_total_lines.load(Relaxed);
        let vis = self.last_visible_height.load(Relaxed);
        let max = total.saturating_sub(vis);
        let cur = self.scroll_offset.load(Relaxed);
        self.scroll_offset.store((cur + lines).min(max), Relaxed);
    }
}

impl Default for MessageList {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

// ─── text building ────────────────────────────────────────────────────────────

impl MessageList {
    fn build_text(&self, area_width: u16) -> Text<'static> {
        if self.messages.is_empty() {
            return Text::from(vec![Line::from(vec![Span::styled(
                "No messages yet — type something below.",
                Style::default().fg(Color::DarkGray),
            )])]);
        }

        let mut all_lines: Vec<Line<'static>> = Vec::new();
        let sep_len = (area_width.saturating_sub(4) as usize).min(60);

        for (i, message) in self.messages.iter().enumerate() {
            if i > 0 {
                all_lines.push(Line::from(Span::styled(
                    "─".repeat(sep_len),
                    Style::default().fg(Color::DarkGray),
                )));
                all_lines.push(Line::from(""));
            }

            // Role badge.
            let (label, label_style) = role_badge(&message.role);
            all_lines.push(Line::from(vec![Span::styled(
                label,
                label_style.add_modifier(Modifier::BOLD),
            )]));
            all_lines.push(Line::from(""));

            // Body.
            let md = build_parts_markdown(message);
            if !md.trim().is_empty() {
                let rendered = self.renderer.render(&md);
                all_lines.extend(rendered.lines);
            }
            all_lines.push(Line::from(""));
        }

        Text::from(all_lines)
    }
}

fn role_badge(role: &MessageRole) -> (String, Style) {
    match role {
        MessageRole::User => (
            " You ".to_string(),
            Style::default().fg(Color::Black).bg(Color::Cyan),
        ),
        MessageRole::Assistant { agent, model_id, duration_ms } => {
            let name = {
                let mut c = agent.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            };
            let dur = duration_ms
                .map(|ms| format!("  {:.1}s", ms as f64 / 1000.0))
                .unwrap_or_default();
            let label = format!(" {} · {}{} ", name, model_id, dur);
            (label, Style::default().fg(Color::Black).bg(Color::Green))
        }
    }
}

fn build_parts_markdown(message: &TranscriptMessage) -> String {
    let mut out = String::new();
    for part in &message.parts {
        match part {
            PartType::Text { text, synthetic } => {
                if !synthetic {
                    out.push_str(text);
                    out.push('\n');
                }
            }
            PartType::Reasoning { text } => {
                out.push_str("_Thinking:_\n\n");
                out.push_str(text);
                out.push_str("\n\n");
            }
            PartType::Tool { name, status, input, output, error } => {
                out.push_str(&format!("**{}**\n", name));
                if let Some(inp) = input {
                    out.push_str(&format!("\n```json\n{}\n```\n", inp));
                }
                match status {
                    ToolStatus::Completed => {
                        if let Some(o) = output {
                            out.push_str(&format!("\n```\n{}\n```\n", o));
                        }
                    }
                    ToolStatus::Error => {
                        if let Some(e) = error {
                            out.push_str(&format!("\n> Error: {}\n", e));
                        }
                    }
                    _ => {}
                }
                out.push('\n');
            }
        }
    }
    out
}

// ─── Component ───────────────────────────────────────────────────────────────

impl Component for MessageList {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let border_style = if self.focused {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = self.build_text(inner.width);
        let total = text.lines.len() as u16;
        self.last_total_lines.store(total, Relaxed);
        self.last_visible_height.store(inner.height, Relaxed);

        let offset = if self.follow_bottom.load(Relaxed) {
            total.saturating_sub(inner.height)
        } else {
            self.scroll_offset.load(Relaxed).min(total.saturating_sub(inner.height))
        };
        self.scroll_offset.store(offset, Relaxed);

        let para = Paragraph::new(text)
            .scroll((offset, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(para, inner);

        // Scroll hint — shown when there is content above the current view.
        if total > inner.height && offset > 0 && inner.width > 6 {
            let pct = offset as u32 * 100 / total.max(1) as u32;
            let hint = format!(" {}% ", pct);
            let hint_x = area.x + area.width.saturating_sub(hint.len() as u16 + 1);
            let hint_area = Rect::new(hint_x, area.y, hint.len() as u16, 1);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    hint,
                    Style::default().fg(Color::DarkGray),
                )),
                hint_area,
            );
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};
        if !self.focused {
            return EventPropagation::Continue;
        }
        let vis = self.last_visible_height.load(Relaxed).max(1);
        match (event.code, event.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.scroll_up_by(1);
                EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.scroll_down_by(1);
                EventPropagation::Stop
            }
            (KeyCode::PageUp, _) | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.scroll_up_by(vis / 2);
                EventPropagation::Stop
            }
            (KeyCode::PageDown, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.scroll_down_by(vis / 2);
                EventPropagation::Stop
            }
            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                self.scroll_offset.store(0, Relaxed);
                self.follow_bottom.store(false, Relaxed);
                EventPropagation::Stop
            }
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.follow_bottom.store(true, Relaxed);
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(text: &str) -> TranscriptMessage {
        TranscriptMessage {
            role: MessageRole::User,
            parts: vec![PartType::Text {
                text: text.to_string(),
                synthetic: false,
            }],
        }
    }

    fn assistant_msg(text: &str) -> TranscriptMessage {
        TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "build".to_string(),
                model_id: "claude-3".to_string(),
                duration_ms: Some(1234),
            },
            parts: vec![PartType::Text {
                text: text.to_string(),
                synthetic: false,
            }],
        }
    }

    #[test]
    fn test_new_empty() {
        let list = MessageList::default();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
    }

    #[test]
    fn test_push_increases_len() {
        let mut list = MessageList::default();
        list.push(user_msg("hello"));
        assert_eq!(list.len(), 1);
        list.push(assistant_msg("world"));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_clear() {
        let mut list = MessageList::default();
        list.push(user_msg("hello"));
        list.clear();
        assert!(list.is_empty());
        assert_eq!(list.scroll_offset.load(Relaxed), 0);
    }

    #[test]
    fn test_follow_bottom_on_push() {
        let mut list = MessageList::default();
        list.follow_bottom.store(false, Relaxed);
        list.push(user_msg("new message"));
        assert!(list.follow_bottom.load(Relaxed));
    }

    #[test]
    fn test_scroll_up_disables_follow() {
        let list = MessageList::default();
        list.last_total_lines.store(100, Relaxed);
        list.last_visible_height.store(20, Relaxed);
        list.scroll_offset.store(50, Relaxed);
        list.scroll_up_by(5);
        assert!(!list.follow_bottom.load(Relaxed));
        assert_eq!(list.scroll_offset.load(Relaxed), 45);
    }

    #[test]
    fn test_scroll_down_clamped() {
        let list = MessageList::default();
        list.last_total_lines.store(30, Relaxed);
        list.last_visible_height.store(20, Relaxed);
        list.scroll_offset.store(0, Relaxed);
        list.scroll_down_by(100);
        assert_eq!(list.scroll_offset.load(Relaxed), 10); // clamped to total - vis
    }

    #[test]
    fn test_component_ids_unique() {
        let l1 = MessageList::default();
        let l2 = MessageList::default();
        assert_ne!(l1.id(), l2.id());
    }

    #[test]
    fn test_is_focusable() {
        assert!(MessageList::default().is_focusable());
    }

    #[test]
    fn test_focus_state() {
        let mut list = MessageList::default();
        assert!(!list.focused());
        list.on_focus();
        assert!(list.focused());
        list.on_blur();
        assert!(!list.focused());
    }

    #[test]
    fn test_build_text_empty_placeholder() {
        let list = MessageList::default();
        let text = list.build_text(80);
        let flat: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(flat.contains("No messages"));
    }

    #[test]
    fn test_build_text_includes_content() {
        let mut list = MessageList::default();
        list.push(user_msg("Hello from test"));
        let text = list.build_text(80);
        let flat: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(flat.contains("Hello from test"));
    }

    #[test]
    fn test_build_text_multi_message_separator() {
        let mut list = MessageList::default();
        list.push(user_msg("first"));
        list.push(assistant_msg("second"));
        let text = list.build_text(80);
        // Separator line uses '─' character.
        let has_sep = text
            .lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains('─')));
        assert!(has_sep);
    }
}
