//! Footer Component — DeepSeek-TUI inspired 1-line status bar.
//!
//! Layout:
//!
//! ```text
//! ~/Projects/repo          ⠿ Thinking…              Ready
//! ```
//!
//! Left: current working directory (muted, truncated to fit)
//! Centre: animated Braille spinner + label while streaming
//! Right: status text

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

const C_TEXT: Color = Color::Rgb(238, 238, 238);
const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
const C_TEXT_DIM: Color = Color::Rgb(80, 80, 80);
const C_PRIMARY: Color = Color::Rgb(250, 178, 131);
const C_ACCENT: Color = Color::Rgb(157, 124, 216);
const C_SUCCESS: Color = Color::Rgb(127, 216, 143);
const C_WARNING: Color = Color::Rgb(245, 167, 66);
const C_ERROR: Color = Color::Rgb(224, 108, 117);
const C_BG: Color = Color::Rgb(10, 10, 10);

// ── Spinner ───────────────────────────────────────────────────────────────────

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_MS: u64 = 80; // ms per frame

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
    /// Spinner frame index
    spinner_tick: u8,
    /// Accumulated ms for spinner timing
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

        // ── Right side: status ────────────────────────────────────────────
        let (status_str, status_color) = if self.streaming {
            ("Streaming".to_string(), C_ACCENT)
        } else if !self.is_connected() {
            ("Connecting…".to_string(), C_TEXT_DIM)
        } else if self.pending_permissions > 0 {
            (format!("△ {} pending", self.pending_permissions), C_WARNING)
        } else {
            (self.status.clone(), C_TEXT_MUTED)
        };
        let right_w = status_str.width();

        // ── Centre: spinner while streaming ──────────────────────────────
        let (centre_str, centre_color) = if self.streaming {
            let frame = SPINNER[self.spinner_tick as usize % SPINNER.len()];
            (format!(" {} Thinking…", frame), C_ACCENT)
        } else {
            // LSP/MCP indicators when connected
            let lsp_color = if self.lsp_count > 0 { C_SUCCESS } else { C_TEXT_DIM };
            let mcp_count = self.mcp_statuses.len();
            let all_ok = self.mcp_statuses.values().all(|s| matches!(s, McpStatus::Connected));
            let mcp_color = if mcp_count > 0 && all_ok { C_SUCCESS } else { C_TEXT_DIM };
            let _ = (lsp_color, mcp_color);
            (String::new(), C_TEXT_DIM)
        };
        let centre_w = centre_str.width();

        // ── Left side: directory ──────────────────────────────────────────
        let max_dir = total_w
            .saturating_sub(centre_w)
            .saturating_sub(right_w)
            .saturating_sub(4); // margins

        let dir = &self.directory;
        let dir_str: String = if dir.width() > max_dir && max_dir > 3 {
            let mut s = "…".to_string();
            let chars: Vec<char> = dir.chars().collect();
            let keep = chars.len().saturating_sub(max_dir.saturating_sub(1));
            s.push_str(&chars[keep..].iter().collect::<String>());
            s
        } else {
            dir.clone()
        };
        let left_w = dir_str.width();

        // ── Gap calculations ──────────────────────────────────────────────
        let used = left_w + centre_w + right_w;
        let gap_total = total_w.saturating_sub(used);
        let gap1 = if centre_w > 0 { gap_total / 2 } else { gap_total };
        let gap2 = gap_total.saturating_sub(gap1);

        // ── Assemble ──────────────────────────────────────────────────────
        let mut spans: Vec<Span<'static>> = Vec::new();

        spans.push(Span::styled(dir_str, Style::default().fg(C_TEXT_MUTED)));
        spans.push(Span::raw(" ".repeat(gap1)));

        if !centre_str.is_empty() {
            spans.push(Span::styled(centre_str, Style::default().fg(centre_color).add_modifier(Modifier::BOLD)));
        }

        spans.push(Span::raw(" ".repeat(gap2)));
        spans.push(Span::styled(status_str, Style::default().fg(status_color)));

        f.render_widget(
            Paragraph::new(Line::from(spans)).style(Style::default().bg(C_BG)),
            area,
        );
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
        // 9 frames × 80ms = 720ms > 1 tick
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
}
