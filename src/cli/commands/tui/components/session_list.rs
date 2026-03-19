//! Session List Component
//!
//! A component for displaying a list of sessions with search and navigation.
//! Supports filtering, pagination, and delete confirmation.

use super::{Component, ComponentEvent, ComponentId, EventPropagation};
use crate::cli::commands::tui::SessionInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List as TuiList, ListItem, ListState},
};
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

const SERVER_URL: &str = "http://127.0.0.1:4096";

#[derive(Debug, Serialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

/// Session list component for displaying and managing sessions.
///
/// Features:
/// - Search/filter sessions by title
/// - Navigation with wrap-around (next/prev)
/// - Page up/down for fast scrolling
/// - Delete confirmation with double-press
/// - Focus-aware styling
pub struct SessionList {
    id: ComponentId,
    sessions: Vec<SessionInfo>,
    state: ListState,
    visible: bool,
    focused: bool,
    search_query: String,
    filtered_sessions: Vec<SessionInfo>,
    delete_pending: Option<String>,
    client: Client,
    loading: bool,
    error_message: Option<String>,
}

impl SessionList {
    /// Create a new SessionList component.
    ///
    /// Initializes with empty sessions, hidden by default,
    /// and selection set to first item.
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            id: ComponentId::new(),
            sessions: Vec::new(),
            state,
            visible: false,
            focused: false,
            search_query: String::new(),
            filtered_sessions: Vec::new(),
            delete_pending: None,
            client: Client::new(),
            loading: false,
            error_message: None,
        }
    }

    /// Show the session list.
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the session list.
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if the session list is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the search query.
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Set the search query and update filtered sessions.
    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
        self.update_filtered();
    }

    /// Clear the search query.
    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_filtered();
    }

    /// Get all sessions.
    pub fn sessions(&self) -> &[SessionInfo] {
        &self.sessions
    }

    /// Set sessions and update filtered list.
    pub fn set_sessions(&mut self, sessions: Vec<SessionInfo>) {
        self.sessions = sessions;
        self.update_filtered();
    }

    /// Get filtered sessions.
    pub fn filtered_sessions(&self) -> &[SessionInfo] {
        &self.filtered_sessions
    }

    /// Get the currently selected session ID if any.
    pub fn selected_session_id(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|idx| self.filtered_sessions.get(idx).map(|s| s.id.clone()))
    }

    /// Get the currently selected session if any.
    pub fn selected_session(&self) -> Option<&SessionInfo> {
        self.state
            .selected()
            .and_then(|idx| self.filtered_sessions.get(idx))
    }

    /// Check if there's a pending delete confirmation.
    pub fn delete_pending(&self) -> Option<&String> {
        self.delete_pending.as_ref()
    }

    /// Set delete pending for a session ID.
    pub fn set_delete_pending(&mut self, session_id: impl Into<String>) {
        self.delete_pending = Some(session_id.into());
    }

    /// Clear delete pending state.
    pub fn clear_delete_pending(&mut self) {
        self.delete_pending = None;
    }

    /// Update filtered sessions based on search query.
    fn update_filtered(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_sessions = self.sessions.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_sessions = self
                .sessions
                .iter()
                .filter(|s| s.title.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
        // Reset selection if out of bounds
        if self.state.selected().unwrap_or(0) >= self.filtered_sessions.len()
            && !self.filtered_sessions.is_empty()
        {
            self.state.select(Some(0));
        } else if self.filtered_sessions.is_empty() {
            self.state.select(None);
        }
    }

    /// Select next item (wraps around to first).
    fn select_next(&mut self) {
        if self.filtered_sessions.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.filtered_sessions.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Select previous item (wraps around to last).
    fn select_prev(&mut self) {
        if self.filtered_sessions.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_sessions.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// Page up - move selection up by 5 items.
    fn page_up(&mut self) {
        if self.filtered_sessions.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let new_idx = current.saturating_sub(5);
        self.state.select(Some(new_idx));
    }

    /// Page down - move selection down by 5 items.
    fn page_down(&mut self) {
        if self.filtered_sessions.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let new_idx = (current + 5).min(self.filtered_sessions.len() - 1);
        self.state.select(Some(new_idx));
    }

    /// Go to first item.
    fn select_first(&mut self) {
        if !self.filtered_sessions.is_empty() {
            self.state.select(Some(0));
        }
    }

    /// Go to last item.
    fn select_last(&mut self) {
        if !self.filtered_sessions.is_empty() {
            self.state.select(Some(self.filtered_sessions.len() - 1));
        }
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn error_message(&self) -> Option<&String> {
        self.error_message.as_ref()
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub async fn load_sessions(&mut self) {
        self.loading = true;
        self.error_message = None;

        match self
            .client
            .get(format!("{}/session", SERVER_URL))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<SessionInfo>>().await {
                        Ok(sessions) => {
                            self.sessions = sessions;
                            self.update_filtered();
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Parse error: {}", e));
                        }
                    }
                } else {
                    self.error_message = Some(format!("HTTP error: {}", response.status()));
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Connection error: {}", e));
            }
        }

        self.loading = false;
    }

    pub async fn create_session(&mut self, title: Option<String>) -> Result<(), String> {
        self.loading = true;
        self.error_message = None;

        let request = CreateSessionRequest { title };

        let result = match self
            .client
            .post(format!("{}/session", SERVER_URL))
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("HTTP error: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Connection error: {}", e)),
        };

        self.loading = false;
        result
    }

    pub async fn delete_session(&mut self, session_id: &str) -> Result<(), String> {
        self.loading = true;
        self.error_message = None;

        let result = match self
            .client
            .delete(format!("{}/session/{}", SERVER_URL, session_id))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("HTTP error: {}", response.status()))
                }
            }
            Err(e) => Err(format!("Connection error: {}", e)),
        };

        self.loading = false;
        result
    }
}

impl Default for SessionList {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for SessionList {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Create block with title showing search query if present
        let title = if self.loading {
            "Sessions (Loading...)".to_string()
        } else if self.search_query.is_empty() {
            "Sessions".to_string()
        } else {
            format!("Sessions (search: {})", self.search_query)
        };

        let block = if self.focused {
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
        } else {
            Block::default().title(title).borders(Borders::ALL)
        };

        // Build list items from filtered sessions
        let items: Vec<ListItem> = self
            .filtered_sessions
            .iter()
            .map(|session| {
                let status_symbol = match session.status.as_str() {
                    "active" => "●",
                    "completed" => "✓",
                    "error" => "✗",
                    _ => "○",
                };
                let content = format!(
                    "{} {} | {} msgs | {}",
                    status_symbol, session.title, session.message_count, session.updated_at
                );
                ListItem::new(Line::from(Span::raw(content)))
            })
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
        if !self.visible || !self.focused {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            // Navigation
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.select_next();
                EventPropagation::Stop
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.select_prev();
                EventPropagation::Stop
            }
            (KeyCode::PageDown, _) => {
                self.page_down();
                EventPropagation::Stop
            }
            (KeyCode::PageUp, _) => {
                self.page_up();
                EventPropagation::Stop
            }
            (KeyCode::Home, _) | (KeyCode::Char('g'), _) => {
                self.select_first();
                EventPropagation::Stop
            }
            (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.select_last();
                EventPropagation::Stop
            }
            // Search input
            (KeyCode::Char(c), KeyModifiers::NONE) if c.is_alphabetic() || c.is_numeric() => {
                self.search_query.push(c);
                self.update_filtered();
                EventPropagation::Stop
            }
            (KeyCode::Backspace, _) if !self.search_query.is_empty() => {
                self.search_query.pop();
                self.update_filtered();
                EventPropagation::Stop
            }
            (KeyCode::Esc, _) if !self.search_query.is_empty() => {
                self.clear_search();
                EventPropagation::Stop
            }
            // Escape to hide
            (KeyCode::Esc, _) => {
                self.hide();
                EventPropagation::Stop
            }
            // Enter to select
            (KeyCode::Enter, _) => {
                // Selection is handled, propagate Stop
                EventPropagation::Stop
            }
            _ => EventPropagation::Continue,
        }
    }

    fn handle_event(&mut self, event: &ComponentEvent) -> EventPropagation {
        match event {
            ComponentEvent::Key(key) => self.handle_input(*key),
            _ => EventPropagation::Continue,
        }
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed
    }

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

    fn create_test_session(id: &str, title: &str, status: &str, count: u32) -> SessionInfo {
        SessionInfo {
            id: id.to_string(),
            title: title.to_string(),
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-02".to_string(),
            status: status.to_string(),
            message_count: count,
        }
    }

    #[test]
    fn test_session_list_new() {
        let list = SessionList::new();
        assert!(!list.is_visible());
        assert!(!list.focused());
        assert_eq!(list.sessions().len(), 0);
        assert_eq!(list.search_query(), "");
        assert!(list.delete_pending().is_none());
    }

    #[test]
    fn test_session_list_default() {
        let list: SessionList = Default::default();
        assert!(!list.is_visible());
    }

    #[test]
    fn test_session_list_show_hide() {
        let mut list = SessionList::new();
        assert!(!list.is_visible());

        list.show();
        assert!(list.is_visible());

        list.hide();
        assert!(!list.is_visible());
    }

    #[test]
    fn test_session_list_set_sessions() {
        let mut list = SessionList::new();
        let sessions = vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "completed", 10),
        ];

        list.set_sessions(sessions);
        assert_eq!(list.sessions().len(), 2);
        assert_eq!(list.filtered_sessions().len(), 2);
    }

    #[test]
    fn test_session_list_search_filtering() {
        let mut list = SessionList::new();
        let sessions = vec![
            create_test_session("1", "Alpha Session", "active", 5),
            create_test_session("2", "Beta Test", "completed", 10),
            create_test_session("3", "Alpha Beta", "active", 3),
        ];

        list.set_sessions(sessions);
        assert_eq!(list.filtered_sessions().len(), 3);

        // Filter by "Alpha"
        list.set_search_query("Alpha");
        assert_eq!(list.filtered_sessions().len(), 2);

        // Filter by "Beta"
        list.set_search_query("Beta");
        assert_eq!(list.filtered_sessions().len(), 2);

        // Clear search
        list.clear_search();
        assert_eq!(list.search_query(), "");
        assert_eq!(list.filtered_sessions().len(), 3);
    }

    #[test]
    fn test_session_list_search_case_insensitive() {
        let mut list = SessionList::new();
        let sessions = vec![create_test_session("1", "Test Session", "active", 5)];

        list.set_sessions(sessions);

        list.set_search_query("test");
        assert_eq!(list.filtered_sessions().len(), 1);

        list.set_search_query("TEST");
        assert_eq!(list.filtered_sessions().len(), 1);

        list.set_search_query("TeSt");
        assert_eq!(list.filtered_sessions().len(), 1);
    }

    #[test]
    fn test_session_list_selection_navigation() {
        let mut list = SessionList::new();
        let sessions = vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "completed", 10),
            create_test_session("3", "Session 3", "active", 3),
        ];

        list.set_sessions(sessions);
        list.show();

        // Initially at index 0
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));

        // Next
        list.select_next();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("2"));

        // Next
        list.select_next();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("3"));

        // Wrap around to first
        list.select_next();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));

        // Prev wraps to last
        list.select_prev();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("3"));
    }

    #[test]
    fn test_session_list_first_last() {
        let mut list = SessionList::new();
        let sessions = vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "completed", 10),
            create_test_session("3", "Session 3", "active", 3),
        ];

        list.set_sessions(sessions);
        list.select_last();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("3"));

        list.select_first();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));
    }

    #[test]
    fn test_session_list_pagination() {
        let mut list = SessionList::new();
        let sessions: Vec<SessionInfo> = (0..20)
            .map(|i| create_test_session(&i.to_string(), &format!("Session {}", i), "active", 5))
            .collect();

        list.set_sessions(sessions);

        // Page down
        list.page_down();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("5"));

        // Page down again
        list.page_down();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("10"));

        // Page up
        list.page_up();
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("5"));
    }

    #[test]
    fn test_session_list_delete_pending() {
        let mut list = SessionList::new();

        assert!(list.delete_pending().is_none());

        list.set_delete_pending("session-123");
        assert_eq!(list.delete_pending(), Some(&"session-123".to_string()));

        list.clear_delete_pending();
        assert!(list.delete_pending().is_none());
    }

    #[test]
    fn test_session_list_component_id() {
        let list1 = SessionList::new();
        let list2 = SessionList::new();
        assert_ne!(list1.id(), list2.id());
    }

    #[test]
    fn test_session_list_is_focusable() {
        let list = SessionList::new();
        assert!(list.is_focusable());
    }

    #[test]
    fn test_session_list_focus_blur() {
        let mut list = SessionList::new();
        assert!(!list.focused());

        list.on_focus();
        assert!(list.focused());

        list.on_blur();
        assert!(!list.focused());
    }

    #[test]
    fn test_session_list_empty_navigation() {
        let mut list = SessionList::new();
        // Should not panic on empty list
        list.select_next();
        list.select_prev();
        list.page_up();
        list.page_down();
        list.select_first();
        list.select_last();
    }

    #[test]
    fn test_session_list_filter_resets_selection() {
        let mut list = SessionList::new();
        let sessions = vec![
            create_test_session("1", "Alpha", "active", 5),
            create_test_session("2", "Beta", "completed", 10),
        ];

        list.set_sessions(sessions);
        list.select_last(); // Select index 1

        // Filter to only show "Alpha", selection should reset to 0
        list.set_search_query("Alpha");
        assert_eq!(list.filtered_sessions().len(), 1);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));
    }

    #[test]
    fn test_session_list_handle_input_navigation() {
        let mut list = SessionList::new();
        let sessions = vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "completed", 10),
        ];

        list.set_sessions(sessions);
        list.show();
        list.on_focus();

        // Test Down key
        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("2"));

        // Test Up key
        let event = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));
    }

    #[test]
    fn test_session_list_handle_input_not_visible() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        // Not visible

        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_session_list_handle_input_not_focused() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        // Not focused

        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_session_list_handle_input_escape() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        // Set search query
        list.set_search_query("test");
        assert_eq!(list.search_query(), "test");

        // First Escape clears search
        let event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.search_query(), "");
        assert!(list.is_visible());

        // Second Escape hides
        let event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert!(!list.is_visible());
    }

    #[test]
    fn test_session_list_handle_input_search() {
        let mut list = SessionList::new();
        list.set_sessions(vec![
            create_test_session("1", "Alpha", "active", 5),
            create_test_session("2", "Beta", "completed", 10),
        ]);
        list.show();
        list.on_focus();

        // Type 'a' to search
        let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.search_query(), "a");
        assert_eq!(list.filtered_sessions().len(), 2); // Both contain 'a'

        // Type 'l' to refine search
        let event = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.search_query(), "al");
        assert_eq!(list.filtered_sessions().len(), 1); // Only Alpha

        // Backspace removes character
        let event = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.search_query(), "a");
    }

    #[test]
    fn test_session_list_handle_input_page_keys() {
        let mut list = SessionList::new();
        let sessions: Vec<SessionInfo> = (0..10)
            .map(|i| create_test_session(&i.to_string(), &format!("Session {}", i), "active", 5))
            .collect();

        list.set_sessions(sessions);
        list.show();
        list.on_focus();

        // Page Down
        let event = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("5"));

        // Page Up
        let event = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("0"));
    }

    #[test]
    fn test_session_list_handle_input_home_end() {
        let mut list = SessionList::new();
        list.set_sessions(vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "completed", 10),
        ]);
        list.show();
        list.on_focus();

        // End key
        let event = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("2"));

        // Home key
        let event = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));
    }

    #[test]
    fn test_session_list_handle_event_key() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        let key_event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        let component_event = ComponentEvent::Key(key_event);

        assert_eq!(list.handle_event(&component_event), EventPropagation::Stop);
    }

    #[test]
    fn test_session_list_handle_event_non_key() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        let component_event = ComponentEvent::Tick;
        assert_eq!(
            list.handle_event(&component_event),
            EventPropagation::Continue
        );
    }

    #[test]
    fn test_session_list_selected_session_id() {
        let mut list = SessionList::new();
        list.set_sessions(vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "completed", 10),
        ]);

        assert_eq!(list.selected_session_id(), Some("1".to_string()));

        list.select_next();
        assert_eq!(list.selected_session_id(), Some("2".to_string()));
    }

    #[test]
    fn test_session_list_handle_input_enter() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
    }

    #[test]
    fn test_session_list_handle_input_unhandled_key() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        // A key that doesn't trigger search (e.g., Tab)
        let event = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Continue);
    }
}
