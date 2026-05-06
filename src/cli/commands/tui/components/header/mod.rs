//! Header Component — DeepSeek-TUI inspired 1-line header bar.
//!
//! Layout (single row):
//!
//! ```text
//! ◆ roc  Session Title  build·claude          ░░░░░▒▒▒  64%
//! ```
//!
//! Left: brand badge + session title + model chip
//! Right: context-utilization bar + percentage

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

use super::{Component, ComponentId, ContextInfo, EventPropagation};

/// Kept for backward-compatibility with re-exports in components/mod.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum HoverButton { #[default] None, Parent, Prev, Next }

// ── Palette ───────────────────────────────────────────────────────────────────

const C_BG_PANEL: Color = Color::Rgb(20, 20, 20);
const C_TEXT: Color = Color::Rgb(238, 238, 238);
const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
const C_TEXT_DIM: Color = Color::Rgb(80, 80, 80);
const C_PRIMARY: Color = Color::Rgb(250, 178, 131);
const C_ACCENT: Color = Color::Rgb(157, 124, 216);
const C_SUCCESS: Color = Color::Rgb(127, 216, 143);
const C_WARNING: Color = Color::Rgb(245, 167, 66);
const C_ERROR: Color = Color::Rgb(224, 108, 117);
const C_INFO: Color = Color::Rgb(86, 182, 194);

// ── Context bar ───────────────────────────────────────────────────────────────

const BAR_WIDTH: usize = 10;
const BAR_EMPTY: char = '░';
const BAR_USED: char = '▓';

fn context_bar_color(pct: u8) -> Color {
    if pct >= 95 { C_ERROR }
    else if pct >= 80 { C_WARNING }
    else { C_PRIMARY }
}

fn pct_color(pct: u8) -> Color {
    if pct >= 95 { C_ERROR }
    else if pct >= 80 { C_WARNING }
    else { C_TEXT_MUTED }
}

// ── Component ─────────────────────────────────────────────────────────────────

pub struct Header {
    id: ComponentId,
    /// Short session title (truncated at render time)
    pub session_title: String,
    /// Active model name
    pub model: String,
    /// 0-100 context utilisation percentage
    pub context_pct: u8,
    /// Whether we're currently streaming
    pub streaming: bool,
    /// Braille frame index for the streaming indicator
    spinner_tick: u8,
}

impl Header {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            session_title: String::new(),
            model: String::new(),
            context_pct: 0,
            streaming: false,
            spinner_tick: 0,
        }
    }

    // Setters used from mod.rs
    pub fn set_session_title(&mut self, t: impl Into<String>) { self.session_title = t.into(); }
    pub fn set_model(&mut self, m: impl Into<String>)         { self.model = m.into(); }
    pub fn set_context_pct(&mut self, pct: u8)               { self.context_pct = pct; }
    pub fn set_streaming(&mut self, s: bool)                  { self.streaming = s; }

    /// Build the header line and render it.
    fn render_line(&self, f: &mut Frame, area: Rect) {
        let total_w = area.width as usize;
        if total_w < 10 { return; }

        // ── Right side: context bar ──────────────────────────────────────
        let used_cols = (self.context_pct as usize * BAR_WIDTH / 100).min(BAR_WIDTH);
        let bar: String = std::iter::repeat_n(BAR_USED, used_cols)
            .chain(std::iter::repeat_n(BAR_EMPTY, BAR_WIDTH - used_cols))
            .collect();
        let pct_str = format!(" {}%", self.context_pct);
        let right_str = format!("{}{}", bar, pct_str);
        let right_w = right_str.width();

        // ── Left side spans ───────────────────────────────────────────────
        // Badge: ◆ roc
        let badge_dot = Span::styled("◆ ", Style::default().fg(C_SUCCESS));
        let badge_txt = Span::styled("roc", Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD));
        let sep = Span::styled("  ", Style::default());

        // Session title — truncated to fit
        let budget = total_w
            .saturating_sub("◆ roc  ".width())
            .saturating_sub(2 + self.model.width() + 4)
            .saturating_sub(right_w + 2);
        let title_str: String = if self.session_title.is_empty() {
            "New Session".to_string()
        } else {
            let t = &self.session_title;
            if t.width() > budget {
                let mut s = String::new();
                for c in t.chars() {
                    if s.width() + c.len_utf8() + 1 > budget { s.push('…'); break; }
                    s.push(c);
                }
                s
            } else {
                t.clone()
            }
        };
        let title_span = Span::styled(title_str, Style::default().fg(C_TEXT));

        // Model chip
        let model_str = if self.model.is_empty() { String::new() } else {
            format!("  {}", self.model)
        };
        let model_span = Span::styled(model_str, Style::default().fg(C_INFO));

        // Streaming indicator
        const SPINNER: &[&str] = &["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];
        let stream_span = if self.streaming {
            let s = SPINNER[self.spinner_tick as usize % SPINNER.len()];
            Span::styled(format!("  {}", s), Style::default().fg(C_ACCENT))
        } else {
            Span::raw("")
        };

        // ── Compute gap ───────────────────────────────────────────────────
        let left_w: usize = "◆ roc".width()
            + 2
            + {
                let s = if self.session_title.is_empty() { "New Session" } else { &self.session_title };
                s.width().min(budget)
            }
            + model_str_width(&self.model)
            + if self.streaming { 3 } else { 0 };

        let gap = total_w.saturating_sub(left_w + right_w).max(1);

        // ── Assemble line ─────────────────────────────────────────────────
        let mut spans: Vec<Span<'static>> = vec![
            badge_dot,
            badge_txt,
            sep,
            title_span,
            model_span,
            stream_span,
            Span::raw(" ".repeat(gap)),
        ];

        // Context bar (bar part)
        let bar_color = context_bar_color(self.context_pct);
        spans.push(Span::styled(bar, Style::default().fg(bar_color)));
        // Percentage
        spans.push(Span::styled(pct_str, Style::default().fg(pct_color(self.context_pct))));

        let line = Line::from(spans);
        f.render_widget(
            Paragraph::new(line).style(Style::default().bg(C_BG_PANEL)),
            area,
        );
    }
}

fn model_str_width(model: &str) -> usize {
    if model.is_empty() { 0 } else { model.width() + 2 }
}

impl Default for Header {
    fn default() -> Self { Self::new() }
}

impl Component for Header {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, f: &mut Frame, area: Rect) {
        self.render_line(f, area);
    }

    fn update(&mut self, delta: Duration) {
        if self.streaming {
            // advance spinner at ~10 fps (100ms per tick)
            // delta is typically ~33ms (30fps render); accumulate 3 frames
            static mut ACCUM: u64 = 0;
            let ms = delta.as_millis() as u64;
            // SAFETY: single-threaded TUI loop
            unsafe {
                ACCUM += ms;
                if ACCUM >= 100 {
                    ACCUM = 0;
                    self.spinner_tick = self.spinner_tick.wrapping_add(1);
                }
            }
        } else {
            self.spinner_tick = 0;
        }
    }

    fn handle_input(&mut self, _: KeyEvent) -> EventPropagation {
        EventPropagation::Continue
    }

    fn is_focusable(&self) -> bool { false }

    fn focused(&self) -> bool { false }
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_default_values() {
        let h = Header::new();
        assert!(h.session_title.is_empty());
        assert_eq!(h.context_pct, 0);
        assert!(!h.streaming);
    }

    #[test]
    fn context_bar_colors() {
        assert_eq!(context_bar_color(0),  C_PRIMARY);
        assert_eq!(context_bar_color(79), C_PRIMARY);
        assert_eq!(context_bar_color(80), C_WARNING);
        assert_eq!(context_bar_color(95), C_ERROR);
    }

    #[test]
    fn context_bar_filled_at_100() {
        let used = 100usize * BAR_WIDTH / 100;
        assert_eq!(used, BAR_WIDTH);
    }
}
