//! TUI command - Terminal User Interface
//!
//! This module provides the Terminal User Interface for ReOpenCode,
//! with integrated Bus event system for toast notifications, prompt manipulation,
//! command execution, and session management.

// pub mod components;
pub mod events;
// pub mod keybindings;

pub use events::TuiEventSubscriber;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event as KeyEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;

use crate::bus::{Bus, Event as BusEvent};
use crate::bus::{
    TuiCommandExecuteProperties, TuiPromptAppendProperties, TuiSessionSelectProperties,
    TuiToastShowProperties,
};
use crate::cli::commands::clipboard::Clipboard;
use tokio::sync::RwLock;

const SERVER_URL: &str = "http://127.0.0.1:4096";

/// Session info from server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub message_count: u32,
}

/// Create session request
#[derive(Debug, Serialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

/// Send message request
#[derive(Debug, Serialize)]
pub struct SendMessageRequest {
    pub content: String,
}

/// Toast entry with expiry timestamp
type ToastEntry = (usize, u64); // (message_index, expires_at_ms)

/// TUI application state
pub struct TuiApp {
    pub running: bool,
    pub input: String,
    pub messages: Vec<String>,
    pub files: Vec<String>,
    pub client: Client,
    pub session_id: Option<String>,
    pub status: String,
    // Bus integration fields
    pub bus: Arc<Bus>,
    pub event_rx: tokio::sync::broadcast::Receiver<BusEvent>,
    pub toasts: Vec<ToastEntry>,
}

impl Default for TuiApp {
    fn default() -> Self {
        let bus = Arc::new(Bus::new("tui"));
        let event_rx = bus.subscribe_channel();

        Self {
            running: true,
            input: String::new(),
            messages: vec!["Welcome to ReOpenCode!".to_string()],
            files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            client: Client::new(),
            session_id: None,
            status: "Connecting...".to_string(),
            bus,
            event_rx,
            toasts: Vec::new(),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initialize session with server
    pub async fn init_session(&mut self) {
        self.status = "Connecting to server...".to_string();

        let request = CreateSessionRequest {
            title: Some("TUI Session".to_string()),
        };

        match self
            .client
            .post(format!("{}/session", SERVER_URL))
            .json(&request)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<SessionInfo>().await {
                        Ok(session) => {
                            self.session_id = Some(session.id.clone());
                            self.status = format!("Connected: {}", session.id);
                            self.messages
                                .push(format!("[System] Session created: {}", session.id));
                        }
                        Err(e) => {
                            self.status = format!("Parse error: {}", e);
                        }
                    }
                } else {
                    self.status = format!("Server error: {}", response.status());
                }
            }
            Err(e) => {
                self.status = format!("Connection failed: {}", e);
                self.messages
                    .push("[Error] Could not connect to server. Is it running?".to_string());
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match key {
            KeyCode::Char('q') if modifiers.is_empty() => self.running = false,
            KeyCode::Char('c')
                if modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) =>
            {
                if !self.input.is_empty() {
                    let _ = Clipboard::copy(&self.input);
                    self.status = "Copied to clipboard".to_string();
                }
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let msg = self.input.clone();
                    self.messages.push(format!("> {}", msg));
                    self.status = "Sending...".to_string();
                    self.input.clear();
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Char(c) if modifiers.is_empty() => {
                self.input.push(c);
            }
            _ => {}
        }
    }

    /// Send message to server (async, called from main loop)
    pub async fn send_message(&mut self, content: String) {
        if let Some(ref session_id) = self.session_id {
            let request = SendMessageRequest { content };

            match self
                .client
                .post(format!("{}/session/{}/message", SERVER_URL, session_id))
                .json(&request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text().await {
                            Ok(text) => {
                                self.messages.push(format!("[Response] {}", text));
                                self.status = "Ready".to_string();
                            }
                            Err(e) => {
                                self.messages.push(format!("[Error] Parse error: {}", e));
                            }
                        }
                    } else {
                        self.messages
                            .push(format!("[Error] Server returned: {}", response.status()));
                    }
                }
                Err(e) => {
                    self.messages.push(format!("[Error] Failed to send: {}", e));
                }
            }
        } else {
            self.messages.push("[Error] No active session".to_string());
        }
    }

    /// Process Bus events from the event channel
    /// Non-blocking: uses try_recv() to avoid stalling the main loop
    pub fn process_bus_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event.event_type.as_str() {
                "tui.toast.show" => {
                    if let Some(props) = event.properties::<TuiToastShowProperties>() {
                        let variant_prefix = match props.variant.as_deref() {
                            Some("error") => "[Error] ",
                            Some("warning") => "[Warning] ",
                            Some("success") => "[Success] ",
                            _ => "[Toast] ",
                        };

                        let toast_msg =
                            format!("{}{}: {}", variant_prefix, props.title, props.message);
                        let msg_index = self.messages.len();
                        self.messages.push(toast_msg);

                        // Store toast with expiry timestamp
                        if let Some(duration_ms) = props.duration {
                            let expires_at = get_current_timestamp_ms() + duration_ms;
                            self.toasts.push((msg_index, expires_at));
                        }
                    }
                }
                "tui.prompt.append" => {
                    if let Some(props) = event.properties::<TuiPromptAppendProperties>() {
                        self.input.push_str(&props.prompt);
                    }
                }
                "tui.command.execute" => {
                    if let Some(props) = event.properties::<TuiCommandExecuteProperties>() {
                        self.messages.push(format!("[Command] {}", props.command));
                    }
                }
                "tui.session.select" => {
                    if let Some(props) = event.properties::<TuiSessionSelectProperties>() {
                        self.session_id = Some(props.session_id.clone());
                        self.status = format!("Session: {}", props.session_id);
                    }
                }
                _ => {
                    // Ignore unknown event types
                }
            }
        }
    }

    /// Remove expired toasts from messages
    pub fn process_expired_toasts(&mut self) {
        let now = get_current_timestamp_ms();

        // Collect indices to remove (in reverse order to maintain indices)
        let mut to_remove: Vec<usize> = self
            .toasts
            .iter()
            .filter(|(_, expires)| *expires <= now)
            .map(|(idx, _)| *idx)
            .collect();

        to_remove.sort_by(|a, b| b.cmp(a)); // Reverse order

        for idx in to_remove {
            if idx < self.messages.len() {
                self.messages.remove(idx);
            }
        }

        // Remove processed toasts
        self.toasts.retain(|(_, expires)| *expires > now);
    }
}

/// Get current timestamp in milliseconds since UNIX_EPOCH
fn get_current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Run the TUI
pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new();
    app.init_session().await;

    // Set up Bus event subscriptions
    let app_shared = Arc::new(RwLock::new(app));

    // Create subscriber with the Bus and subscribe to events
    let bus = Arc::clone(&app_shared.read().await.bus);
    let subscriber = TuiEventSubscriber::new(bus);
    if let Err(e) = subscriber
        .subscribe_to_events(Arc::clone(&app_shared))
        .await
    {
        eprintln!("Warning: Failed to subscribe to events: {}", e);
    }

    // Run the main loop with shared app state
    let result = run_app_shared(&mut terminal, app_shared).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app_shared<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: Arc<RwLock<TuiApp>>,
) -> Result<()> {
    let mut pending_message: Option<String> = None;

    while app.read().await.running {
        // Read app state for rendering
        {
            let app_ref = app.read().await;
            terminal.draw(|f| ui(f, &app_ref))?;
        }

        // Handle keyboard events
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            if let KeyEvent::Key(key) = crossterm::event::read()? {
                let mut app_ref = app.write().await;
                if key.code == KeyCode::Enter && !app_ref.input.is_empty() {
                    pending_message = Some(app_ref.input.clone());
                }
                app_ref.handle_key(key.code, key.modifiers);
            }
        }

        // Process Bus events (non-blocking)
        {
            let mut app_ref = app.write().await;
            app_ref.process_bus_events();
            app_ref.process_expired_toasts();
        }

        // Handle pending message
        if let Some(msg) = pending_message.take() {
            let mut app_ref = app.write().await;
            app_ref.send_message(msg).await;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("ReOpenCode v0.1.0 - Press 'q' to quit")
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(Color::Green))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(status, chunks[1]);

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .map(|m| ListItem::new(Line::from(m.clone())))
        .collect();
    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));
    f.render_widget(messages, chunks[2]);

    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[3]);

    let files: Vec<ListItem> = app
        .files
        .iter()
        .map(|f| {
            ListItem::new(Line::from(Span::styled(
                f.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            )))
        })
        .collect();
    let files = List::new(files).block(Block::default().borders(Borders::ALL).title("Files"));
    f.render_widget(files, chunks[4]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::{TUI_COMMAND_EXECUTE, TUI_PROMPT_APPEND, TUI_SESSION_SELECT, TUI_TOAST_SHOW};
    use std::time::Duration;

    #[tokio::test]
    async fn test_process_bus_events_toast() {
        let mut app = TuiApp::new();

        // Manually publish an event to the bus
        let bus_clone = Arc::clone(&app.bus);
        bus_clone
            .publish(
                &TUI_TOAST_SHOW,
                TuiToastShowProperties {
                    title: "Test".to_string(),
                    message: "Hello from Bus".to_string(),
                    variant: Some("success".to_string()),
                    duration: None,
                },
            )
            .await;

        // Small delay for async publish
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Process events (simulating main loop)
        app.process_bus_events();

        // Verify toast was added
        assert!(
            app.messages
                .iter()
                .any(|m: &String| m.contains("[Success] Test: Hello from Bus"))
        );
    }

    #[tokio::test]
    async fn test_process_bus_events_prompt_append() {
        let mut app = TuiApp::new();
        app.input = "existing ".to_string();

        let bus_clone = Arc::clone(&app.bus);
        bus_clone
            .publish(
                &TUI_PROMPT_APPEND,
                TuiPromptAppendProperties {
                    prompt: "appended".to_string(),
                },
            )
            .await;

        tokio::time::sleep(Duration::from_millis(10)).await;
        app.process_bus_events();

        assert_eq!(app.input, "existing appended");
    }

    #[tokio::test]
    async fn test_process_bus_events_session_select() {
        let mut app = TuiApp::new();

        let bus_clone = Arc::clone(&app.bus);
        bus_clone
            .publish(
                &TUI_SESSION_SELECT,
                TuiSessionSelectProperties {
                    session_id: "test-session-abc".to_string(),
                },
            )
            .await;

        tokio::time::sleep(Duration::from_millis(10)).await;
        app.process_bus_events();

        assert_eq!(app.session_id, Some("test-session-abc".to_string()));
        assert_eq!(app.status, "Session: test-session-abc");
    }

    #[tokio::test]
    async fn test_process_bus_events_command_execute() {
        let mut app = TuiApp::new();

        let bus_clone = Arc::clone(&app.bus);
        bus_clone
            .publish(
                &TUI_COMMAND_EXECUTE,
                TuiCommandExecuteProperties {
                    command: "cargo clippy".to_string(),
                },
            )
            .await;

        tokio::time::sleep(Duration::from_millis(10)).await;
        app.process_bus_events();

        assert!(
            app.messages
                .iter()
                .any(|m: &String| m.contains("[Command] cargo clippy"))
        );
    }

    #[tokio::test]
    async fn test_process_expired_toasts_integration() {
        let mut app = TuiApp::new();

        // Add toast with 0ms duration (expires immediately)
        let bus_clone = Arc::clone(&app.bus);
        bus_clone
            .publish(
                &TUI_TOAST_SHOW,
                TuiToastShowProperties {
                    title: "Quick".to_string(),
                    message: "Gone fast".to_string(),
                    variant: None,
                    duration: Some(0),
                },
            )
            .await;

        tokio::time::sleep(Duration::from_millis(10)).await;
        app.process_bus_events();
        let count_after_publish = app.messages.len();

        // Wait a bit more and process expired
        tokio::time::sleep(Duration::from_millis(10)).await;
        app.process_expired_toasts();

        // Toast should be removed
        assert!(app.messages.len() < count_after_publish);
    }

    #[tokio::test]
    async fn test_active_toasts_not_removed_integration() {
        let mut app = TuiApp::new();

        // Add toast with 10 second duration
        let bus_clone = Arc::clone(&app.bus);
        bus_clone
            .publish(
                &TUI_TOAST_SHOW,
                TuiToastShowProperties {
                    title: "Persistent".to_string(),
                    message: "Stays around".to_string(),
                    variant: None,
                    duration: Some(10000),
                },
            )
            .await;

        tokio::time::sleep(Duration::from_millis(10)).await;
        app.process_bus_events();
        let count_after_publish = app.messages.len();

        // Process expired (should not remove active toast)
        app.process_expired_toasts();

        // Toast should NOT be removed
        assert_eq!(app.messages.len(), count_after_publish);
    }
}
