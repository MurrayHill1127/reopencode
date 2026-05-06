//! TUI entry point — DeepSeek-TUI inspired layout.
//!
//! Layout zones (top → bottom):
//!   header   (1 row):  brand badge + session + model + context bar
//!   messages (flex):   scrollable conversation with per-cell cached markdown
//!   composer (5 rows): multi-line rounded-border input
//!   footer   (1 row):  dir + animated spinner + status

pub mod components;
pub mod events;
pub mod keybindings;
pub mod markdown;
pub mod slash_commands;
pub mod syntax;
pub mod theme;
pub mod tool_family;
pub mod transcript;
pub mod transcript_renderer;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
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
    Header, LeftSidebar, McpServerInfo, MessageList, Sidebar, SlashAutocomplete, StatusDialog, TextArea,
};
use keybindings::{KeybindInfo, KeybindsConfig, LeaderState};
use slash_commands::{SlashCommand, parse_slash};
use std::collections::HashMap;
use theme::ThemeContext;
use transcript::{MessageRole, TranscriptMessage, create_assistant_message, create_user_message};

use components::footer::Footer;

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
    pub session_title: String,
    pub status: String,
    pub messages: Vec<TranscriptMessage>,

    // ── Core UI components ────────────────────────────────────────────────
    pub header: Header,
    pub message_list: MessageList,
    pub input_component: TextArea,
    pub footer: Footer,
    pub left_sidebar: LeftSidebar,
    pub right_sidebar: Sidebar,

    // ── Overlay / dialog components ───────────────────────────────────────
    pub status_dialog: StatusDialog,
    pub command_dialog: CommandDialog,
    pub session_list: SessionList,
    pub dialog_agent: DialogAgent,
    pub slash_autocomplete: SlashAutocomplete,
    pub mcp_status_panel: McpStatusPanel,

    // ── Supporting state ──────────────────────────────────────────────────
    pub keybinds: KeybindsConfig,
    pub leader_state: LeaderState,
    pub theme: ThemeContext,
    pub mcp_statuses: HashMap<String, McpStatus>,
    pub current_directory: String,
    pub lsp_count: usize,
    pub pending_permissions: usize,
    pub context_info: ContextInfo,
    pub focus_manager: FocusManager,

    // ── Slash-command pending actions ─────────────────────────────────────
    pending_new_session: bool,
    pending_new_title: Option<String>,

    // ── Toast expiry state ────────────────────────────────────────────────
    pub toasts: Vec<(usize, u64)>,

    // ── Stream state ──────────────────────────────────────────────────────
    stream_rx: Option<UnboundedReceiver<StreamChunk>>,
    stream_started: Option<std::time::Instant>,
    pub frame_count: u64,

    // ── Staged Esc cancel ─────────────────────────────────────────────────
    esc_stage: u8,
    esc_stage_time: Option<std::time::Instant>,
}

impl Default for TuiApp {
    fn default() -> Self {
        let current_dir = std::env::current_dir()
            .ok()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                if let Some(home) = dirs::home_dir() {
                    let h = home.to_string_lossy();
                    if s.starts_with(&*h) {
                        return s.replacen(&*h, "~", 1);
                    }
                }
                s
            })
            .unwrap_or_else(|| ".".to_string());

        let mut header = Header::new();
        header.set_session_title("New Session");
        header.set_model("");

        let mut footer = Footer::new();
        footer.set_directory(current_dir.clone());

        Self {
            running: true,
            client: Client::new(),
            session_id: None,
            session_title: "New Session".to_string(),
            status: "Connecting…".to_string(),
            messages: vec![],

            header,
            message_list: MessageList::new(vec![]),
            input_component: TextArea::new(),
            footer,
            left_sidebar: LeftSidebar::new(),
            right_sidebar: Sidebar::new(ThemeContext::default()),

            status_dialog: StatusDialog::new(),
            command_dialog: CommandDialog::new(),
            session_list: SessionList::new(),
            dialog_agent: DialogAgent::new(),
            slash_autocomplete: SlashAutocomplete::new(),
            mcp_status_panel: McpStatusPanel::new(),

            keybinds: KeybindsConfig::default(),
            leader_state: LeaderState::default(),
            theme: ThemeContext::default(),
            mcp_statuses: HashMap::new(),
            current_directory: current_dir,
            lsp_count: 0,
            pending_permissions: 0,
            context_info: ContextInfo::default(),
            focus_manager: FocusManager::new(),

            pending_new_session: false,
            pending_new_title: None,

            toasts: Vec::new(),

            stream_rx: None,
            stream_started: None,
            frame_count: 0,

            esc_stage: 0,
            esc_stage_time: None,
        }
    }
}

impl TuiApp {
    pub fn new() -> Self { Self::default() }

    pub fn init(&mut self) {
        self.input_component.set_placeholder("Ask anything…");
        self.focus_manager.register(self.input_component.id());
        self.focus_manager.register(self.message_list.id());
        if self.focus_manager.set_focus(self.input_component.id()).is_ok() {
            self.input_component.on_focus();
        }
        self.init_commands();
        self.init_agents();
    }

    fn init_agents(&mut self) {
        let agents = vec![
            AgentInfo::new("build").with_description("General coding assistant").with_category("default"),
            AgentInfo::new("explore").with_description("Code exploration").with_category("subagent"),
            AgentInfo::new("librarian").with_description("Documentation lookup").with_category("subagent"),
        ];
        self.dialog_agent.set_agents(agents);
        self.dialog_agent.set_current_agent("build");
    }

    fn init_commands(&mut self) {
        let commands = vec![
            CommandEntry::new("New Session",    "session.new").with_category("Session").with_keybind("ctrl+n"),
            CommandEntry::new("List Sessions",  "session.list").with_category("Session").with_keybind("ctrl+p"),
            CommandEntry::new("MCP Status",     "mcp.status").with_category("View").with_keybind("ctrl+m"),
            CommandEntry::new("Model Selection","model.list").with_category("Model").with_keybind("ctrl+shift+m"),
            CommandEntry::new("Exit",           "app.exit").with_category("Application").with_keybind("ctrl+c"),
            CommandEntry::new("Help",           "help.show").with_category("Application").with_keybind("?"),
        ];
        self.command_dialog.set_commands(commands);
    }

    fn execute_slash(&mut self, cmd: SlashCommand) {
        match cmd {
            SlashCommand::Exit => { self.running = false; }
            SlashCommand::New { title } => {
                self.pending_new_session = true;
                self.pending_new_title = title;
            }
            SlashCommand::Clear => {
                self.messages.clear();
                self.message_list.clear();
                self.status = "Cleared".to_string();
                self.footer.set_status("Cleared".to_string());
            }
            SlashCommand::Help => { self.push_help_message(); }
            SlashCommand::Sessions => {
                self.left_sidebar.toggle();
                if self.left_sidebar.is_visible() { self.left_sidebar.on_focus(); }
            }
            SlashCommand::Unknown(name) => {
                let err = create_assistant_message(
                    &format!("Unknown command: /{}\n\nAvailable commands:\n  /exit    — quit\n  /new [title] — new session\n  /clear   — clear conversation\n  /help    — show this help\n  /sessions — toggle sidebar", name),
                    "system", "internal", None,
                );
                self.messages.push(err.clone());
                self.message_list.push(err);
            }
        }
    }

    fn execute_shell(&mut self, msg: &str) {
        let cmd_str = msg.strip_prefix('!').unwrap_or(msg).trim().to_string();
        if cmd_str.is_empty() { return; }

        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() { return; }

        let program = parts[0];
        let args = &parts[1..];
        let output = std::process::Command::new(program)
            .args(args)
            .output();

        let result_text = match output {
            Ok(out) => {
                let mut s = format!("$ {}\n", cmd_str);
                if !out.stdout.is_empty() {
                    s.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                if !out.stderr.is_empty() {
                    s.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                if !out.status.success() {
                    s.push_str(&format!("\nexit code: {}", out.status.code().unwrap_or(-1)));
                }
                s
            }
            Err(e) => format!("$ {}\nFailed: {}", cmd_str, e),
        };

        let user_msg = create_user_message(msg);
        self.messages.push(user_msg.clone());
        self.message_list.push(user_msg);
        let shell_msg = create_assistant_message(&result_text, "system", "shell", None);
        self.messages.push(shell_msg.clone());
        self.message_list.push(shell_msg);
        self.status = "Shell command run".to_string();
    }

    fn push_help_message(&mut self) {
        let text = "**Slash Commands**\n\
            \n\
            `/exit` — quit roc\n\
            `/new [title]` — create a new session\n\
            `/clear` — clear the current conversation\n\
            `/sessions` — toggle the sessions sidebar\n\
            `/help` — show this message\n\
            \n\
            `!` prefix — run shell command locally\n\
            \n\
            **Keyboard Shortcuts**\n\
            \n\
            `Ctrl+B` — toggle left sidebar (sessions)\n\
            `Ctrl+R` — toggle right sidebar (context, MCP, LSP)\n\
            `Ctrl+\\`` — toggle code block concealment\n\
            `Ctrl+P` — session list overlay\n\
            `Ctrl+M` — MCP status\n\
            `Ctrl+C` — cancel streaming / quit\n\
            `Ctrl+J` / `Shift+Enter` — newline in input\n\
            `Tab` — cycle focus / autocomplete slash cmd\n\
            `↑`/`↓` (empty input) — prompt history\n\
            `Esc` — 1st: cancel stream, 2nd: clear composer\n\
            `j`/`k` — scroll messages (when msg list focused)\n\
            `g`/`G` — jump to top / bottom\n";
        let msg = create_assistant_message(text, "system", "internal", None);
        self.messages.push(msg.clone());
        self.message_list.push(msg);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        use components::EventPropagation;

        // Overlay components get priority
        if self.left_sidebar.is_visible() && self.left_sidebar.is_focused() {
            if self.left_sidebar.handle_input(key) == EventPropagation::Stop {
                if !self.left_sidebar.is_focused() {
                    // Sidebar blurred — return focus to input
                    self.input_component.on_focus();
                }
                return None;
            }
        }

        if self.session_list.is_visible() && self.session_list.focused() {
            if self.session_list.handle_input(key) == EventPropagation::Stop {
                return None;
            }
        }

        if self.command_dialog.is_visible() {
            if self.command_dialog.handle_input(key) == EventPropagation::Stop {
                return None;
            }
        }

        if self.dialog_agent.is_visible() {
            if self.dialog_agent.handle_input(key) == EventPropagation::Stop {
                if !self.dialog_agent.is_visible() {
                    if let Some(name) = self.dialog_agent.get_selected_agent().map(str::to_string) {
                        self.dialog_agent.set_current_agent(name);
                    }
                }
                return None;
            }
        }

        let key_info = KeybindInfo::from_crossterm(&key);
        let leader_active = self.leader_state.is_active();
        self.leader_state.update();
        if self.leader_state.check_and_handle(&key_info) { return None; }

        // Esc: staged cancel (stream → clear composer → no-op)
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            if self.stream_rx.is_some() {
                self.stream_rx = None;
                self.status = "Cancelled".to_string();
                self.footer.set_streaming(false);
                self.header.set_streaming(false);
                self.input_component.set_streaming(false);
                self.esc_stage = 1;
                self.esc_stage_time = Some(std::time::Instant::now());
                self.footer.set_status("Press Esc again to clear composer".to_string());
                return None;
            } else if !self.input_component.is_empty() {
                self.input_component.clear();
                self.esc_stage = 2;
                self.esc_stage_time = Some(std::time::Instant::now());
                self.footer.set_status("Composer cleared".to_string());
                return None;
            }
            return None;
        }

        // Reset staged Esc on any other key
        if self.esc_stage > 0 {
            self.esc_stage = 0;
            self.esc_stage_time = None;
        }

        // Global bindings
        if self.keybinds.matches("app_exit", &key_info, leader_active) {
            if self.stream_rx.is_some() {
                // Cancel streaming on first Ctrl+C, quit on second
                self.stream_rx = None;
                self.status = "Cancelled".to_string();
                self.footer.set_streaming(false);
                self.header.set_streaming(false);
                self.input_component.set_streaming(false);
            } else {
                self.running = false;
            }
            return None;
        }

        // Ctrl+B toggles left sidebar
        if key.code == KeyCode::Char('b') && key.modifiers == KeyModifiers::CONTROL {
            self.left_sidebar.toggle();
            if self.left_sidebar.is_visible() {
                self.left_sidebar.on_focus();
                self.input_component.on_blur();
            } else {
                self.input_component.on_focus();
            }
            return None;
        }

        // Ctrl+R toggles right sidebar (context / MCP / LSP / diffs)
        if key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::CONTROL {
            self.right_sidebar.toggle();
            if self.right_sidebar.is_expanded() {
                self.right_sidebar.expand();
                self.input_component.on_blur();
            } else {
                self.right_sidebar.collapse();
                self.input_component.on_focus();
            }
            return None;
        }

        // Ctrl+` toggles code block concealment
        if key.code == KeyCode::BackTab || (key.code == KeyCode::Char('`') && key.modifiers == KeyModifiers::CONTROL) {
            self.message_list.toggle_conceal();
            self.footer.set_status(if self.message_list.code_concealed {
                "Code blocks concealed".to_string()
            } else {
                "Code blocks expanded".to_string()
            });
            return None;
        }

        if self.keybinds.matches("session_list", &key_info, leader_active) {
            if self.session_list.is_visible() {
                self.session_list.hide();
                self.session_list.on_blur();
            } else {
                self.session_list.show();
                self.session_list.on_focus();
                self.session_list.set_current_session_id(self.session_id.clone());
            }
            return None;
        }

        if self.keybinds.matches("mcp_status_toggle", &key_info, leader_active) {
            if self.status_dialog.is_visible() {
                self.status_dialog.hide();
            } else {
                self.status_dialog.set_mcp_statuses(self.mcp_statuses.clone());
                self.status_dialog.set_lsp_count(self.lsp_count);
                self.status_dialog.show();
            }
            return None;
        }

        if self.keybinds.matches("command_list", &key_info, leader_active) {
            if self.command_dialog.is_visible() {
                self.command_dialog.hide();
            } else {
                self.command_dialog.clear_filter();
                self.command_dialog.show();
            }
            return None;
        }

        if self.keybinds.matches("agent_list", &key_info, leader_active) {
            if self.dialog_agent.is_visible() {
                self.dialog_agent.hide();
            } else {
                self.dialog_agent.clear_filter();
                self.dialog_agent.show();
            }
            return None;
        }

        // Slash autocomplete: Tab completes, Up/Down navigates
        if self.slash_autocomplete.visible {
            if key.code == KeyCode::Tab {
                if let Some(cmd) = self.slash_autocomplete.take_completion() {
                    self.input_component.set_text(format!("{} ", cmd));
                }
                return None;
            }
            if matches!(key.code, KeyCode::Up | KeyCode::Down) {
                self.slash_autocomplete.handle_input(key);
                return None;
            }
            if key.code == KeyCode::Esc {
                self.slash_autocomplete.visible = false;
                return None;
            }
        }

        // Tab focus cycling
        if self.keybinds.matches("agent_cycle", &key_info, leader_active) {
            self.focus_next();
            return None;
        }
        if self.keybinds.matches("agent_cycle_reverse", &key_info, leader_active) {
            self.focus_prev();
            return None;
        }

        // Input component — checks take_submit internally
        self.input_component.handle_input(key);

        // Check if user pressed Enter to submit
        if let Some(msg) = self.input_component.take_submit() {
            // Slash commands are consumed here — not sent to the LLM
            if let Some(cmd) = parse_slash(&msg) {
                self.execute_slash(cmd);
                return None;
            }
            // Shell mode: !command runs locally
            if msg.starts_with('!') {
                self.execute_shell(&msg);
                return None;
            }
            let user_msg = create_user_message(&msg);
            self.messages.push(user_msg.clone());
            self.message_list.push(user_msg);
            self.status = "Sending…".to_string();
            return Some(msg);
        }

        None
    }

    fn focus_next(&mut self) {
        if let Some(prev) = self.focus_manager.get_focused() {
            if prev == self.input_component.id() { self.input_component.on_blur(); }
            else if prev == self.message_list.id() { self.message_list.on_blur(); }
        }
        if let Some(next) = self.focus_manager.next_focus() {
            if next == self.input_component.id() { self.input_component.on_focus(); }
            else if next == self.message_list.id() { self.message_list.on_focus(); }
        }
    }

    fn focus_prev(&mut self) {
        if let Some(prev) = self.focus_manager.get_focused() {
            if prev == self.input_component.id() { self.input_component.on_blur(); }
            else if prev == self.message_list.id() { self.message_list.on_blur(); }
        }
        if let Some(prev) = self.focus_manager.prev_focus() {
            if prev == self.input_component.id() { self.input_component.on_focus(); }
            else if prev == self.message_list.id() { self.message_list.on_focus(); }
        }
    }

    pub async fn init_session(&mut self) {
        self.status = "Connecting…".to_string();
        let request = CreateSessionRequest { title: Some("ROC Session".to_string()) };
        match self.client.post(format!("{}/session", SERVER_URL)).json(&request).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<SessionInfo>().await {
                    Ok(session) => {
                        self.session_id = Some(session.id.clone());
                        self.session_title = session.title.clone();
                        self.header.set_session_title(&session.title);
                        self.footer.set_session_id(Some(session.id.clone()));
                        self.status = "Ready".to_string();
                        self.footer.set_status("Ready".into());
                        // Populate sidebar with the new session
                        self.left_sidebar.set_current(Some(session.id.clone()));
                        self.left_sidebar.set_sessions(vec![(session.id, session.title)]);
                    }
                    Err(e) => { self.status = format!("Parse error: {}", e); }
                }
            }
            Ok(resp) => { self.status = format!("Server error: {}", resp.status()); }
            Err(_) => { self.status = "Server offline".to_string(); }
        }
    }

    pub async fn send_message(&mut self, content: String) {
        let session_id = match self.session_id.clone() {
            Some(id) => id,
            None => {
                let err = create_assistant_message("No active session", "system", "internal", None);
                self.messages.push(err.clone());
                self.message_list.push(err);
                return;
            }
        };

        // Push empty assistant placeholder — chunks stream into it
        let placeholder = create_assistant_message("", "build", "claude", None);
        self.messages.push(placeholder.clone());
        self.message_list.push(placeholder);
        self.stream_started = Some(std::time::Instant::now());

        // Update UI state
        self.set_streaming(true);

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
                            Err(e) => { let _ = tx.send(StreamChunk::Error(e.to_string())); return; }
                            Ok(b) => {
                                buf.push_str(&String::from_utf8_lossy(&b));
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

    fn set_streaming(&mut self, on: bool) {
        self.header.set_streaming(on);
        self.footer.set_streaming(on);
        self.input_component.set_streaming(on);
        if on {
            self.status = "Streaming…".to_string();
            let agent = self.dialog_agent.current_agent().unwrap_or("build").to_string();
            let model = shorten_model_id(&self.header.model);
            self.footer.set_agent(agent);
            self.footer.set_model_short(model);
        }
    }

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
                    let elapsed = self.stream_started.take().map(|s| s.elapsed().as_millis() as u64);
                    if let Some(msg) = self.messages.last_mut() {
                        if let MessageRole::Assistant { ref mut duration_ms, .. } = msg.role {
                            *duration_ms = elapsed;
                        }
                    }
                    self.message_list.update_last_duration(elapsed);
                    self.stream_rx = None;
                    self.status = "Ready".to_string();
                    self.footer.set_status("Ready".into());
                    self.set_streaming(false);
                    break;
                }
                Ok(StreamChunk::Error(e)) => {
                    let err = create_assistant_message(&format!("Error: {}", e), "system", "internal", None);
                    self.messages.push(err.clone());
                    self.message_list.push(err);
                    self.stream_rx = None;
                    self.status = "Error".to_string();
                    self.footer.set_status("Error".into());
                    self.set_streaming(false);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.stream_rx = None;
                    self.set_streaming(false);
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
        // (toast system retained for backward compat with events.rs)
    }

    pub async fn fetch_lsp(&mut self) {
        if let Ok(resp) = self.client.get(format!("{}/lsp", SERVER_URL)).send().await {
            if let Ok(list) = resp.json::<Vec<serde_json::Value>>().await {
                self.lsp_count = list.len();
            }
        }
    }

    pub async fn fetch_permissions(&mut self) {
        if let Ok(resp) = self.client.get(format!("{}/permission", SERVER_URL)).send().await {
            if let Ok(perms) = resp.json::<Vec<PermissionRequest>>().await {
                self.pending_permissions = perms.len();
                self.footer.set_pending_permissions(self.pending_permissions);
            }
        }
    }
}

// ── Public entry point ────────────────────────────────────────────────────────

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

    if let Err(e) = result { eprintln!("Error: {:?}", e); }
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
        terminal.draw(|f| ui(f, app))?;

        let now = std::time::Instant::now();
        let delta = now.duration_since(last_update);
        last_update = now;

        app.input_component.update(delta);
        app.header.update(delta);
        app.footer.update(delta);
        app.process_expired_toasts();

        // ── Esc stage timeout (3s) ───────────────────────────────────────
        if app.esc_stage > 0 {
            if let Some(t) = app.esc_stage_time {
                if t.elapsed() > std::time::Duration::from_secs(3) {
                    app.esc_stage = 0;
                    app.esc_stage_time = None;
                }
            }
        }

        // ── Terminal title ────────────────────────────────────────────────
        let title = if let Some(ref sid) = app.session_id {
            format!("roc | {} ({})", app.session_title, &sid[..sid.len().min(8)])
        } else {
            "roc".to_string()
        };
        let _ = execute!(io::stdout(), crossterm::terminal::SetTitle(&title));

        // ── Per-frame state sync ──────────────────────────────────────────
        // Slash autocomplete filter update
        app.slash_autocomplete.update_filter(&app.input_component.text());
        // Footer enriched status
        app.footer.set_mode_slash(app.input_component.starts_with_slash());
        app.footer.set_token_pct(app.context_info.percentage.unwrap_or(0));
        // Sync sidebar context info
        if app.left_sidebar.is_visible() {
            app.left_sidebar.set_agent(
                app.dialog_agent.current_agent().unwrap_or("build").to_string(),
                app.header.model.clone(),
            );
            app.left_sidebar.set_token_pct(app.context_info.percentage.unwrap_or(0));
        }

        // Sync right sidebar
        if app.right_sidebar.is_expanded() {
            app.right_sidebar.set_context(app.context_info.clone());
            app.right_sidebar.set_session_title(app.session_title.clone());
            let servers: Vec<McpServerInfo> = app
                .mcp_statuses
                .iter()
                .map(|(name, status)| McpServerInfo::new(name.clone(), status.clone(), 0))
                .collect();
            app.right_sidebar.set_mcp_servers(servers);
        }

        // ── Pending slash-command actions ─────────────────────────────────
        if app.pending_new_session {
            app.pending_new_session = false;
            let _title = app.pending_new_title.take();
            app.init_session().await;
        }
        if let Some(sess_id) = app.left_sidebar.take_pending_select() {
            app.session_id = Some(sess_id.clone());
            app.messages.clear();
            app.message_list.clear();
            app.status = "Session switched".to_string();
            app.footer.set_status("Session switched".to_string());
            app.header.set_session_title(&sess_id[..8.min(sess_id.len())]);
            app.input_component.on_focus();
        }

        let poll_ms = if app.stream_rx.is_some() { 33 } else { 100 };

        if crossterm::event::poll(std::time::Duration::from_millis(poll_ms))? {
            loop {
                match event::read()? {
                    Event::Key(key) => {
                        if let Some(msg) = app.handle_key(key) {
                            pending_message = Some(msg);
                        }
                    }
                    Event::Resize(_, _) => {}
                    _ => {}
                }
                if !crossterm::event::poll(std::time::Duration::from_millis(0))? { break; }
            }
        }

        if let Some(msg) = pending_message.take() {
            app.send_message(msg).await;
        }
    }

    Ok(())
}

// ── Render ────────────────────────────────────────────────────────────────────

fn ui(f: &mut Frame, app: &mut TuiApp) {
    let area = f.area();

    // Fill background
    use ratatui::style::Style;
    use ratatui::widgets::Block;
    f.render_widget(
        Block::default().style(Style::default().bg(ratatui::style::Color::Rgb(10, 10, 10))),
        area,
    );

    // Horizontal split: left sidebar | content | right sidebar
    let left_visible = app.left_sidebar.is_visible();
    let right_visible = app.right_sidebar.is_expanded();

    let (left_area, content_area, right_area) = match (left_visible, right_visible) {
        (true, true) => {
            let h = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(32), Constraint::Min(20), Constraint::Length(40)])
                .split(area);
            (Some(h[0]), h[1], Some(h[2]))
        }
        (true, false) => {
            let h = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(32), Constraint::Min(20)])
                .split(area);
            (Some(h[0]), h[1], None)
        }
        (false, true) => {
            let h = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(20), Constraint::Length(40)])
                .split(area);
            (None, h[0], Some(h[1]))
        }
        (false, false) => {
            (None, area, None)
        }
    };

    // 4-zone vertical layout on content_area
    let zones = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(3),    // messages
            Constraint::Length(5), // composer (border + 3 content + border)
            Constraint::Length(1), // footer
        ])
        .split(content_area);

    let [header_area, messages_area, composer_area, footer_area] =
        [zones[0], zones[1], zones[2], zones[3]];

    if let Some(sa) = left_area {
        app.left_sidebar.render(f, sa);
    }
    if let Some(ra) = right_area {
        app.right_sidebar.render(f, ra);
    }
    render_header(f, app, header_area);
    render_messages(f, app, messages_area);
    render_composer(f, app, composer_area);
    if app.slash_autocomplete.visible {
        app.slash_autocomplete.render(f, composer_area);
    }
    render_footer(f, app, footer_area);

    // Overlay dialogs (rendered on top of everything)
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

fn render_header(f: &mut Frame, app: &TuiApp, area: Rect) {
    app.header.render(f, area);
}

fn render_messages(f: &mut Frame, app: &TuiApp, area: Rect) {
    app.message_list.render(f, area);
}

fn render_composer(f: &mut Frame, app: &mut TuiApp, area: Rect) {
    // Add 1-col horizontal margin so the rounded border doesn't touch the edge
    let inner = Rect {
        x: area.x + 1,
        y: area.y,
        width: area.width.saturating_sub(2),
        height: area.height,
    };
    app.input_component.render(f, inner);
}

fn render_footer(f: &mut Frame, app: &TuiApp, area: Rect) {
    app.footer.render(f, area);
}

fn shorten_model_id(s: &str) -> String {
    if s.is_empty() { return String::new(); }
    let s = s.replace("claude-", "cl-");
    if s.len() > 14 { s[..14].to_string() } else { s }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_new_is_running() {
        assert!(TuiApp::new().running);
    }

    #[test]
    fn app_default_no_session() {
        assert_eq!(TuiApp::default().session_id, None);
    }

    #[test]
    fn session_info_fields() {
        let info = SessionInfo {
            id: "abc".into(), title: "T".into(),
            created_at: "2024".into(), updated_at: "2024".into(),
            status: "active".into(), message_count: 1,
        };
        assert_eq!(info.id, "abc");
    }
}
