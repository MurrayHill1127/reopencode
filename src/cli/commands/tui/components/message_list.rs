//! MessageList — per-cell cached scrollable conversation view.
//!
//! ## Caching Architecture
//!
//! Each message has a `revision: u64` counter. When a cell is rendered, the
//! resulting `Vec<Line>` is stored alongside the revision and the width at which
//! it was rendered. On the next frame we reuse the cached lines if both the
//! revision and the width are unchanged — avoiding markdown re-parsing on every
//! frame (critical during streaming and on terminal resize).
//!
//! During streaming, `append_last_text` bumps the last cell's revision so it
//! gets re-rendered with the new content.

use super::{Component, ComponentId, EventPropagation};
use crate::cli::commands::tui::markdown;
use crate::cli::commands::tui::transcript::{MessageRole, PartType, ToolStatus, TranscriptMessage};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering::Relaxed};

// ── Palette ───────────────────────────────────────────────────────────────────

const C_TEXT: Color = Color::Rgb(238, 238, 238);
const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
const C_TEXT_DIM: Color = Color::Rgb(80, 80, 80);
const C_PRIMARY: Color = Color::Rgb(250, 178, 131); // user badge
const C_ACCENT: Color = Color::Rgb(157, 124, 216);  // assistant badge
const C_ERROR: Color = Color::Rgb(224, 108, 117);
const C_BG: Color = Color::Rgb(10, 10, 10);

// ── Per-cell cache ────────────────────────────────────────────────────────────

struct CachedCell {
    revision: u64,
    last_width: u16,
    lines: Vec<Line<'static>>,
}

impl CachedCell {
    fn empty() -> Self {
        Self { revision: 0, last_width: 0, lines: Vec::new() }
    }
}

// ── Component ─────────────────────────────────────────────────────────────────

pub struct MessageList {
    id: ComponentId,
    messages: Vec<TranscriptMessage>,
    revisions: Vec<u64>,
    cache: Vec<CachedCell>,
    scroll_offset: AtomicU16,
    last_total_lines: AtomicU16,
    last_visible_height: AtomicU16,
    follow_bottom: AtomicBool,
    focused: bool,
}

impl MessageList {
    pub fn new(messages: Vec<TranscriptMessage>) -> Self {
        let n = messages.len();
        let revisions = vec![1u64; n];
        let cache = (0..n).map(|_| CachedCell::empty()).collect();
        Self {
            id: ComponentId::new(),
            messages,
            revisions,
            cache,
            scroll_offset: AtomicU16::new(0),
            last_total_lines: AtomicU16::new(0),
            last_visible_height: AtomicU16::new(1),
            follow_bottom: AtomicBool::new(true),
            focused: false,
        }
    }

    pub fn len(&self) -> usize { self.messages.len() }
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.revisions.clear();
        self.cache.clear();
        self.scroll_offset.store(0, Relaxed);
        self.follow_bottom.store(true, Relaxed);
    }

    pub fn push(&mut self, msg: TranscriptMessage) {
        self.messages.push(msg);
        self.revisions.push(1);
        self.cache.push(CachedCell::empty());
        self.follow_bottom.store(true, Relaxed);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.follow_bottom.store(true, Relaxed);
    }

    /// Append text to the last message's first non-synthetic text part, bump revision.
    pub fn append_last_text(&mut self, text: &str) {
        if let Some(msg) = self.messages.last_mut() {
            for part in &mut msg.parts {
                if let PartType::Text { text: t, synthetic } = part {
                    if !*synthetic {
                        t.push_str(text);
                        if let Some(rev) = self.revisions.last_mut() { *rev += 1; }
                        return;
                    }
                }
            }
            msg.parts.push(PartType::Text { text: text.to_string(), synthetic: false });
            if let Some(rev) = self.revisions.last_mut() { *rev += 1; }
        }
    }

    /// Update the duration of the last assistant message, bump revision.
    pub fn update_last_duration(&mut self, duration_ms: Option<u64>) {
        if let Some(msg) = self.messages.last_mut() {
            if let MessageRole::Assistant { duration_ms: ref mut d, .. } = msg.role {
                *d = duration_ms;
                if let Some(rev) = self.revisions.last_mut() { *rev += 1; }
            }
        }
    }

    // ── Scroll helpers ────────────────────────────────────────────────────

    fn scroll_up_by(&self, n: u16) {
        self.follow_bottom.store(false, Relaxed);
        let cur = self.scroll_offset.load(Relaxed);
        self.scroll_offset.store(cur.saturating_sub(n), Relaxed);
    }

    fn scroll_down_by(&self, n: u16) {
        self.follow_bottom.store(false, Relaxed);
        let total = self.last_total_lines.load(Relaxed);
        let vis = self.last_visible_height.load(Relaxed);
        let max = total.saturating_sub(vis);
        let cur = self.scroll_offset.load(Relaxed);
        self.scroll_offset.store((cur + n).min(max), Relaxed);
    }

    // ── Cell rendering ────────────────────────────────────────────────────

    /// Render a single message cell into `Vec<Line>`, using or populating cache.
    fn render_cell(&self, idx: usize, width: u16) -> &[Line<'static>] {
        let rev = self.revisions[idx];
        let cell = &self.cache[idx];

        // SAFETY: we only mutate cache through interior mutability logic below.
        // We use a raw pointer trick to get mutable access while holding &self.
        // This is safe because render() is never called concurrently (single-thread TUI).
        let cell_mut: &mut CachedCell = unsafe {
            let ptr = self.cache.as_ptr().add(idx) as *mut CachedCell;
            &mut *ptr
        };

        if cell.revision == rev && cell.last_width == width {
            return &cell_mut.lines;
        }

        // Re-render
        let lines = build_cell_lines(&self.messages[idx], width);
        cell_mut.revision = rev;
        cell_mut.last_width = width;
        cell_mut.lines = lines;
        &cell_mut.lines
    }
}

impl Default for MessageList {
    fn default() -> Self { Self::new(Vec::new()) }
}

// ── Cell line builder ─────────────────────────────────────────────────────────

fn build_cell_lines(msg: &TranscriptMessage, width: u16) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let w = width as usize;

    // Role badge line
    match &msg.role {
        MessageRole::User => {
            lines.push(Line::from(vec![
                Span::styled("you", Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD)),
            ]));
        }
        MessageRole::Assistant { agent, model_id, duration_ms } => {
            let mut spans = vec![
                Span::styled(agent.clone(), Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            ];
            if !model_id.is_empty() && model_id != "internal" {
                spans.push(Span::styled(
                    format!(" · {}", shorten_model(model_id)),
                    Style::default().fg(C_TEXT_MUTED),
                ));
            }
            if let Some(ms) = duration_ms {
                spans.push(Span::styled(
                    format!("  {:.1}s", *ms as f64 / 1000.0),
                    Style::default().fg(C_TEXT_DIM),
                ));
            }
            lines.push(Line::from(spans));
        }
    }

    lines.push(Line::from("")); // blank between badge and body

    // Body — render each part
    for part in &msg.parts {
        match part {
            PartType::Text { text, synthetic } if !synthetic => {
                if text.trim().is_empty() { continue; }
                let content_w = (w as u16).saturating_sub(2);
                let rendered = markdown::render_markdown(text, content_w);
                for line in rendered {
                    // 2-column indent for body
                    let mut indented = vec![Span::raw("  ")];
                    indented.extend(line.spans);
                    lines.push(Line::from(indented));
                }
            }
            PartType::Reasoning { text } => {
                lines.push(Line::from(vec![
                    Span::styled("  Thinking", Style::default().fg(C_TEXT_DIM).add_modifier(Modifier::ITALIC)),
                ]));
                if !text.trim().is_empty() {
                    let content_w = (w as u16).saturating_sub(4);
                    for line in markdown::render_markdown(text, content_w) {
                        let mut indented = vec![Span::raw("    ")];
                        indented.extend(line.spans);
                        lines.push(Line::from(indented));
                    }
                }
                lines.push(Line::from(""));
            }
            PartType::Tool { name, status, input, output, error } => {
                let (status_str, status_color) = match status {
                    ToolStatus::Pending  => ("○ pending", C_TEXT_DIM),
                    ToolStatus::Running  => ("◌ running", C_ACCENT),
                    ToolStatus::Completed=> ("● done",    C_TEXT_MUTED),
                    ToolStatus::Error    => ("✕ error",   C_ERROR),
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(name.clone(), Style::default().fg(C_TEXT_MUTED).add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(status_str, Style::default().fg(status_color)),
                ]));
                if matches!(status, ToolStatus::Error) {
                    if let Some(e) = error {
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(e.clone(), Style::default().fg(C_ERROR)),
                        ]));
                    }
                }
                let _ = (input, output); // shown in tool name only for brevity
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    lines
}

fn shorten_model(s: &str) -> String {
    // e.g. "claude-3-5-sonnet-20241022" → "claude-3.5-sonnet"
    let s = s.replace("claude-", "cl-");
    if s.len() > 18 { s[..18].to_string() } else { s }
}

// ── Component impl ────────────────────────────────────────────────────────────

impl Component for MessageList {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, f: &mut Frame, area: Rect) {
        // Reserve 2-col left/right padding
        let inner = Rect {
            x: area.x + 2,
            y: area.y,
            width: area.width.saturating_sub(4),
            height: area.height,
        };

        if self.messages.is_empty() {
            let y_mid = inner.y + inner.height / 2;
            let placeholder = "Start a conversation — type below";
            let x_off = (inner.width as usize).saturating_sub(placeholder.len()) / 2;
            let target = Rect { x: inner.x + x_off as u16, y: y_mid, width: inner.width, height: 1 };
            f.render_widget(
                Paragraph::new(Span::styled(placeholder, Style::default().fg(C_TEXT_DIM))),
                target,
            );
            return;
        }

        // Collect all lines (reuse cache)
        let mut all_lines: Vec<Line<'static>> = Vec::new();
        let n = self.messages.len();
        for i in 0..n {
            let cell_lines = self.render_cell(i, inner.width);
            all_lines.extend_from_slice(cell_lines);
            // Blank line between messages (but not after the last one)
            if i + 1 < n {
                all_lines.push(Line::from(""));
            }
        }

        let total = all_lines.len() as u16;
        self.last_total_lines.store(total, Relaxed);
        self.last_visible_height.store(inner.height, Relaxed);

        let offset = if self.follow_bottom.load(Relaxed) {
            total.saturating_sub(inner.height)
        } else {
            self.scroll_offset.load(Relaxed).min(total.saturating_sub(inner.height))
        };
        self.scroll_offset.store(offset, Relaxed);

        f.render_widget(
            Paragraph::new(all_lines)
                .scroll((offset, 0))
                .wrap(Wrap { trim: false })
                .style(Style::default().bg(C_BG)),
            inner,
        );

        // Scroll position hint
        if total > inner.height && offset > 0 && inner.width > 6 {
            let pct = offset as u32 * 100 / total.max(1) as u32;
            let hint = format!(" {}% ", pct);
            let hx = inner.x + inner.width.saturating_sub(hint.len() as u16 + 1);
            let hint_rect = Rect::new(hx, inner.y, hint.len() as u16, 1);
            f.render_widget(
                Paragraph::new(Span::styled(hint, Style::default().fg(C_TEXT_DIM))),
                hint_rect,
            );
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.focused { return EventPropagation::Continue; }
        let vis = self.last_visible_height.load(Relaxed).max(1);
        match (event.code, event.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.scroll_up_by(1); EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.scroll_down_by(1); EventPropagation::Stop
            }
            (KeyCode::PageUp, _) | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.scroll_up_by(vis / 2); EventPropagation::Stop
            }
            (KeyCode::PageDown, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.scroll_down_by(vis / 2); EventPropagation::Stop
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

    fn update(&mut self, _: std::time::Duration) {}
    fn is_focusable(&self) -> bool { true }
    fn focused(&self) -> bool { self.focused }
    fn on_focus(&mut self) { self.focused = true; }
    fn on_blur(&mut self) { self.focused = false; }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn user_msg(text: &str) -> TranscriptMessage {
        TranscriptMessage {
            role: MessageRole::User,
            parts: vec![PartType::Text { text: text.to_string(), synthetic: false }],
        }
    }

    fn asst_msg(text: &str) -> TranscriptMessage {
        TranscriptMessage {
            role: MessageRole::Assistant {
                agent: "build".into(),
                model_id: "claude-3".into(),
                duration_ms: Some(1200),
            },
            parts: vec![PartType::Text { text: text.to_string(), synthetic: false }],
        }
    }

    #[test]
    fn new_empty() {
        assert!(MessageList::default().is_empty());
    }

    #[test]
    fn push_increments_len() {
        let mut ml = MessageList::default();
        ml.push(user_msg("hi"));
        assert_eq!(ml.len(), 1);
    }

    #[test]
    fn clear_resets() {
        let mut ml = MessageList::default();
        ml.push(user_msg("hi"));
        ml.clear();
        assert!(ml.is_empty());
    }

    #[test]
    fn append_text_bumps_revision() {
        let mut ml = MessageList::default();
        ml.push(asst_msg("hello"));
        let rev_before = ml.revisions[0];
        ml.append_last_text(" world");
        assert!(ml.revisions[0] > rev_before);
    }

    #[test]
    fn build_cell_lines_user() {
        let msg = user_msg("hello world");
        let lines = build_cell_lines(&msg, 80);
        let flat: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(flat.contains("you"));
        assert!(flat.contains("hello world"));
    }

    #[test]
    fn build_cell_lines_assistant_has_model() {
        let msg = asst_msg("reply");
        let lines = build_cell_lines(&msg, 80);
        let flat: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(flat.contains("build"));
    }

    #[test]
    fn follow_bottom_on_push() {
        let mut ml = MessageList::default();
        ml.follow_bottom.store(false, Relaxed);
        ml.push(user_msg("new"));
        assert!(ml.follow_bottom.load(Relaxed));
    }

    #[test]
    fn scroll_up_disables_follow() {
        let ml = MessageList::default();
        ml.last_total_lines.store(100, Relaxed);
        ml.last_visible_height.store(20, Relaxed);
        ml.scroll_offset.store(50, Relaxed);
        ml.scroll_up_by(5);
        assert!(!ml.follow_bottom.load(Relaxed));
        assert_eq!(ml.scroll_offset.load(Relaxed), 45);
    }

    #[test]
    fn component_ids_unique() {
        assert_ne!(MessageList::default().id(), MessageList::default().id());
    }
}
