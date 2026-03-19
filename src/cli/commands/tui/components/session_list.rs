//! Session List Component
//!
//! A component for displaying a list of sessions with search and navigation.
//! Supports filtering, pagination, delete confirmation, and date-based grouping.

use crate::cli::commands::tui::SessionInfo;
use chrono::{Local, NaiveDate};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List as TuiList, ListItem, ListState, Paragraph},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{Component, ComponentEvent, ComponentId, EventPropagation};

const SERVER_URL: &str = "http://127.0.0.1:4096";
const DELETE_CONFIRM_TIMEOUT_MS: u64 = 3000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DateCategory {
    Today,
    Yesterday,
    ThisWeek,
    Older,
}

impl DateCategory {
    pub fn from_date_str(date_str: &str) -> Self {
        let parsed_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.fZ"))
            .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S"))
            .or_else(|_| chrono::DateTime::parse_from_rfc3339(date_str).map(|dt| dt.date_naive()))
            .or_else(|_| chrono::DateTime::parse_from_str(date_str, "%+").map(|dt| dt.date_naive()))
            .ok();

        match parsed_date {
            Some(date) => {
                let today = Local::now().date_naive();
                let yesterday = today - chrono::Duration::days(1);
                let week_ago = today - chrono::Duration::days(7);

                if date == today {
                    DateCategory::Today
                } else if date == yesterday {
                    DateCategory::Yesterday
                } else if date >= week_ago {
                    DateCategory::ThisWeek
                } else {
                    DateCategory::Older
                }
            }
            None => DateCategory::Older,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            DateCategory::Today => "Today",
            DateCategory::Yesterday => "Yesterday",
            DateCategory::ThisWeek => "This Week",
            DateCategory::Older => "Older",
        }
    }

    pub fn all() -> [DateCategory; 4] {
        [
            DateCategory::Today,
            DateCategory::Yesterday,
            DateCategory::ThisWeek,
            DateCategory::Older,
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

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
    current_session_id: Option<String>,
    rename_session_id: Option<String>,
    delete_first_press: Option<Instant>,
    grouped_sessions: HashMap<DateCategory, Vec<SessionInfo>>,
}

impl SessionList {
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
            current_session_id: None,
            rename_session_id: None,
            delete_first_press: None,
            grouped_sessions: HashMap::new(),
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

    pub fn clear_delete_pending(&mut self) {
        self.delete_pending = None;
        self.delete_first_press = None;
    }

    pub fn set_current_session_id(&mut self, id: Option<String>) {
        self.current_session_id = id;
    }

    pub fn current_session_id(&self) -> Option<&String> {
        self.current_session_id.as_ref()
    }

    pub fn is_current_session(&self, session_id: &str) -> bool {
        self.current_session_id.as_deref() == Some(session_id)
    }

    pub fn rename_session_id(&self) -> Option<&String> {
        self.rename_session_id.as_ref()
    }

    pub fn set_rename_session(&mut self, session_id: Option<String>) {
        self.rename_session_id = session_id;
    }

    pub fn get_rename_request(&self) -> Option<(String, String)> {
        self.rename_session_id
            .as_ref()
            .map(|id| (id.clone(), String::new()))
    }

    pub fn clear_rename_session(&mut self) {
        self.rename_session_id = None;
    }

    pub fn handle_delete_press(&mut self) -> Option<String> {
        if let Some(instant) = self.delete_first_press {
            if instant.elapsed().as_millis() as u64 > DELETE_CONFIRM_TIMEOUT_MS {
                self.delete_first_press = Some(Instant::now());
                if let Some(session) = self.selected_session() {
                    self.delete_pending = Some(session.id.clone());
                }
                return None;
            }
            self.delete_first_press = None;
            let session_id = self.delete_pending.clone();
            self.delete_pending = None;
            return session_id;
        }

        self.delete_first_press = Some(Instant::now());
        if let Some(session) = self.selected_session() {
            self.delete_pending = Some(session.id.clone());
        }
        None
    }

    pub fn is_delete_confirm_pending(&self) -> bool {
        self.delete_first_press.is_some()
    }

    fn clear_delete_confirmation(&mut self) {
        self.delete_first_press = None;
        self.delete_pending = None;
    }

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
        self.filtered_sessions
            .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        self.group_by_date();
        if self.state.selected().unwrap_or(0) >= self.filtered_sessions.len()
            && !self.filtered_sessions.is_empty()
        {
            self.state.select(Some(0));
        } else if self.filtered_sessions.is_empty() {
            self.state.select(None);
        }
    }

    fn group_by_date(&mut self) {
        self.grouped_sessions.clear();
        for session in &self.filtered_sessions {
            let category = DateCategory::from_date_str(&session.updated_at);
            self.grouped_sessions
                .entry(category)
                .or_insert_with(Vec::new)
                .push(session.clone());
        }
    }

    pub fn grouped_sessions(&self) -> &HashMap<DateCategory, Vec<SessionInfo>> {
        &self.grouped_sessions
    }

    fn select_next(&mut self) {
        if self.filtered_sessions.is_empty() {
            return;
        }
        self.clear_delete_confirmation();
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

    fn select_prev(&mut self) {
        if self.filtered_sessions.is_empty() {
            return;
        }
        self.clear_delete_confirmation();
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

        let mut items: Vec<ListItem> = Vec::new();
        let selected_idx = self.state.selected().unwrap_or(0);

        for category in DateCategory::all() {
            if let Some(sessions) = self.grouped_sessions.get(&category) {
                if !sessions.is_empty() {
                    items.push(ListItem::new(Line::from(Span::styled(
                        format!("── {} ──", category.label()),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))));
                }

                for session in sessions {
                    let is_current = self.is_current_session(&session.id);
                    let is_delete_pending = self
                        .delete_pending
                        .as_ref()
                        .map(|id| id == &session.id)
                        .unwrap_or(false);

                    let status_symbol = if is_current {
                        "●"
                    } else {
                        match session.status.as_str() {
                            "active" => "○",
                            "completed" => "✓",
                            "error" => "✗",
                            _ => "○",
                        }
                    };

                    let content = format!(
                        "{} {} | {} msgs | {}",
                        status_symbol, session.title, session.message_count, session.updated_at
                    );

                    let style = if is_delete_pending {
                        Style::default()
                            .bg(Color::Red)
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else if is_current {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    items.push(ListItem::new(Line::from(Span::styled(content, style))));
                }
            }
        }

        if items.is_empty() {
            items.push(ListItem::new(Line::from(Span::styled(
                "No sessions found",
                Style::default().fg(Color::DarkGray),
            ))));
        }

        let list = TuiList::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, area, &mut self.state.clone());

        if self.delete_first_press.is_some() {
            let msg = "Press ctrl+d again to confirm delete (Esc to cancel)";
            let msg_area = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
            frame.render_widget(
                Paragraph::new(msg).style(Style::default().fg(Color::Red).bg(Color::Black)),
                msg_area,
            );
        }
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.visible || !self.focused {
            return EventPropagation::Continue;
        }

        match (event.code, event.modifiers) {
            (KeyCode::Down, _)
            | (KeyCode::Char('j'), KeyModifiers::CONTROL)
            | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.select_next();
                EventPropagation::Stop
            }
            (KeyCode::Up, _)
            | (KeyCode::Char('k'), KeyModifiers::CONTROL)
            | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
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
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.handle_delete_press();
                EventPropagation::Stop
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                if let Some(session) = self.selected_session() {
                    self.rename_session_id = Some(session.id.clone());
                }
                EventPropagation::Stop
            }
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
            (KeyCode::Esc, _) if self.delete_first_press.is_some() => {
                self.clear_delete_confirmation();
                EventPropagation::Stop
            }
            (KeyCode::Esc, _) => {
                self.hide();
                EventPropagation::Stop
            }
            (KeyCode::Enter, _) => EventPropagation::Stop,
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

        let event = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Continue);
    }

    #[test]
    fn test_date_category_from_date_str() {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
        let three_days_ago = (Local::now() - chrono::Duration::days(3)).format("%Y-%m-%d").to_string();
        let ten_days_ago = (Local::now() - chrono::Duration::days(10)).format("%Y-%m-%d").to_string();

        assert_eq!(DateCategory::from_date_str(&today), DateCategory::Today);
        assert_eq!(DateCategory::from_date_str(&yesterday), DateCategory::Yesterday);
        assert_eq!(DateCategory::from_date_str(&three_days_ago), DateCategory::ThisWeek);
        assert_eq!(DateCategory::from_date_str(&ten_days_ago), DateCategory::Older);
    }

    #[test]
    fn test_date_category_invalid_date() {
        assert_eq!(DateCategory::from_date_str("invalid"), DateCategory::Older);
        assert_eq!(DateCategory::from_date_str(""), DateCategory::Older);
    }

    #[test]
    fn test_date_category_label() {
        assert_eq!(DateCategory::Today.label(), "Today");
        assert_eq!(DateCategory::Yesterday.label(), "Yesterday");
        assert_eq!(DateCategory::ThisWeek.label(), "This Week");
        assert_eq!(DateCategory::Older.label(), "Older");
    }

    #[test]
    fn test_session_list_current_session_id() {
        let mut list = SessionList::new();
        assert!(list.current_session_id().is_none());

        list.set_current_session_id(Some("session-123".to_string()));
        assert_eq!(list.current_session_id(), Some(&"session-123".to_string()));
        assert!(list.is_current_session("session-123"));
        assert!(!list.is_current_session("session-456"));

        list.set_current_session_id(None);
        assert!(list.current_session_id().is_none());
    }

    #[test]
    fn test_session_list_rename_session() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        assert!(list.rename_session_id().is_none());

        let event = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.rename_session_id(), Some(&"1".to_string()));

        list.clear_rename_session();
        assert!(list.rename_session_id().is_none());
    }

    #[test]
    fn test_session_list_delete_confirmation_first_press() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        assert!(list.delete_pending().is_none());
        assert!(!list.is_delete_confirm_pending());

        let result = list.handle_delete_press();
        assert!(result.is_none());
        assert!(list.is_delete_confirm_pending());
        assert_eq!(list.delete_pending(), Some(&"1".to_string()));
    }

    #[test]
    fn test_session_list_delete_confirmation_esc_cancel() {
        let mut list = SessionList::new();
        list.set_sessions(vec![create_test_session("1", "Session 1", "active", 5)]);
        list.show();
        list.on_focus();

        let _ = list.handle_delete_press();
        assert!(list.is_delete_confirm_pending());

        let event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert!(!list.is_delete_confirm_pending());
        assert!(list.delete_pending().is_none());
    }

    #[test]
    fn test_session_list_ctrl_p_ctrl_n_navigation() {
        let mut list = SessionList::new();
        list.set_sessions(vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "active", 5),
            create_test_session("3", "Session 3", "active", 5),
        ]);
        list.show();
        list.on_focus();

        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));

        let event = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL);
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("2"));

        let event = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
        assert_eq!(list.handle_input(event), EventPropagation::Stop);
        assert_eq!(list.selected_session().map(|s| s.id.as_str()), Some("1"));
    }

    #[test]
    fn test_session_list_navigation_clears_delete_pending() {
        let mut list = SessionList::new();
        list.set_sessions(vec![
            create_test_session("1", "Session 1", "active", 5),
            create_test_session("2", "Session 2", "active", 5),
        ]);
        list.show();
        list.on_focus();

        let _ = list.handle_delete_press();
        assert!(list.is_delete_confirm_pending());

        list.select_next();
        assert!(!list.is_delete_confirm_pending());
    }

    #[test]
    fn test_session_list_grouped_sessions() {
        let mut list = SessionList::new();
        let today = Local::now().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now() - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();

        let sessions = vec![
            SessionInfo {
                id: "1".to_string(),
                title: "Today Session".to_string(),
                created_at: today.clone(),
                updated_at: today,
                status: "active".to_string(),
                message_count: 5,
            },
            SessionInfo {
                id: "2".to_string(),
                title: "Yesterday Session".to_string(),
                created_at: yesterday.clone(),
                updated_at: yesterday,
                status: "active".to_string(),
                message_count: 3,
            },
        ];

        list.set_sessions(sessions);
        let grouped = list.grouped_sessions();

        assert!(grouped.contains_key(&DateCategory::Today));
        assert!(grouped.contains_key(&DateCategory::Yesterday));
    }
}
