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
    event::{self, Event, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::mcp::types::McpStatus;
use components::mcp_status::McpStatusPanel;
use components::session_list::SessionList;
use components::{
    AgentInfo, CommandDialog, CommandEntry, Component, ContextInfo, DialogAgent, FocusManager,
    Footer, Header, List, MessageList, Sidebar, StatusDialog, TextArea,
};
use keybindings::{KeybindInfo, KeybindsConfig, LeaderState};
use ratatui::layout::Position;
use std::collections::HashMap;
use theme::ThemeContext;
use transcript::{MessageRole, TranscriptMessage, create_assistant_message, create_user_message};

const SERVER_URL: &str = "http://127.0.0.1:4096";

#[derive(Debug)]
enum StreamChunk {
    Text(String),
    Done,
    Error(String),
}

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
    /// Receives text chunks from an in-progress SSE stream.
    stream_rx: Option<UnboundedReceiver<StreamChunk>>,
    /// When the current stream started (for elapsed-time tracking).
    stream_started: Option<std::time::Instant>,
    /// Frame counter for spinner animation.
    frame_count: u64,
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

        Self {
            running: true,
            client: Client::new(),
            session_id: None,
            status: "Connecting…".to_string(),
            messages: vec![],
            files: vec![],
            input_component: TextArea::new(),
            message_list: MessageList::new(vec![]),
            file_list: List::with_title(vec![], "Files"),
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
            stream_rx: None,
            stream_started: None,
            frame_count: 0,
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
                            self.status = "Ready".to_string();
                        }
                        Err(e) => {
                            self.status = format!("Parse error: {}", e);
                        }
                    }
                } else {
                    self.status = format!("Server error: {}", response.status());
                }
            }
            Err(_) => {
                self.status = "Server offline".to_string();
            }
        }
    }

    pub async fn send_message(&mut self, content: String) {
        let Some(ref session_id) = self.session_id else {
            let err = create_assistant_message("No active session", "system", "internal", None);
            self.messages.push(err.clone());
            self.message_list.push(err);
            return;
        };

        // Add placeholder message that chunks will stream into.
        let placeholder = create_assistant_message("", "build", "moonshot-v1-8k", None);
        self.messages.push(placeholder.clone());
        self.message_list.push(placeholder);
        self.stream_started = Some(std::time::Instant::now());
        self.status = "Streaming...".to_string();

        let (tx, rx): (UnboundedSender<StreamChunk>, _) = unbounded_channel();
        self.stream_rx = Some(rx);

        let client = self.client.clone();
        let url = format!("{}/session/{}/stream", SERVER_URL, session_id);
        let request = SendMessageRequest { content };

        tokio::spawn(async move {
            let result = client.post(&url).json(&request).send().await;
            match result {
                Err(e) => { let _ = tx.send(StreamChunk::Error(e.to_string())); }
                Ok(resp) if !resp.status().is_success() => {
                    let _ = tx.send(StreamChunk::Error(format!("HTTP {}", resp.status())));
                }
                Ok(resp) => {
                    let mut bytes = resp.bytes_stream();
                    let mut buf = String::new();
                    while let Some(chunk) = bytes.next().await {
                        match chunk {
                            Err(e) => {
                                let _ = tx.send(StreamChunk::Error(e.to_string()));
                                return;
                            }
                            Ok(b) => {
                                buf.push_str(&String::from_utf8_lossy(&b));
                                // SSE events are delimited by \n\n
                                while let Some(pos) = buf.find("\n\n") {
                                    let event = buf[..pos].to_string();
                                    buf = buf[pos + 2..].to_string();
                                    for line in event.lines() {
                                        if let Some(data) = line.strip_prefix("data: ") {
                                            let _ = tx.send(StreamChunk::Text(data.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let _ = tx.send(StreamChunk::Done);
                }
            }
        });
    }

    /// Drain pending SSE chunks and update the in-progress message.
    pub fn process_stream_chunks(&mut self) {
        use tokio::sync::mpsc::error::TryRecvError;
        use transcript::PartType;

        let Some(ref mut rx) = self.stream_rx else { return; };

        loop {
            match rx.try_recv() {
                Ok(StreamChunk::Text(t)) => {
                    if let Some(msg) = self.messages.last_mut() {
                        if let Some(PartType::Text { text, .. }) = msg.parts.first_mut() {
                            text.push_str(&t);
                        }
                    }
                    self.message_list.append_last_text(&t);
                }
                Ok(StreamChunk::Done) => {
                    let elapsed_ms = self.stream_started.take()
                        .map(|s| s.elapsed().as_millis() as u64);
                    if let Some(msg) = self.messages.last_mut() {
                        if let MessageRole::Assistant { ref mut duration_ms, .. } = msg.role {
                            *duration_ms = elapsed_ms;
                        }
                    }
                    self.message_list.update_last_duration(elapsed_ms);
                    self.stream_rx = None;
                    self.status = "Ready".to_string();
                    break;
                }
                Ok(StreamChunk::Error(e)) => {
                    let err_msg = create_assistant_message(
                        &format!("Stream error: {}", e),
                        "system",
                        "internal",
                        None,
                    );
                    self.messages.push(err_msg.clone());
                    self.message_list.push(err_msg);
                    self.stream_rx = None;
                    self.status = "Error".to_string();
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.stream_rx = None;
                    self.status = "Ready".to_string();
                    break;
                }
            }
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new();
    app.init();
    app.init_session().await;

    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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
        app.process_stream_chunks();
        app.frame_count = app.frame_count.wrapping_add(1);
        terminal.draw(|f| ui(f, &mut *app))?;

        let now = std::time::Instant::now();
        let delta = now.duration_since(last_update);
        last_update = now;

        app.input_component.update(delta);
        app.footer.update(delta);
        app.process_expired_toasts();

        let poll_ms = if app.stream_rx.is_some() { 33 } else { 100 };

        // Block waiting for the first event, then drain all queued events.
        if crossterm::event::poll(std::time::Duration::from_millis(poll_ms))? {
            loop {
                match event::read()? {
                    Event::Key(key) => {
                        if let Some(msg) = app.handle_key(key) {
                            pending_message = Some(msg);
                        }
                    }
                    Event::Resize(_, _) => {
                        // terminal.draw handles resize automatically
                    }
                    _ => {}
                }
                if !crossterm::event::poll(std::time::Duration::from_millis(0))? {
                    break;
                }
            }
        }

        if let Some(msg) = pending_message.take() {
            app.send_message(msg).await;
        }
    }

    Ok(())
}

fn ui(f: &mut Frame, app: &mut TuiApp) {
    let area = f.area();
    let bg = app.theme.background();

    // Fill the entire terminal with our background color
    let bg_block = ratatui::widgets::Block::default()
        .style(ratatui::style::Style::default().bg(bg));
    f.render_widget(bg_block, area);

    // Layout: body | input (3 rows) | footer (1 row)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // body
            Constraint::Length(3), // input prompt
            Constraint::Length(1), // footer
        ])
        .split(area);

    const SIDEBAR_W: u16 = 42;
    // Auto-show sidebar when terminal is wide (≥ 120 cols), matching opencode's behaviour.
    // Also respect explicit sidebar toggle.
    let show_sidebar = (app.sidebar.is_expanded() || area.width >= 120) && area.width > SIDEBAR_W + 20;

    if show_sidebar {
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(SIDEBAR_W), Constraint::Min(1)])
            .split(outer[0]);
        render_sidebar(f, app, body[0]);
        render_messages(f, app, body[1]);
    } else {
        render_messages(f, app, outer[0]);
    }

    render_input(f, app, outer[1]);
    render_footer(f, app, outer[2]);

    // Overlay dialogs
    if app.status_dialog.is_visible() {
        app.status_dialog.render(f, area);
    }
    if app.command_dialog.is_visible() {
        app.command_dialog.render(f, area);
    }
    if app.dialog_agent.is_visible() {
        app.dialog_agent.render(f, area);
    }
    if app.session_list.is_visible() {
        let w = (area.width * 3 / 5).max(40).min(area.width);
        let h = (area.height * 2 / 3).max(10).min(area.height);
        let x = area.x + area.width.saturating_sub(w) / 2;
        let y = area.y + area.height.saturating_sub(h) / 2;
        app.session_list.render(f, Rect::new(x, y, w, h));
    }
}

// ─── sub-renderers ────────────────────────────────────────────────────────────

fn render_sidebar(f: &mut Frame, app: &TuiApp, area: Rect) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Paragraph};

    let theme = &app.theme;
    let panel_bg = theme.colors().background_panel;
    let text = theme.text();
    let muted = theme.colors().text_muted;
    let primary = theme.primary();
    let success = theme.success();

    // Fill sidebar with panel background
    f.render_widget(Block::default().style(Style::default().bg(panel_bg)), area);

    // Content area with 2-col padding on each side, 1-row top/bottom
    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let mut lines: Vec<Line> = Vec::new();

    // Session title (bold)
    let title = app.session_id
        .as_ref()
        .map(|_| "New Session".to_string())
        .unwrap_or_else(|| "New Session".to_string());
    lines.push(Line::from(Span::styled(
        title,
        Style::default().fg(text).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Keybind hints - matches opencode sidebar style
    let key_style = Style::default().fg(primary).add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(muted);

    lines.push(Line::from(vec![
        Span::styled("Enter ", key_style),
        Span::styled("send", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+p ", key_style),
        Span::styled("sessions", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+b ", key_style),
        Span::styled("sidebar", desc_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Ctrl+c ", key_style),
        Span::styled("quit", desc_style),
    ]));

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(panel_bg)),
        inner,
    );

    // Version at bottom — "● OpenCode ROC" like opencode's "● OpenCode <version>"
    if area.height > 3 {
        let ver_y = area.y + area.height.saturating_sub(2);
        let bottom_area = Rect {
            x: area.x + 2,
            y: ver_y,
            width: area.width.saturating_sub(4),
            height: 1,
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("●", Style::default().fg(success)),
                Span::styled(" Open", Style::default().fg(muted)),
                Span::styled("Code", Style::default().fg(text).add_modifier(Modifier::BOLD)),
                Span::styled(" ROC", Style::default().fg(primary)),
            ]))
            .style(Style::default().bg(panel_bg)),
            bottom_area,
        );
    }
}

fn render_messages(f: &mut Frame, app: &TuiApp, area: Rect) {
    // 4-col horizontal padding (2 each side), full height — matches opencode's message area
    let inner = Rect {
        x: area.x + 2,
        y: area.y,
        width: area.width.saturating_sub(4),
        height: area.height,
    };
    app.message_list.render(f, inner);
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn render_input(f: &mut Frame, app: &TuiApp, area: Rect) {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let theme = &app.theme;
    let muted = theme.colors().text_muted;
    let primary = theme.primary();
    let text_color = theme.text();
    let border_color = theme.colors().border;
    let bg = theme.background();

    // Top separator line spanning the full width
    let sep = "─".repeat(area.width as usize);
    f.render_widget(
        Paragraph::new(Span::styled(sep, Style::default().fg(border_color).bg(bg))),
        Rect { x: area.x, y: area.y, width: area.width, height: 1 },
    );

    // Input content (rows below separator), with 1-col padding each side
    let content = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(1),
    };

    let is_streaming = app.stream_rx.is_some();
    let input_text = app.input_component.text();

    if is_streaming {
        // Animated spinner
        let frame_idx = (app.frame_count / 3) as usize % SPINNER_FRAMES.len();
        let spinner = SPINNER_FRAMES[frame_idx];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{} ", spinner), Style::default().fg(primary)),
                Span::styled("Thinking…", Style::default().fg(muted)),
            ])),
            content,
        );
    } else if input_text.is_empty() {
        // Empty: show "> " cursor + placeholder + hint on row 2
        let row0 = Rect { x: content.x, y: content.y, width: content.width, height: 1 };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(primary)),
                Span::styled("Ask anything…", Style::default().fg(muted)),
            ])),
            row0,
        );
        if content.height > 1 {
            let row1 = Rect { x: content.x, y: content.y + 1, width: content.width, height: 1 };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("Enter", Style::default().fg(muted)),
                    Span::styled(" send  ", Style::default().fg(muted)),
                    Span::styled("Shift+Enter", Style::default().fg(muted)),
                    Span::styled(" newline", Style::default().fg(muted)),
                ])),
                row1,
            );
        }
        // Place hardware cursor at start of prompt
        f.set_cursor_position(Position { x: content.x + 2, y: content.y });
    } else {
        // Show typed lines with "> " prefix on first line
        let (cursor_row, cursor_col) = app.input_component.cursor_position();
        for (i, line) in input_text.lines().enumerate() {
            if i as u16 >= content.height { break; }
            let row = Rect { x: content.x, y: content.y + i as u16, width: content.width, height: 1 };
            let prefix = if i == 0 { "> " } else { "  " };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(primary)),
                    Span::styled(line.to_string(), Style::default().fg(text_color)),
                ])),
                row,
            );
        }
        // Hardware cursor at typed position
        let cursor_x = content.x + 2 + cursor_col as u16;
        let cursor_y = content.y + cursor_row as u16;
        if cursor_y < content.y + content.height {
            f.set_cursor_position(Position { x: cursor_x, y: cursor_y });
        }
    }
}

fn render_footer(f: &mut Frame, app: &TuiApp, area: Rect) {
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

    let theme = &app.theme;
    let muted = theme.colors().text_muted;
    let text = theme.text();
    let success = theme.success();
    let warning = theme.warning();
    let bg = theme.background();

    // Left side: working directory path (muted)
    let dir = &app.current_directory;

    // Right side: build right section then measure its length
    let lsp_dot = if app.lsp_count > 0 { "•" } else { "•" };
    let lsp_dot_style = Style::default().fg(if app.lsp_count > 0 { success } else { muted });
    let mcp_count = app.mcp_statuses.len();

    let mut right_spans: Vec<Span> = Vec::new();

    if app.pending_permissions > 0 {
        right_spans.push(Span::styled(
            format!("△ {} Permission{}  ", app.pending_permissions,
                if app.pending_permissions > 1 { "s" } else { "" }),
            Style::default().fg(warning),
        ));
    }

    right_spans.push(Span::styled(lsp_dot, lsp_dot_style));
    right_spans.push(Span::styled(format!(" {} LSP  ", app.lsp_count), Style::default().fg(text)));
    right_spans.push(Span::styled("⊙ ", Style::default().fg(success)));
    right_spans.push(Span::styled(format!("{} MCP  ", mcp_count), Style::default().fg(text)));
    right_spans.push(Span::styled("/status", Style::default().fg(muted)));

    let right_width: usize = right_spans.iter().map(|s| s.content.chars().count()).sum();
    let total_w = area.width as usize;
    // Truncate directory to fit
    let max_dir = total_w.saturating_sub(right_width + 2);
    let dir_shown: String = dir.chars().take(max_dir).collect();
    let gap = total_w.saturating_sub(dir_shown.chars().count() + right_width).max(1);

    let mut spans: Vec<Span> = vec![Span::styled(dir_shown, Style::default().fg(muted))];
    spans.push(Span::raw(" ".repeat(gap)));
    spans.extend(right_spans);

    f.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::default().bg(bg)),
        area,
    );
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
