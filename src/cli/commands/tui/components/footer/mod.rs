//! Footer Component — 1-line status bar.
//!
//! ## Layout when idle
//!
//! ```text
//! ~/Projects/ROC   build·cl-3.5   I  0%   ^B ^P
//! ```
//!
//! Left: current working directory (muted, truncated)
//! Centre: agent · model (INFO color) — or Braille spinner while streaming
//! Right: mode indicator + token % + keybind hints

use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

use super::{Component, ComponentId, EventPropagation};
use crate::mcp::types::McpStatus;

// ── Palette ───────────────────────────────────────────────────────────────────

const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
const C_TEXT_DIM: Color = Color::Rgb(80, 80, 80);
const C_PRIMARY: Color = Color::Rgb(250, 178, 131);
const C_ACCENT: Color = Color::Rgb(157, 124, 216);
const C_SUCCESS: Color = Color::Rgb(127, 216, 143);
const C_WARNING: Color = Color::Rgb(245, 167, 66);
const C_INFO: Color = Color::Rgb(86, 182, 194);
const C_BG: Color = Color::Rgb(10, 10, 10);

// ── Spinner ───────────────────────────────────────────────────────────────────

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_MS: u64 = 80;

// ── Component ─────────────────────────────────────────────────────────────────

pub struct Footer {
    id: ComponentId,
    pub directory: String,
    pub status: String,
    pub streaming: bool,
    pub pending_permissions: usize,
    pub lsp_count: usize,
    pub mcp_statuses: HashMap<String, McpStatus>,
    pub session_id: Option<String>,
    // ── New fields for enriched status bar ────────────────────────────────
    pub agent: String,
    pub model_short: String,
    pub token_pct: u8,
    pub mode_slash: bool,
    spinner_tick: u8,
    spinner_accum: u64,
}

impl Footer {
    pub fn new() -> Self {
        let directory = std::env::current_dir()
            .ok()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                if let Some(home) = dirs::home_dir() {
                    let h = home.to_string_lossy();
                    if s.starts_with(&*h) {
                        return s.replacen(&*h, "~", 1);
                    }
                }
                s
            })
            .unwrap_or_else(|| ".".to_string());

        Self {
            id: ComponentId::new(),
            directory,
            status: "Ready".to_string(),
            streaming: false,
            pending_permissions: 0,
            lsp_count: 0,
            mcp_statuses: HashMap::new(),
            session_id: None,
            agent: String::new(),
            model_short: String::new(),
            token_pct: 0,
            mode_slash: false,
            spinner_tick: 0,
            spinner_accum: 0,
        }
    }

    pub fn set_directory(&mut self, d: String)             { self.directory = d; }
    pub fn set_status(&mut self, s: String)                { self.status = s; }
    pub fn set_streaming(&mut self, s: bool)               { self.streaming = s; }
    pub fn set_session_id(&mut self, id: Option<String>)   { self.session_id = id; }
    pub fn set_lsp_count(&mut self, n: usize)              { self.lsp_count = n; }
    pub fn set_pending_permissions(&mut self, n: usize)    { self.pending_permissions = n; }
    pub fn set_mcp_statuses(&mut self, m: HashMap<String, McpStatus>) { self.mcp_statuses = m; }
    pub fn set_agent(&mut self, a: String)                 { self.agent = a; }
    pub fn set_model_short(&mut self, m: String)           { self.model_short = m; }
    pub fn set_token_pct(&mut self, p: u8)                 { self.token_pct = p; }
    pub fn set_mode_slash(&mut self, s: bool)              { self.mode_slash = s; }

    pub fn directory(&self) -> &str { &self.directory }
    pub fn is_connected(&self) -> bool { self.session_id.is_some() }
}

impl Default for Footer {
    fn default() -> Self { Self::new() }
}

impl Component for Footer {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, f: &mut Frame, area: Rect) {
        let total_w = area.width as usize;
        if total_w == 0 { return; }

        if self.streaming {
            self.render_streaming(f, area, total_w);
        } else {
            self.render_idle(f, area, total_w);
        }
    }

    fn update(&mut self, delta: Duration) {
        if self.streaming {
            self.spinner_accum += delta.as_millis() as u64;
            while self.spinner_accum >= SPINNER_MS {
                self.spinner_accum -= SPINNER_MS;
                self.spinner_tick = self.spinner_tick.wrapping_add(1);
            }
        } else {
            self.spinner_tick = 0;
            self.spinner_accum = 0;
        }
    }

    fn handle_input(&mut self, _: KeyEvent) -> EventPropagation { EventPropagation::Continue }
    fn is_focusable(&self) -> bool { false }
    fn focused(&self) -> bool { false }
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}

impl Footer {
    fn render_streaming(&self, f: &mut Frame, area: Rect, total_w: usize) {
        let frame = SPINNER[self.spinner_tick as usize % SPINNER.len()];
        let centre_str = format!(" {} Thinking…", frame);
        let right_str = "Streaming";
        let centre_w = centre_str.width();
        let right_w = right_str.width();
        let max_dir = total_w.saturating_sub(centre_w + right_w + 4);
        let dir_str = truncate_left(&self.directory, max_dir);
        let left_w = dir_str.width();
        let used = left_w + centre_w + right_w;
        let gap = total_w.saturating_sub(used);
        let gap1 = gap / 2;
        let gap2 = gap.saturating_sub(gap1);
        let spans = vec![
            Span::styled(dir_str, Style::default().fg(C_TEXT_MUTED)),
            Span::raw(" ".repeat(gap1)),
            Span::styled(centre_str, Style::default().fg(C_ACCENT).add_modifier(Modifier::BOLD)),
            Span::raw(" ".repeat(gap2)),
            Span::styled(right_str, Style::default().fg(C_ACCENT)),
        ];
        f.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(C_BG)),
            area,
        );
    }

    fn render_idle(&self, f: &mut Frame, area: Rect, total_w: usize) {
        // ── Right: mode + token% + hints ─────────────────────────────────
        let (mode_char, mode_color) = if self.mode_slash {
            ("/", C_PRIMARY)
        } else {
            ("I", C_TEXT_DIM)
        };

        let right_str = if self.pending_permissions > 0 {
            format!("△ {} pending", self.pending_permissions)
        } else if !self.is_connected() {
            "Connecting…".to_string()
        } else {
            format!("{}  {}%  ^B ^P", mode_char, self.token_pct)
        };
        let right_color = if self.pending_permissions > 0 { C_WARNING }
            else if !self.is_connected() { C_TEXT_DIM }
            else { mode_color };
        let right_w = right_str.width();

        // ── Centre: agent · model ─────────────────────────────────────────
        let centre_str = if !self.agent.is_empty() && !self.model_short.is_empty() {
            format!(" {}·{}", self.agent, self.model_short)
        } else if !self.agent.is_empty() {
            format!(" {}", self.agent)
        } else {
            String::new()
        };
        let centre_w = centre_str.width();

        // ── Left: directory ───────────────────────────────────────────────
        let max_dir = total_w.saturating_sub(centre_w + right_w + 4);
        let dir_str = truncate_left(&self.directory, max_dir);
        let left_w = dir_str.width();

        // ── Gap math ──────────────────────────────────────────────────────
        let used = left_w + centre_w + right_w;
        let gap = total_w.saturating_sub(used);
        let gap1 = if centre_w > 0 { gap / 2 } else { gap };
        let gap2 = gap.saturating_sub(gap1);

        // ── Assemble ──────────────────────────────────────────────────────
        let mut spans: Vec<Span<'static>> = vec![
            Span::styled(dir_str, Style::default().fg(C_TEXT_MUTED)),
            Span::raw(" ".repeat(gap1)),
        ];
        if !centre_str.is_empty() {
            spans.push(Span::styled(centre_str, Style::default().fg(C_INFO)));
        }
        spans.push(Span::raw(" ".repeat(gap2)));
        spans.push(Span::styled(right_str, Style::default().fg(right_color)));

        f.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(C_BG)),
            area,
        );
    }
}

fn truncate_left(s: &str, max: usize) -> String {
    if s.width() <= max || max < 4 {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    let keep = chars.len().saturating_sub(max.saturating_sub(1));
    format!("…{}", chars[keep..].iter().collect::<String>())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_new_not_connected() {
        let f = Footer::new();
        assert!(!f.is_connected());
    }

    #[test]
    fn footer_connected_after_session_set() {
        let mut f = Footer::new();
        f.set_session_id(Some("abc".into()));
        assert!(f.is_connected());
    }

    #[test]
    fn spinner_advances_on_update() {
        let mut f = Footer::new();
        f.streaming = true;
        let tick_before = f.spinner_tick;
        f.update(Duration::from_millis(SPINNER_MS + 1));
        assert_ne!(f.spinner_tick, tick_before);
    }

    #[test]
    fn spinner_resets_when_not_streaming() {
        let mut f = Footer::new();
        f.streaming = true;
        f.spinner_tick = 5;
        f.streaming = false;
        f.update(Duration::from_millis(100));
        assert_eq!(f.spinner_tick, 0);
    }

    #[test]
    fn setters_work() {
        let mut f = Footer::new();
        f.set_agent("build".into());
        f.set_model_short("cl-3.5".into());
        f.set_token_pct(42);
        f.set_mode_slash(true);
        assert_eq!(f.agent, "build");
        assert_eq!(f.model_short, "cl-3.5");
        assert_eq!(f.token_pct, 42);
        assert!(f.mode_slash);
    }
}
