//! TUI Command - Terminal User Interface with Component System

pub mod components;
pub mod events;
pub mod keybindings;
pub mod syntax;
pub mod theme;
pub mod transcript;
pub mod transcript_renderer;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;

use crate::mcp::types::McpStatus;
use components::mcp_status::McpStatusPanel;
use components::{Component, ContextInfo, List, Sidebar, TextArea};
use keybindings::{KeybindInfo, KeybindsConfig, LeaderState};
use std::collections::HashMap;
use theme::ThemeContext;

const SERVER_URL: &str = "http://127.0.0.1:4096";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub session_id: String,
    pub tool: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
    pub message_count: u32,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageRequest {
    pub content: String,
}

pub struct TuiApp {
    pub running: bool,
    pub client: Client,
    pub session_id: Option<String>,
    pub status: String,
    pub messages: Vec<String>,
    pub files: Vec<String>,
    pub input_component: TextArea,
    pub message_list: List<String>,
    pub file_list: List<String>,
    pub toasts: Vec<(usize, u64)>,
    pub keybinds: KeybindsConfig,
    pub leader_state: LeaderState,
    pub theme: ThemeContext,
    pub mcp_status_panel: McpStatusPanel,
    pub mcp_status_expanded: bool,
    pub mcp_statuses: HashMap<String, McpStatus>,
    pub current_directory: String,
    pub lsp_count: usize,
    pub pending_permissions: usize,
    pub sidebar: Sidebar,
    pub context_info: ContextInfo,
}

impl Default for TuiApp {
    fn default() -> Self {
        let current_dir = std::env::current_dir()
            .ok()
            .and_then(|p| {
                let path_str = p.to_string_lossy().to_string();
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy();
                    if path_str.starts_with(&*home_str) {
                        return Some(path_str.replacen(&*home_str, "~", 1));
                    }
                }
                Some(path_str)
            })
            .unwrap_or_else(|| ".".to_string());

        Self {
            running: true,
            client: Client::new(),
            session_id: None,
            status: "Connecting...".to_string(),
            messages: vec!["Welcome to ReOpenCode!".to_string()],
            files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            input_component: TextArea::with_title("Input (Enter to send, q to quit)"),
            message_list: List::with_title(vec!["Welcome to ReOpenCode!".to_string()], "Messages"),
            file_list: List::with_title(
                vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
                "Files",
            ),
            toasts: Vec::new(),
            keybinds: KeybindsConfig::default(),
            leader_state: LeaderState::default(),
            theme: ThemeContext::default(),
            mcp_status_panel: McpStatusPanel::new(),
            mcp_status_expanded: false,
            mcp_statuses: HashMap::new(),
            current_directory: current_dir,
            lsp_count: 0,
            pending_permissions: 0,
            sidebar: Sidebar::new(ThemeContext::default()),
            context_info: ContextInfo::default(),
        }
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) {
        self.input_component
            .set_placeholder("Type your message here...");
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        let key_info = KeybindInfo::from_crossterm(&key);
        let leader_active = self.leader_state.is_active();

        self.leader_state.update();

        if self.leader_state.check_and_handle(&key_info) {
            return None;
        }

        if self.keybinds.matches("app_exit", &key_info, leader_active) {
            self.running = false;
            return None;
        }

        if self
            .keybinds
            .matches("input_submit", &key_info, leader_active)
        {
            let input = self.input_component.text();
            if !input.is_empty() {
                let msg = input.clone();
                self.messages.push(format!("> {}", input));
                self.status = "Sending...".to_string();
                self.input_component.clear();
                self.message_list = List::with_title(self.messages.clone(), "Messages");
                return Some(msg);
            }
            return None;
        }

        if self
            .keybinds
            .matches("input_clear", &key_info, leader_active)
        {
            self.input_component.clear();
            return None;
        }

        if self
            .keybinds
            .matches("mcp_status_toggle", &key_info, leader_active)
        {
            self.mcp_status_expanded = !self.mcp_status_expanded;
            return None;
        }

        if self
            .keybinds
            .matches("sidebar_toggle", &key_info, leader_active)
        {
            self.sidebar.toggle();
            return None;
        }

        self.input_component.handle_input(key);
        None
    }

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
                            self.message_list = List::with_title(self.messages.clone(), "Messages");
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
                self.message_list = List::with_title(self.messages.clone(), "Messages");
            }
        }
    }

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
                                self.message_list =
                                    List::with_title(self.messages.clone(), "Messages");
                            }
                            Err(e) => {
                                self.messages.push(format!("[Error] Parse error: {}", e));
                                self.message_list =
                                    List::with_title(self.messages.clone(), "Messages");
                            }
                        }
                    } else {
                        self.messages
                            .push(format!("[Error] Server returned: {}", response.status()));
                        self.message_list = List::with_title(self.messages.clone(), "Messages");
                    }
                }
                Err(e) => {
                    self.messages.push(format!("[Error] Failed to send: {}", e));
                    self.message_list = List::with_title(self.messages.clone(), "Messages");
                }
            }
        } else {
            self.messages.push("[Error] No active session".to_string());
            self.message_list = List::with_title(self.messages.clone(), "Messages");
        }
    }

    pub fn process_expired_toasts(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut to_remove: Vec<usize> = self
            .toasts
            .iter()
            .filter(|(_, expires)| *expires <= now)
            .map(|(idx, _)| *idx)
            .collect();

        to_remove.sort_by(|a, b| b.cmp(a));
        for idx in to_remove {
            if idx < self.messages.len() {
                self.messages.remove(idx);
            }
        }
        self.toasts.retain(|(_, expires)| *expires > now);
    }

    pub async fn fetch_lsp(&mut self) {
        match self.client.get(format!("{}/lsp", SERVER_URL)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<serde_json::Value>>().await {
                        Ok(lsp_list) => {
                            self.lsp_count = lsp_list.len();
                        }
                        Err(_) => {
                            self.lsp_count = 0;
                        }
                    }
                } else {
                    self.lsp_count = 0;
                }
            }
            Err(_) => {
                self.lsp_count = 0;
            }
        }
    }

    pub async fn fetch_permissions(&mut self) {
        match self
            .client
            .get(format!("{}/permission", SERVER_URL))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<PermissionRequest>>().await {
                        Ok(permissions) => {
                            self.pending_permissions = permissions.len();
                        }
                        Err(_) => {
                            self.pending_permissions = 0;
                        }
                    }
                } else {
                    self.pending_permissions = 0;
                }
            }
            Err(_) => {
                self.pending_permissions = 0;
            }
        }
    }
}

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new();
    app.init();
    app.init_session().await;

    let result = run_app(&mut terminal, &mut app).await;

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

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> Result<()> {
    let mut pending_message: Option<String> = None;
    let mut last_update = std::time::Instant::now();

    while app.running {
        terminal.draw(|f| ui(f, &mut *app))?;

        let now = std::time::Instant::now();
        let delta = now.duration_since(last_update);
        last_update = now;

        app.input_component.update(delta);
        app.process_expired_toasts();

        if crossterm::event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(msg) = app.handle_key(key)
        {
            pending_message = Some(msg);
        }

        if let Some(msg) = pending_message.take() {
            app.send_message(msg).await;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &mut TuiApp) {
    let theme = &app.theme;

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
        .style(Style::default().fg(theme.primary()))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let status = Paragraph::new(app.status.as_str())
        .style(Style::default().fg(theme.success()))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(status, chunks[1]);

    // Render footer with directory on left and MCP/LSP/Permissions status on right
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Directory path on the left
    let directory_paragraph = Paragraph::new(app.current_directory.as_str())
        .style(Style::default().fg(theme.text_muted()))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(directory_paragraph, footer_chunks[0]);

    // Render permissions indicator (between directory and LSP/MCP)
    if app.pending_permissions > 0 {
        let perm_indicator = format!("△ {} Permission(s)", app.pending_permissions);
        let perm_paragraph = Paragraph::new(perm_indicator)
            .style(Style::default().fg(theme.warning()))
            .block(Block::default().borders(Borders::NONE));

        // Position permissions indicator to the left of LSP indicator
        let perm_area = Rect::new(f.area().width - 45, f.area().y + 1, 20, 1);
        f.render_widget(perm_paragraph, perm_area);
    }

    // Render LSP indicator before MCP indicator
    let lsp_color = if app.lsp_count > 0 {
        theme.success()
    } else {
        theme.text_muted()
    };
    let lsp_indicator = format!("• {} LSP", app.lsp_count);
    let lsp_paragraph = Paragraph::new(lsp_indicator)
        .style(Style::default().fg(lsp_color))
        .block(Block::default().borders(Borders::NONE));

    // Position LSP indicator to the left of MCP indicator
    let lsp_area = Rect::new(f.area().width - 30, f.area().y + 1, 12, 1);
    f.render_widget(lsp_paragraph, lsp_area);

    // Render MCP status footer indicator
    let mcp_count = app.mcp_statuses.len();
    let all_ok = app
        .mcp_statuses
        .values()
        .all(|s| matches!(s, McpStatus::Connected));
    let mcp_color = if mcp_count == 0 {
        theme.text_muted()
    } else if all_ok {
        theme.success()
    } else {
        theme.error()
    };

    let mcp_indicator = format!("⊙ {} MCP", mcp_count);
    let mcp_footer = Paragraph::new(mcp_indicator)
        .style(Style::default().fg(mcp_color))
        .block(Block::default().borders(Borders::NONE));

    // Position MCP indicator in the bottom right of the status area
    let mcp_area = Rect::new(f.area().width - 15, f.area().y + 1, 15, 1);
    f.render_widget(mcp_footer, mcp_area);

    // Render sidebar when expanded
    if app.sidebar.is_expanded() {
        let sidebar_width = app.sidebar.width();
        let sidebar_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(sidebar_width)])
            .split(f.area());

        let sidebar_area = Rect::new(
            sidebar_chunks[1].x,
            sidebar_chunks[1].y,
            sidebar_chunks[1].width,
            sidebar_chunks[1].height,
        );

        app.sidebar.render(f, sidebar_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tui_app_new() {
        let app = TuiApp::new();
        assert!(app.running);
    }

    #[test]
    fn test_tui_app_default() {
        let app = TuiApp::default();
        assert!(app.running);
        assert_eq!(app.session_id, None);
    }

    #[test]
    fn test_session_info() {
        let info = SessionInfo {
            id: "test".to_string(),
            title: "Test".to_string(),
            created_at: "2024".to_string(),
            updated_at: "2024".to_string(),
            status: "active".to_string(),
            message_count: 1,
        };
        assert_eq!(info.id, "test");
    }
}
