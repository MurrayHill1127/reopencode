//! Left sidebar — collapsible 32-col panel.
//!
//! ## Layout
//!
//! ```text
//!  Sessions               ^B
//!  ──────────────────────────
//!  ● current session title
//!    older session title
//!    another one
//!  ──────────────────────────
//!  build · cl-3.5-sonnet
//!  ░░░░░▓▓▓░░  28%
//! ```
//!
//! Toggle with Ctrl+B. Focus with Enter to navigate sessions.

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::UnicodeWidthStr;

// ── Palette ───────────────────────────────────────────────────────────────────

use crate::cli::commands::tui::palette::{
    BORDER as C_BORDER, INFO as C_INFO, PRIMARY as C_PRIMARY, SECONDARY as C_ACCENT,
    SUCCESS as C_SUCCESS, SURFACE as C_BG_PANEL, SURFACE_HI as C_SEL_BG,
    TEXT as C_TEXT, TEXT_DIM as C_TEXT_DIM, TEXT_MUTED as C_TEXT_MUTED, WARNING as C_WARNING,
};

const BAR_WIDTH: usize = 10;
const BAR_USED: char = '▓';
const BAR_EMPTY: char = '░';

// ── Data types ────────────────────────────────────────────────────────────────

struct SidebarSession {
    id: String,
    title: String,
}

// ── Component ─────────────────────────────────────────────────────────────────

pub struct LeftSidebar {
    id: ComponentId,
    visible: bool,
    focused: bool,
    sessions: Vec<SidebarSession>,
    current_id: Option<String>,
    selected_idx: usize,
    pending_select: Option<String>,
    // Context info displayed at bottom
    pub agent: String,
    pub model: String,
    pub token_pct: u8,
}

impl LeftSidebar {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            visible: false,
            focused: false,
            sessions: Vec::new(),
            current_id: None,
            selected_idx: 0,
            pending_select: None,
            agent: String::new(),
            model: String::new(),
            token_pct: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible { self.focused = false; }
    }
    pub fn show(&mut self)  { self.visible = true; }
    pub fn hide(&mut self)  { self.visible = false; self.focused = false; }
    pub fn is_visible(&self) -> bool { self.visible }
    pub fn is_focused(&self) -> bool { self.focused }

    pub fn set_sessions(&mut self, list: Vec<(String, String)>) {
        self.sessions = list.into_iter().map(|(id, title)| SidebarSession { id, title }).collect();
        self.selected_idx = self.selected_idx.min(self.sessions.len().saturating_sub(1));
    }

    pub fn set_current(&mut self, id: Option<String>) {
        self.current_id = id.clone();
        // Move selection to current session
        if let Some(cur) = &id {
            if let Some(pos) = self.sessions.iter().position(|s| &s.id == cur) {
                self.selected_idx = pos;
            }
        }
    }

    pub fn set_agent(&mut self, agent: String, model: String) {
        self.agent = agent;
        self.model = model;
    }

    pub fn set_token_pct(&mut self, pct: u8) { self.token_pct = pct; }

    /// Consume a pending session selection (called each frame by TuiApp).
    pub fn take_pending_select(&mut self) -> Option<String> {
        self.pending_select.take()
    }

    fn select_prev(&mut self) {
        if self.sessions.is_empty() { return; }
        if self.selected_idx > 0 { self.selected_idx -= 1; }
    }

    fn select_next(&mut self) {
        if self.sessions.is_empty() { return; }
        if self.selected_idx + 1 < self.sessions.len() { self.selected_idx += 1; }
    }

    // ── Rendering ─────────────────────────────────────────────────────────

    fn render_inner(&self, f: &mut Frame, inner: Rect) {
        let w = inner.width as usize;
        let mut lines: Vec<Line<'static>> = Vec::new();

        // ── Header: "Sessions" + hint ─────────────────────────────────────
        let hint = "^B";
        let title = "Sessions";
        let gap = w.saturating_sub(title.width() + hint.width() + 1);
        lines.push(Line::from(vec![
            Span::styled(title, Style::default().fg(C_TEXT_MUTED).add_modifier(Modifier::BOLD)),
            Span::raw(" ".repeat(gap.max(1))),
            Span::styled(hint, Style::default().fg(C_TEXT_DIM)),
        ]));

        // Rule
        lines.push(Line::from(Span::styled(
            "─".repeat(w),
            Style::default().fg(C_BORDER),
        )));

        // ── Session list ──────────────────────────────────────────────────
        if self.sessions.is_empty() {
            lines.push(Line::from(Span::styled("  (no sessions)", Style::default().fg(C_TEXT_DIM))));
        } else {
            for (i, sess) in self.sessions.iter().enumerate() {
                let is_current = self.current_id.as_deref() == Some(&sess.id);
                let is_selected = self.focused && i == self.selected_idx;

                let bullet = if is_current { "● " } else { "  " };
                let max_title = w.saturating_sub(bullet.width() + 1);
                let title_str = truncate_str(&sess.title, max_title);

                let (bullet_style, title_style) = if is_selected {
                    (
                        Style::default().fg(C_PRIMARY).bg(C_SEL_BG),
                        Style::default().fg(C_TEXT).bg(C_SEL_BG).add_modifier(Modifier::BOLD),
                    )
                } else if is_current {
                    (
                        Style::default().fg(C_SUCCESS),
                        Style::default().fg(C_TEXT).add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default().fg(C_TEXT_DIM),
                        Style::default().fg(C_TEXT_MUTED),
                    )
                };

                let padding = w.saturating_sub(bullet.width() + title_str.width());
                let mut row_spans = vec![
                    Span::styled(bullet, bullet_style),
                    Span::styled(title_str, title_style),
                ];
                if padding > 0 && is_selected {
                    row_spans.push(Span::styled(" ".repeat(padding), Style::default().bg(C_SEL_BG)));
                }
                lines.push(Line::from(row_spans));
            }
        }

        // Rule
        lines.push(Line::from(Span::styled("─".repeat(w), Style::default().fg(C_BORDER))));

        // ── Context section ───────────────────────────────────────────────
        if !self.agent.is_empty() || !self.model.is_empty() {
            let agent_model = if !self.model.is_empty() {
                format!("{} · {}", self.agent, self.model)
            } else {
                self.agent.clone()
            };
            lines.push(Line::from(Span::styled(
                truncate_str(&agent_model, w),
                Style::default().fg(C_INFO),
            )));
        }

        // Context utilisation bar
        let used = (self.token_pct as usize * BAR_WIDTH / 100).min(BAR_WIDTH);
        let bar: String = std::iter::repeat_n(BAR_USED, used)
            .chain(std::iter::repeat_n(BAR_EMPTY, BAR_WIDTH - used))
            .collect();
        let bar_color = if self.token_pct >= 95 { C_WARNING } else { C_PRIMARY };
        lines.push(Line::from(vec![
            Span::styled(bar, Style::default().fg(bar_color)),
            Span::styled(
                format!("  {}%", self.token_pct),
                Style::default().fg(C_TEXT_MUTED),
            ),
        ]));

        // Keyboard nav hint when focused
        if self.focused {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "j/k: nav  Enter: open  Esc: back",
                Style::default().fg(C_TEXT_DIM),
            )));
        }

        f.render_widget(
            Paragraph::new(lines).style(Style::default().bg(C_BG_PANEL)),
            inner,
        );
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.width() <= max { return s.to_string(); }
    let mut out = String::new();
    for c in s.chars() {
        if out.width() + c.len_utf8() + 1 > max { out.push('…'); break; }
        out.push(c);
    }
    out
}

impl Default for LeftSidebar {
    fn default() -> Self { Self::new() }
}

impl Component for LeftSidebar {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible || area.width < 4 { return; }

        let border_color = if self.focused { C_ACCENT } else { C_BORDER };
        let block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(C_BG_PANEL));

        let inner = block.inner(area);
        f.render_widget(block, area);
        self.render_inner(f, inner);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.visible || !self.focused { return EventPropagation::Continue; }

        match (event.code, event.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.select_prev();
                EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.select_next();
                EventPropagation::Stop
            }
            (KeyCode::Enter, _) => {
                if let Some(sess) = self.sessions.get(self.selected_idx) {
                    self.pending_select = Some(sess.id.clone());
                }
                self.focused = false;
                EventPropagation::Stop
            }
            (KeyCode::Esc, _) => {
                self.focused = false;
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _: std::time::Duration) {}
    fn is_focusable(&self) -> bool { true }
    fn focused(&self) -> bool { self.focused }
    fn on_focus(&mut self) { self.focused = true; }
    fn on_blur(&mut self)  { self.focused = false; }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_visibility() {
        let mut sb = LeftSidebar::new();
        assert!(!sb.is_visible());
        sb.toggle();
        assert!(sb.is_visible());
        sb.toggle();
        assert!(!sb.is_visible());
    }

    #[test]
    fn set_sessions_and_current() {
        let mut sb = LeftSidebar::new();
        sb.set_sessions(vec![
            ("id1".into(), "First".into()),
            ("id2".into(), "Second".into()),
        ]);
        sb.set_current(Some("id2".into()));
        assert_eq!(sb.selected_idx, 1);
        assert_eq!(sb.current_id.as_deref(), Some("id2"));
    }

    #[test]
    fn navigation_wraps_at_bounds() {
        let mut sb = LeftSidebar::new();
        sb.set_sessions(vec![("a".into(), "A".into()), ("b".into(), "B".into())]);
        sb.on_focus();
        sb.select_prev(); // already at 0, stays 0
        assert_eq!(sb.selected_idx, 0);
        sb.select_next();
        assert_eq!(sb.selected_idx, 1);
        sb.select_next(); // at end, stays 1
        assert_eq!(sb.selected_idx, 1);
    }

    #[test]
    fn take_pending_select_consumes() {
        let mut sb = LeftSidebar::new();
        sb.set_sessions(vec![("sess1".into(), "My Session".into())]);
        sb.on_focus();
        sb.pending_select = Some("sess1".into());
        let r = sb.take_pending_select();
        assert_eq!(r, Some("sess1".into()));
        assert!(sb.take_pending_select().is_none());
    }

    #[test]
    fn unique_ids() {
        assert_ne!(LeftSidebar::new().id(), LeftSidebar::new().id());
    }
}
