//! MessageList Component - Renders Markdown messages with syntax highlighting

use super::{Component, ComponentId, EventPropagation};
use crate::cli::commands::tui::transcript::MessageRole;
use crate::cli::commands::tui::transcript::{PartType, ToolStatus, TranscriptMessage};
use crate::cli::commands::tui::transcript_renderer::TranscriptRenderer;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List as TuiList, ListState},
};
use std::time::Duration;

pub struct MessageList {
    id: ComponentId,
    messages: Vec<TranscriptMessage>,
    state: ListState,
    title: String,
    focused: bool,
    renderer: TranscriptRenderer,
}

impl MessageList {
    pub fn new(messages: Vec<TranscriptMessage>) -> Self {
        let mut state = ListState::default();
        if !messages.is_empty() {
            state.select(Some(0));
        }
        Self {
            id: ComponentId::new(),
            messages,
            state,
            title: String::from("Messages"),
            focused: false,
            renderer: TranscriptRenderer::new(),
        }
    }

    pub fn with_title(messages: Vec<TranscriptMessage>, title: impl Into<String>) -> Self {
        let mut this = Self::new(messages);
        this.title = title.into();
        this
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn selected(&self) -> Option<&TranscriptMessage> {
        self.state.selected().and_then(|i| self.messages.get(i))
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn select(&mut self, index: usize) {
        if index < self.messages.len() {
            self.state.select(Some(index));
        } else if !self.messages.is_empty() {
            self.state.select(Some(self.messages.len() - 1));
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.state.select(None);
    }

    pub fn push(&mut self, message: TranscriptMessage) {
        self.messages.push(message);
        // Auto-scroll to bottom when new message is added
        if self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        if !self.messages.is_empty() {
            self.state.select(Some(self.messages.len() - 1));
        }
    }

    pub fn scroll_offset(&self) -> usize {
        self.state.offset()
    }

    pub fn next(&mut self) {
        if self.messages.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
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
        if self.messages.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.messages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn first(&mut self) {
        if !self.messages.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn last(&mut self) {
        if !self.messages.is_empty() {
            self.state.select(Some(self.messages.len() - 1));
        }
    }

    pub fn page_down(&mut self, visible: usize) {
        if self.messages.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        self.state
            .select(Some((current + visible).min(self.messages.len() - 1)));
    }

    pub fn page_up(&mut self, visible: usize) {
        if self.messages.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        self.state.select(Some(current.saturating_sub(visible)));
    }

    fn get_role_style(&self, role: &MessageRole) -> Style {
        match role {
            MessageRole::User => Style::default().fg(Color::Cyan),
            MessageRole::Assistant { .. } => Style::default().fg(Color::White),
        }
    }

    fn get_role_label(&self, role: &MessageRole) -> &'static str {
        match role {
            MessageRole::User => "User",
            MessageRole::Assistant { agent, .. } => {
                // Use agent name for assistant label
                match agent.as_str() {
                    "oracle" => "Oracle",
                    "build" => "Build",
                    "explore" => "Explore",
                    "librarian" => "Librarian",
                    _ => "Assistant",
                }
            }
        }
    }
}

impl Default for MessageList {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl Component for MessageList {
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

        // Convert messages to list items using TranscriptRenderer
        let items: Vec<ratatui::widgets::ListItem> = self
            .messages
            .iter()
            .map(|message| {
                // Render markdown content using TranscriptRenderer
                let markdown = self.format_message_to_markdown(message);
                let text = self.renderer.render(&markdown);
                
                // Create a line with role label and first line of content
                let role_label = self.get_role_label(&message.role);
                let role_style = self.get_role_style(&message.role);
                
                let prefix = format!("[{}] ", role_label);
                let first_line = text.lines.first().cloned().unwrap_or(Line::from(""));
                
                // Combine prefix with first line
                let styled_prefix = ratatui::text::Span::styled(prefix, role_style);
                let mut spans = vec![styled_prefix];
                spans.extend(first_line.spans);
                
                ratatui::widgets::ListItem::new(Line::from(spans))
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

impl MessageList {
    /// Convert a TranscriptMessage to a Markdown string for rendering
    fn format_message_to_markdown(&self, message: &TranscriptMessage) -> String {
        let mut result = String::new();
        
        // Add role header
        match &message.role {
            MessageRole::User => {
                result.push_str("## User\n\n");
            }
            MessageRole::Assistant {
                agent,
                model_id,
                duration_ms,
            } => {
                // Titlecase agent name
                let mut chars = agent.chars();
                let titlecase_agent = match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                };
                
                let duration_str = duration_ms.map(|ms| {
                    let seconds = ms as f64 / 1000.0;
                    format!(" · {:.1}s", seconds)
                }).unwrap_or_default();
                
                result.push_str(&format!(
                    "## Assistant ({} · {}{})\n\n",
                    titlecase_agent, model_id, duration_str
                ));
            }
        }
        
        // Format parts
        for part in &message.parts {
            match part {
                PartType::Text { text, synthetic } => {
                    if !synthetic {
                        result.push_str(text);
                        result.push_str("\n\n");
                    }
                }
                PartType::Reasoning { text } => {
                    result.push_str(&format!("_Thinking:_\n\n{}\n\n", text));
                }
                PartType::Tool {
                    name,
                    status,
                    input,
                    output,
                    error,
                } => {
                    result.push_str(&format!("**Tool: {}**\n", name));
                    
                    if let Some(inp) = input {
                        result.push_str(&format!("\n**Input:**\n```json\n{}\n```\n", inp));
                    }
                    
                    if *status == ToolStatus::Completed
                        && let Some(out) = output
                    {
                        result.push_str(&format!("\n**Output:**\n```\n{}\n```\n", out));
                    }
                    
                    if *status == ToolStatus::Error
                        && let Some(err) = error
                    {
                        result.push_str(&format!("\n**Error:**\n```\n{}\n```\n", err));
                    }
                    
                    result.push('\n');
                }
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_list_new() {
        let messages = vec![
            TranscriptMessage {
                role: MessageRole::User,
                parts: vec![],
            },
        ];
        let list = MessageList::new(messages);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_message_list_component_id() {
        let l1 = MessageList::default();
        let l2 = MessageList::default();
        assert_ne!(l1.id(), l2.id());
    }

    #[test]
    fn test_message_list_is_focusable() {
        let list = MessageList::default();
        assert!(list.is_focusable());
    }

    #[test]
    fn test_message_list_push() {
        let mut list = MessageList::default();
        assert!(list.is_empty());
        
        list.push(TranscriptMessage {
            role: MessageRole::User,
            parts: vec![],
        });
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_message_list_navigation() {
        let messages = vec![
            TranscriptMessage {
                role: MessageRole::User,
                parts: vec![],
            },
            TranscriptMessage {
                role: MessageRole::Assistant {
                    agent: "oracle".to_string(),
                    model_id: "test".to_string(),
                    duration_ms: None,
                },
                parts: vec![],
            },
            TranscriptMessage {
                role: MessageRole::User,
                parts: vec![],
            },
        ];
        let mut list = MessageList::new(messages);
        
        assert_eq!(list.selected_index(), Some(0));
        
        list.next();
        assert_eq!(list.selected_index(), Some(1));
        
        list.next();
        assert_eq!(list.selected_index(), Some(2));
        
        list.next(); // Wrap around
        assert_eq!(list.selected_index(), Some(0));
        
        list.prev();
        assert_eq!(list.selected_index(), Some(2));
        
        list.first();
        assert_eq!(list.selected_index(), Some(0));
        
        list.last();
        assert_eq!(list.selected_index(), Some(2));
    }

    #[test]
    fn test_message_list_scroll_to_bottom() {
        let mut list = MessageList::default();
        
        for _ in 0..5 {
            list.push(TranscriptMessage {
                role: MessageRole::User,
                parts: vec![],
            });
        }
        
        list.scroll_to_bottom();
        assert_eq!(list.selected_index(), Some(4));
    }

    #[test]
    fn test_message_list_clear() {
        let mut list = MessageList::default();
        list.push(TranscriptMessage {
            role: MessageRole::User,
            parts: vec![],
        });
        
        assert_eq!(list.len(), 1);
        list.clear();
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
        assert_eq!(list.selected_index(), None);
    }
}
