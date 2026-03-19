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
use components::session_list::SessionList;
use components::{
    AgentInfo, CommandDialog, CommandEntry, Component, ContextInfo, DialogAgent, FocusManager,
    Footer, List, MessageList, Sidebar, StatusDialog, TextArea,
};
use keybindings::{KeybindInfo, KeybindsConfig, LeaderState};
use std::collections::HashMap;
use theme::ThemeContext;
use transcript::{TranscriptMessage, create_assistant_message, create_user_message};

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
    pub messages: Vec<TranscriptMessage>,
    pub files: Vec<String>,
    pub input_component: TextArea,
    pub message_list: MessageList,
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
    pub footer: Footer,
    pub status_dialog: StatusDialog,
    pub focus_manager: FocusManager,
    pub command_dialog: CommandDialog,
    pub session_list: SessionList,
    pub dialog_agent: DialogAgent,
}

impl Default for TuiApp {
    fn default() -> Self {
        let current_dir = std::env::current_dir()
            .ok()
            .map(|p| {
                let path_str = p.to_string_lossy().to_string();
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy();
                    if path_str.starts_with(&*home_str) {
                        return path_str.replacen(&*home_str, "~", 1);
                    }
                }
                path_str
            })
            .unwrap_or_else(|| ".".to_string());

        let welcome_msg = create_user_message("Welcome to ReOpenCode!");

        Self {
            running: true,
            client: Client::new(),
            session_id: None,
            status: "Connecting...".to_string(),
            messages: vec![welcome_msg.clone()],
            files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            input_component: TextArea::with_title("Input (Enter to send, q to quit)"),
            message_list: MessageList::with_title(vec![welcome_msg], "Messages"),
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
            current_directory: current_dir.clone(),
            lsp_count: 0,
            pending_permissions: 0,
            sidebar: Sidebar::new(ThemeContext::default()),
            context_info: ContextInfo::default(),
            footer: Footer::new(ThemeContext::default()),
            status_dialog: StatusDialog::new(),
            focus_manager: FocusManager::new(),
            command_dialog: CommandDialog::new(),
            session_list: SessionList::new(),
            dialog_agent: DialogAgent::new(),
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

        // Register focusable components
        self.focus_manager.register(self.input_component.id());
        self.focus_manager.register(self.message_list.id());

        // Set initial focus to input component
        if self
            .focus_manager
            .set_focus(self.input_component.id())
            .is_ok()
        {
            self.input_component.on_focus();
        }

        // Initialize command dialog with available commands
        self.init_commands();

        self.init_agents();
    }

    fn init_agents(&mut self) {
        let agents = vec![
            AgentInfo::new("build")
                .with_description("General coding assistant")
                .with_category("default"),
            AgentInfo::new("explore")
                .with_description("Code exploration")
                .with_category("subagent"),
            AgentInfo::new("librarian")
                .with_description("Documentation lookup")
                .with_category("subagent"),
        ];
        self.dialog_agent.set_agents(agents);
        self.dialog_agent.set_current_agent("build");
    }

    fn init_commands(&mut self) {
        let commands = vec![
            CommandEntry::new("New Session", "session.new")
                .with_category("Session")
                .with_keybind("ctrl+n"),
            CommandEntry::new("List Sessions", "session.list")
                .with_category("Session")
                .with_keybind("ctrl+p"),
            CommandEntry::new("Toggle Sidebar", "sidebar.toggle")
                .with_category("View")
                .with_keybind("ctrl+b"),
            CommandEntry::new("MCP Status", "mcp.status")
                .with_category("View")
                .with_keybind("ctrl+m"),
            CommandEntry::new("Model Selection", "model.list")
                .with_category("Model")
                .with_keybind("ctrl+shift+m"),
            CommandEntry::new("Exit", "app.exit")
                .with_category("Application")
                .with_keybind("ctrl+c"),
            CommandEntry::new("Help", "help.show")
                .with_category("Application")
                .with_keybind("?"),
        ];
        self.command_dialog.set_commands(commands);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        use crate::cli::commands::tui::components::EventPropagation;

        if self.session_list.is_visible() && self.session_list.focused() {
            let propagation = self.session_list.handle_input(key);
            if propagation == EventPropagation::Stop {
                if self.session_list.rename_session_id().is_some() {
                    self.session_list.clear_rename_session();
                }
                return None;
            }
        }

        if self.command_dialog.is_visible() {
            let propagation = self.command_dialog.handle_input(key);
            if propagation == EventPropagation::Stop {
                return None;
            }
        }

        if self.dialog_agent.is_visible() {
            use crate::cli::commands::tui::components::EventPropagation;
            let propagation = self.dialog_agent.handle_input(key);
            if propagation == EventPropagation::Stop {
                if !self.dialog_agent.is_visible() {
                    if let Some(agent_name) = self
                        .dialog_agent
                        .get_selected_agent()
                        .map(|s| s.to_string())
                    {
                        self.dialog_agent.set_current_agent(agent_name);
                    }
                }
                return None;
            }
        }

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
            .matches("session_list", &key_info, leader_active)
        {
            if self.session_list.is_visible() {
                self.session_list.hide();
                self.session_list.on_blur();
            } else {
                self.session_list.show();
                self.session_list.on_focus();
                self.session_list
                    .set_current_session_id(self.session_id.clone());
            }
            return None;
        }

        if self
            .keybinds
            .matches("input_submit", &key_info, leader_active)
        {
            let input = self.input_component.text();
            if !input.is_empty() {
                let msg = input.clone();
                let user_msg = create_user_message(&input);
                self.messages.push(user_msg.clone());
                self.status = "Sending...".to_string();
                self.input_component.clear();
                self.message_list.push(user_msg);
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

        if self
            .keybinds
            .matches("status_view", &key_info, leader_active)
        {
            if self.status_dialog.is_visible() {
                self.status_dialog.hide();
            } else {
                self.status_dialog
                    .set_mcp_statuses(self.mcp_statuses.clone());
                self.status_dialog.set_lsp_count(self.lsp_count);
                self.status_dialog.show();
            }
            return None;
        }

        if self
            .keybinds
            .matches("command_list", &key_info, leader_active)
        {
            if self.command_dialog.is_visible() {
                self.command_dialog.hide();
            } else {
                self.command_dialog.clear_filter();
                self.command_dialog.show();
            }
            return None;
        }

        if self
            .keybinds
            .matches("agent_list", &key_info, leader_active)
        {
            if self.dialog_agent.is_visible() {
                self.dialog_agent.hide();
            } else {
                self.dialog_agent.clear_filter();
                self.dialog_agent.show();
            }
            return None;
        }

        // Handle Tab/Shift+Tab focus navigation
        if self
            .keybinds
            .matches("agent_cycle", &key_info, leader_active)
        {
            self.focus_next();
            return None;
        }

        if self
            .keybinds
            .matches("agent_cycle_reverse", &key_info, leader_active)
        {
            self.focus_prev();
            return None;
        }

        self.input_component.handle_input(key);
        None
    }

    fn focus_next(&mut self) {
        let prev_id = self.focus_manager.get_focused();
        if let Some(prev) = prev_id {
            if prev == self.input_component.id() {
                self.input_component.on_blur();
            } else if prev == self.message_list.id() {
                self.message_list.on_blur();
            }
        }

        if let Some(next_id) = self.focus_manager.next_focus() {
            if next_id == self.input_component.id() {
                self.input_component.on_focus();
            } else if next_id == self.message_list.id() {
                self.message_list.on_focus();
            }
        }
    }

    fn focus_prev(&mut self) {
        let prev_id = self.focus_manager.get_focused();
        if let Some(prev) = prev_id {
            if prev == self.input_component.id() {
                self.input_component.on_blur();
            } else if prev == self.message_list.id() {
                self.message_list.on_blur();
            }
        }

        if let Some(prev_id) = self.focus_manager.prev_focus() {
            if prev_id == self.input_component.id() {
                self.input_component.on_focus();
            } else if prev_id == self.message_list.id() {
                self.message_list.on_focus();
            }
        }
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
                            self.footer.set_session_id(self.session_id.clone());
                            self.status = format!("Connected: {}", session.id);
                            let system_msg = create_assistant_message(
                                &format!("Session created: {}", session.id),
                                "system",
                                "internal",
                                None,
                            );
                            self.messages.push(system_msg.clone());
                            self.message_list.push(system_msg);
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
                let error_msg = create_assistant_message(
                    "Could not connect to server. Is it running?",
                    "system",
                    "internal",
                    None,
                );
                self.messages.push(error_msg.clone());
                self.message_list.push(error_msg);
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
                                let assistant_msg =
                                    create_assistant_message(&text, "assistant", "default", None);
                                self.messages.push(assistant_msg.clone());
                                self.status = "Ready".to_string();
                                self.message_list.push(assistant_msg);
                            }
                            Err(e) => {
                                let error_msg = create_assistant_message(
                                    &format!("Parse error: {}", e),
                                    "system",
                                    "internal",
                                    None,
                                );
                                self.messages.push(error_msg.clone());
                                self.message_list.push(error_msg);
                            }
                        }
                    } else {
                        let error_msg = create_assistant_message(
                            &format!("Server returned: {}", response.status()),
                            "system",
                            "internal",
                            None,
                        );
                        self.messages.push(error_msg.clone());
                        self.message_list.push(error_msg);
                    }
                }
                Err(e) => {
                    let error_msg = create_assistant_message(
                        &format!("Failed to send: {}", e),
                        "system",
                        "internal",
                        None,
                    );
                    self.messages.push(error_msg.clone());
                    self.message_list.push(error_msg);
                }
            }
        } else {
            let error_msg =
                create_assistant_message("No active session", "system", "internal", None);
            self.messages.push(error_msg.clone());
            self.message_list.push(error_msg);
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

        if !to_remove.is_empty() {
            to_remove.sort_by(|a, b| b.cmp(a));

            for idx in to_remove {
                if idx < self.messages.len() {
                    self.messages.remove(idx);
                }
            }

            self.message_list.clear();
            for msg in &self.messages {
                self.message_list.push(msg.clone());
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
                            self.footer.set_lsp_count(self.lsp_count);
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
                            self.footer
                                .set_pending_permissions(self.pending_permissions);
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
        app.footer.update(delta);
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

    app.footer.render(f, chunks[1]);

    app.message_list.render(f, chunks[2]);

    app.input_component.render(f, chunks[3]);

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

    if app.status_dialog.is_visible() {
        app.status_dialog.render(f, f.area());
    }

    if app.command_dialog.is_visible() {
        app.command_dialog.render(f, f.area());
    }

    if app.dialog_agent.is_visible() {
        app.dialog_agent.render(f, f.area());
    }

    if app.session_list.is_visible() {
        let session_area = Rect::new(
            f.area().width / 4,
            f.area().height / 4,
            f.area().width / 2,
            f.area().height / 2,
        );
        app.session_list.render(f, session_area);
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
