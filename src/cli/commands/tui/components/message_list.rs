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
use crate::cli::commands::tui::tool_family::{ToolFamily, classify_tool};
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
const C_SUCCESS: Color = Color::Rgb(127, 216, 143);
const C_WARNING: Color = Color::Rgb(245, 167, 66);
const C_INFO: Color = Color::Rgb(86, 182, 194);
const C_REASONING_BG: Color = Color::Rgb(26, 22, 12); // warm amber tint
const C_DIFF_ADD: Color = Color::Rgb(103, 185, 115);
const C_DIFF_DEL: Color = Color::Rgb(220, 95, 105);
const C_TOOL_BORDER: Color = Color::Rgb(55, 55, 65);
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
    pub code_concealed: bool,
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
            code_concealed: false,
        }
    }

    pub fn len(&self) -> usize { self.messages.len() }
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }
    pub fn toggle_conceal(&mut self) {
        self.code_concealed = !self.code_concealed;
        for rev in &mut self.revisions { *rev += 1; }
    }

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
        let lines = build_cell_lines(&self.messages[idx], width, self.code_concealed);
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

fn build_cell_lines(msg: &TranscriptMessage, width: u16, conceal: bool) -> Vec<Line<'static>> {
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
                let rendered = if conceal {
                    markdown::render_markdown_concealed(text, content_w)
                } else {
                    markdown::render_markdown(text, content_w)
                };
                for line in rendered {
                    // 2-column indent for body
                    let mut indented = vec![Span::raw("  ")];
                    indented.extend(line.spans);
                    lines.push(Line::from(indented));
                }
            }
            PartType::Reasoning { text } => {
                lines.push(Line::from(vec![
                    Span::styled("  … thinking", Style::default().fg(C_WARNING).add_modifier(Modifier::BOLD)),
                ]));
                if !text.trim().is_empty() {
                    let content_w = (w as u16).saturating_sub(6);
                    for line in markdown::render_markdown(text, content_w) {
                        let mut indented = vec![
                            Span::styled("    ╎ ", Style::default().fg(C_TOOL_BORDER)),
                        ];
                        // Add warm background to the reasoning body
                        let warm_spans: Vec<Span<'static>> = line.spans.into_iter().map(|s| {
                            Span {
                                style: s.style.bg(C_REASONING_BG),
                                content: s.content,
                            }
                        }).collect();
                        indented.extend(warm_spans);
                        lines.push(Line::from(indented));
                    }
                }
                lines.push(Line::from(""));
            }
            PartType::Tool { name, status, input, output, error } => {
                render_tool_block(&mut lines, name, status, input.as_deref(), output.as_deref(), error.as_deref(), w);
            }
            _ => {}
        }
    }

    lines
}

fn shorten_model(s: &str) -> String {
    let s = s.replace("claude-", "cl-");
    if s.len() > 18 { s[..18].to_string() } else { s }
}

// ── Rich tool rendering ───────────────────────────────────────────────────────

fn render_tool_block(
    lines: &mut Vec<Line<'static>>,
    name: &str,
    status: &ToolStatus,
    input: Option<&str>,
    output: Option<&str>,
    error: Option<&str>,
    w: usize,
) {
    let family = classify_tool(name);
    let (status_str, status_color) = match status {
        ToolStatus::Pending  => ("○ pending", C_TEXT_DIM),
        ToolStatus::Running  => ("◌ running", C_ACCENT),
        ToolStatus::Completed=> ("● done",    C_TEXT_MUTED),
        ToolStatus::Error    => ("✕ error",   C_ERROR),
    };

    // Header: glyph + label + tool name + status
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(format!("{} {}", family.glyph(), family.label()), Style::default().fg(C_TEXT_MUTED).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(name.to_string(), Style::default().fg(C_INFO)),
        Span::raw("  "),
        Span::styled(status_str, Style::default().fg(status_color)),
    ]));

    // Error message
    if matches!(status, ToolStatus::Error) {
        if let Some(e) = error {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(e.to_string(), Style::default().fg(C_ERROR)),
            ]));
        }
    }

    // Specialized body rendering
    match family {
        ToolFamily::Patch => render_patch_output(lines, name, input, output, w),
        ToolFamily::Run => render_run_output(lines, input, output, w),
        ToolFamily::Find | ToolFamily::Read | ToolFamily::Web => {
            render_search_output(lines, output, w);
        }
        _ => {
            if let Some(out) = output {
                if !out.trim().is_empty() {
                    for chunk in wrap_text(out.trim(), w.saturating_sub(6)) {
                        lines.push(Line::from(vec![
                            Span::raw("    "),
                            Span::styled(chunk, Style::default().fg(C_TEXT_DIM)),
                        ]));
                    }
                }
            }
        }
    }

    lines.push(Line::from(""));
}

fn render_patch_output(
    lines: &mut Vec<Line<'static>>,
    _name: &str,
    input: Option<&str>,
    output: Option<&str>,
    w: usize,
) {
    let inner_w = w.saturating_sub(6);
    if inner_w < 10 { return; }

    if let Some(out) = output {
        let diff_lines: Vec<&str> = out.lines().collect();
        if diff_lines.is_empty() { return; }

        let file_label = extract_file(input, diff_lines.as_slice());
        let top = box_top(&file_label, inner_w);
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(top, Style::default().fg(C_TOOL_BORDER)),
        ]));

        for dline in &diff_lines {
            let (dl, color) = classify_diff_line(dline);
            let truncated = truncate_or_pad(&dl, inner_w);
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled("│ ", Style::default().fg(C_TOOL_BORDER)),
                Span::styled(truncated, Style::default().fg(color)),
            ]));
        }

        let bot = format!("└{}┘", "─".repeat(inner_w));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(bot, Style::default().fg(C_TOOL_BORDER)),
        ]));
    }
}

fn render_run_output(
    lines: &mut Vec<Line<'static>>,
    input: Option<&str>,
    output: Option<&str>,
    w: usize,
) {
    let inner_w = w.saturating_sub(6);
    if inner_w < 10 { return; }

    if let Some(inp) = input {
        let cmd = extract_command(inp);
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(format!("$ {}", truncate_or_pad(&cmd, inner_w)), Style::default().fg(C_PRIMARY)),
        ]));
    }

    if let Some(out) = output {
        if !out.trim().is_empty() {
            for line_str in out.lines().take(12) {
                let truncated = truncate_or_pad(line_str, inner_w);
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(truncated, Style::default().fg(C_TEXT_DIM)),
                ]));
            }
        }
    }
}

fn render_search_output(
    lines: &mut Vec<Line<'static>>,
    output: Option<&str>,
    w: usize,
) {
    let inner_w = w.saturating_sub(6);
    if inner_w < 10 { return; }

    if let Some(out) = output {
        if out.trim().is_empty() { return; }
        for rline in out.lines().take(10) {
            let truncated = truncate_or_pad(rline, inner_w);
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(truncated, Style::default().fg(C_TEXT_DIM)),
            ]));
        }
    }
}

// ── Diff helpers ──────────────────────────────────────────────────────────────

fn classify_diff_line(line: &str) -> (String, Color) {
    let display = line.to_string();
    if display.starts_with("+++") || display.starts_with("---") {
        (display, C_TEXT_MUTED)
    } else if display.starts_with("@@") {
        (display, C_INFO)
    } else if display.starts_with('+') && !display.starts_with("+++") {
        (display, C_DIFF_ADD)
    } else if display.starts_with('-') && !display.starts_with("---") {
        (display, C_DIFF_DEL)
    } else {
        (display, C_TEXT_DIM)
    }
}

fn extract_file(input: Option<&str>, diff_lines: &[&str]) -> String {
    for line in diff_lines {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            return rest.to_string();
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            return rest.to_string();
        }
    }
    if let Some(inp) = input {
        if let Some(path) = extract_json_field(inp, "path") {
            return path;
        }
        if let Some(path) = extract_json_field(inp, "file_path") {
            return path;
        }
    }
    String::new()
}

fn extract_command(input: &str) -> String {
    if let Some(cmd) = extract_json_field(input, "command") {
        return cmd;
    }
    if let Some(cmd) = extract_json_field(input, "cmd") {
        return cmd;
    }
    if input.len() > 80 { format!("{}…", &input[..80]) } else { input.to_string() }
}

fn extract_json_field(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\"", key);
    if let Some(start) = json.find(&search) {
        let after = &json[start + search.len()..];
        if let Some(val_start) = after.find('"') {
            let val = &after[val_start + 1..];
            if let Some(val_end) = val.find('"') {
                return Some(val[..val_end].to_string());
            }
        }
    }
    None
}

fn box_top(label: &str, w: usize) -> String {
    if label.is_empty() {
        format!("┌{}┐", "─".repeat(w))
    } else {
        let header = format!(" {} ", label);
        let dashes = w.saturating_sub(header.len());
        format!("┌{}{}┐", header, "─".repeat(dashes))
    }
}

fn truncate_or_pad(s: &str, w: usize) -> String {
    use unicode_width::UnicodeWidthStr;
    if s.width() > w {
        let mut out = String::new();
        for c in s.chars() {
            if out.width() + c.len_utf8() + 1 > w {
                out.push('…');
                break;
            }
            out.push(c);
        }
        out
    } else {
        format!("{:<width$}", s, width = w)
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    if width == 0 { return vec![text.to_string()]; }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;
    for word in text.split_whitespace() {
        let ww = word.width();
        if cur_w == 0 {
            cur.push_str(word);
            cur_w = ww;
        } else if cur_w + 1 + ww <= width {
            cur.push(' ');
            cur.push_str(word);
            cur_w += 1 + ww;
        } else {
            lines.push(cur.clone());
            cur = word.to_string();
            cur_w = ww;
        }
    }
    if !cur.is_empty() { lines.push(cur); }
    if lines.is_empty() { lines.push(String::new()); }
    lines
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
        let lines = build_cell_lines(&msg, 80, false);
        let flat: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(flat.contains("you"));
        assert!(flat.contains("hello world"));
    }

    #[test]
    fn build_cell_lines_assistant_has_model() {
        let msg = asst_msg("reply");
        let lines = build_cell_lines(&msg, 80, false);
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

    #[test]
    fn streaming_markdown_full_pipeline() {
        // Simulate real SSE streaming with sentinel-encoded chunks
        let original = "## Fix\n\nHere is **bold** and `code`.\n\n```rust\nfn main() {}\n```\n\n- item 1\n- item 2";

        // Server replaces \n with \x01 before SSE
        let safe = original.replace('\n', "\x01");

        // Server splits by spaces
        let chunks: Vec<String> = safe.split_inclusive(' ').map(|s| s.to_string()).collect();
        assert!(chunks.len() > 1, "should produce multiple chunks");

        // Create placeholder like send_message does
        let mut ml = MessageList::default();
        ml.push(asst_msg(""));

        // Simulate streaming: TUI receives chunks, decodes \x01 → \n
        for chunk in &chunks {
            let decoded = chunk.replace('\x01', "\n");
            ml.append_last_text(&decoded);
        }

        // Verify full text was reconstructed
        let full_text: String = ml.messages[0].parts.iter()
            .filter_map(|p| match p {
                PartType::Text { text, synthetic: false } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(full_text, original, "SSE round-trip must preserve text exactly");

        // Render
        let lines = build_cell_lines(&ml.messages[0], 80, false);
        let all_text: String = lines.iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
            .collect();

        // Verify markdown was rendered
        assert!(all_text.contains("Fix"), "should contain heading text");
        assert!(all_text.contains("bold"), "should contain bold text");
        assert!(all_text.contains("code"), "should contain inline code text");
        assert!(all_text.contains("fn main"), "should contain code block text");
        assert!(all_text.contains("item 1"), "should contain list text");

        // Verify bold styling
        let has_bold = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.style.add_modifier.contains(Modifier::BOLD) && s.content.contains("bold"))
        });
        assert!(has_bold, "**bold** should be rendered with BOLD modifier. Lines: {:?}",
            lines.iter().map(|l| l.spans.iter().map(|s| format!("{:?}:{:?}", s.content, s.style)).collect::<Vec<_>>()).collect::<Vec<_>>());

        // Verify inline code styling
        let has_code = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.style.fg == Some(C_INFO) && s.content.contains("code"))
        });
        assert!(has_code, "`code` should be rendered with C_INFO color");

        // Verify code block border
        let has_border = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.content.contains('╭'))
        });
        assert!(has_border, "code block should have border");

        // Verify list bullet
        let has_bullet = lines.iter().any(|l| {
            l.spans.iter().any(|s| s.content.contains('•'))
        });
        assert!(has_bullet, "list items should have bullet points");
    }
}
