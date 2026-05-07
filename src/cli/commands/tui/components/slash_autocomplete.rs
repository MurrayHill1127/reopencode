//! Slash command autocomplete popup — shown when user types `/` in the input.
//!
//! Tab/Enter selects. Esc dismisses. Fuzzy prefix matching against known commands.

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::cli::commands::tui::palette::{
    BORDER as C_BORDER, PRIMARY as C_PRIMARY, SECONDARY as C_ACCENT, SURFACE as C_BG_ELEM,
    SURFACE_HI as C_SEL_BG, TEXT as C_TEXT, TEXT_MUTED as C_TEXT_MUTED,
};

/// Known slash commands with descriptions.
const COMMANDS: &[(&str, &str)] = &[
    ("/exit", "quit roc"),
    ("/quit", "quit roc"),
    ("/new", "create a new session"),
    ("/clear", "clear the current conversation"),
    ("/sessions", "toggle the sessions sidebar"),
    ("/help", "show help message"),
    ("/undo", "revert to last user message"),
    ("/redo", "restore undone messages"),
    ("/compact", "compact conversation context"),
    ("/copy", "copy last assistant message"),
];

pub struct SlashAutocomplete {
    id: ComponentId,
    pub visible: bool,
    selected: usize,
    matches: Vec<(&'static str, &'static str)>,
    filter: String,
}

impl SlashAutocomplete {
    pub fn new() -> Self {
        Self {
            id: ComponentId::new(),
            visible: false,
            selected: 0,
            matches: Vec::new(),
            filter: String::new(),
        }
    }

    /// Update the filter based on current input text.
    /// Returns true if the popup should be visible.
    pub fn update_filter(&mut self, input: &str) {
        // Only show autocomplete when input starts with / and has more chars
        self.visible = false;
        self.matches.clear();
        self.selected = 0;
        self.filter.clear();

        if !input.starts_with('/') { return; }
        let typed = input.trim();
        if typed == "/" { return; }
        if typed.contains(' ') {
            // After a space, it's a /command with arguments — don't show popup
            return;
        }

        // Find matches
        let lower = typed.to_lowercase();
        self.matches = COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.to_lowercase().starts_with(&lower))
            .copied()
            .collect();

        if self.matches.is_empty() { return; }
        self.visible = true;
        self.filter = typed.to_string();
        if self.selected >= self.matches.len() {
            self.selected = 0;
        }
    }

    /// Returns the completed text if Enter/Tab was pressed, or None.
    pub fn take_completion(&mut self) -> Option<&'static str> {
        if self.matches.is_empty() { return None; }
        let cmd = self.matches[self.selected].0;
        self.visible = false;
        self.matches.clear();
        Some(cmd)
    }

    /// Returns (x, y, width, height) for the popup relative to the composer area.
    pub fn popup_rect(&self, composer_area: Rect) -> Rect {
        let n = self.matches.len().max(1) as u16;
        let h = n + 3; // border top + items + border bottom
        let w = 32u16;
        Rect {
            x: composer_area.x + 3,
            y: composer_area.y.saturating_sub(h),
            width: w,
            height: h,
        }
    }
}

impl Default for SlashAutocomplete {
    fn default() -> Self { Self::new() }
}

impl Component for SlashAutocomplete {
    fn id(&self) -> ComponentId { self.id }

    fn render(&self, f: &mut Frame, area: Rect) {
        if !self.visible || self.matches.is_empty() { return; }
        let popup = self.popup_rect(area);

        // Dim background behind popup
        let bg = Block::default().style(Style::default().bg(C_BG_ELEM));
        f.render_widget(Clear, popup);
        f.render_widget(bg, popup);

        let block = Block::default()
            .title(" Commands ")
            .title_style(Style::default().fg(C_ACCENT))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(C_BORDER))
            .style(Style::default().bg(C_BG_ELEM));

        let inner = block.inner(popup);
        f.render_widget(block, popup);

        let mut rlines: Vec<Line<'static>> = Vec::new();
        for (i, (cmd, desc)) in self.matches.iter().enumerate() {
            let style = if i == self.selected {
                Style::default().fg(C_PRIMARY).bg(C_SEL_BG).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(C_TEXT)
            };
            rlines.push(Line::from(vec![
                Span::styled(format!(" {} ", cmd), style),
                Span::styled(desc.to_string(), Style::default().fg(C_TEXT_MUTED)),
            ]));
        }

        f.render_widget(Paragraph::new(rlines), inner);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.visible { return EventPropagation::Continue; }

        match event.code {
            KeyCode::Esc => {
                self.visible = false;
                self.matches.clear();
                EventPropagation::Stop
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 { self.selected -= 1; }
                EventPropagation::Stop
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.matches.len() { self.selected += 1; }
                EventPropagation::Stop
            }
            KeyCode::Tab | KeyCode::Enter => {
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn is_focusable(&self) -> bool { false }
    fn focused(&self) -> bool { false }
    fn on_focus(&mut self) {}
    fn on_blur(&mut self) {}
}
