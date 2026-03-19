//! List Component - generic selectable list

use super::{Component, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List as TuiList, ListItem, ListState},
};
use std::fmt::Display;
use std::time::Duration;

pub struct List<T: Display> {
    id: ComponentId,
    items: Vec<T>,
    state: ListState,
    title: String,
    focused: bool,
    scrollable: bool,
}

impl<T: Display> List<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            id: ComponentId::new(),
            items,
            state,
            title: String::from("List"),
            focused: false,
            scrollable: true,
        }
    }

    pub fn with_title(items: Vec<T>, title: impl Into<String>) -> Self {
        let mut this = Self::new(items);
        this.title = title.into();
        this
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    pub fn selected(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }
    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.state.select(Some(index));
        } else if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }
    pub fn items(&self) -> &[T] {
        &self.items
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }
    pub fn is_scrollable(&self) -> bool {
        self.scrollable
    }
    pub fn set_scrollable(&mut self, scrollable: bool) {
        self.scrollable = scrollable;
    }
    pub fn clear(&mut self) {
        self.items.clear();
        self.state.select(None);
    }
    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }
    pub fn scroll_offset(&self) -> usize {
        self.state.offset()
    }
    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    pub fn first(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }
    pub fn last(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(self.items.len() - 1));
        }
    }
    pub fn page_down(&mut self, visible: usize) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        self.state
            .select(Some((current + visible).min(self.items.len() - 1)));
    }
    pub fn page_up(&mut self, visible: usize) {
        if self.items.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        self.state.select(Some(current.saturating_sub(visible)));
    }
}

impl<T: Display> Default for List<T> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<T: Display + Send + Sync + 'static> Component for List<T> {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        let block = if self.focused {
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
        } else {
            Block::default()
                .title(self.title.as_str())
                .borders(Borders::ALL)
        };

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| ListItem::new(Line::from(Span::raw(item.to_string()))))
            .collect();
        let list = TuiList::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, area, &mut self.state.clone());
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        if !self.focused {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.prev();
                EventPropagation::Stop
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.next();
                EventPropagation::Stop
            }
            (KeyCode::Home, _) | (KeyCode::Char('g'), _) => {
                self.first();
                EventPropagation::Stop
            }
            (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.last();
                EventPropagation::Stop
            }
            (KeyCode::PageUp, _) => {
                self.page_up(10);
                EventPropagation::Stop
            }
            (KeyCode::PageDown, _) => {
                self.page_down(10);
                EventPropagation::Stop
            }
            (KeyCode::Enter, _) => EventPropagation::Stop,
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {}
    fn is_focusable(&self) -> bool {
        true
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

    #[test]
    fn test_list_new() {
        let list = List::new(vec!["A", "B", "C"]);
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_list_component_id() {
        let l1: List<String> = List::empty();
        let l2: List<String> = List::empty();
        assert_ne!(l1.id(), l2.id());
    }

    #[test]
    fn test_list_is_focusable() {
        let list: List<String> = List::empty();
        assert!(list.is_focusable());
    }
}
