//! TUI command - Terminal User Interface

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io;

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

/// TUI application state
pub struct TuiApp {
    pub running: bool,
    pub input: String,
    pub messages: Vec<String>,
    pub files: Vec<String>,
    pub client: Client,
    pub session_id: Option<String>,
    pub status: String,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            running: true,
            input: String::new(),
            messages: vec!["Welcome to ReOpenCode!".to_string()],
            files: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            client: Client::new(),
            session_id: None,
            status: "Connecting...".to_string(),
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
        
        match self.client
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
                            self.messages.push(format!("[System] Session created: {}", session.id));
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
                self.messages.push("[Error] Could not connect to server. Is it running?".to_string());
            }
        }
    }
    
    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.running = false,
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
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            _ => {}
        }
    }

    /// Send message to server (async, called from main loop)
    pub async fn send_message(&mut self, content: String) {
        if let Some(ref session_id) = self.session_id {
            let request = SendMessageRequest { content };
            
            match self.client
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
                        self.messages.push(format!("[Error] Server returned: {}", response.status()));
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
    
    while app.running {
        terminal.draw(|f| ui(f, app))?;
        
        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Enter && !app.input.is_empty() {
                    pending_message = Some(app.input.clone());
                }
                app.handle_key(key.code);
            }
        }
        
        if let Some(msg) = pending_message.take() {
            app.send_message(msg).await;
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
    let messages = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Messages"));
    f.render_widget(messages, chunks[2]);
    
    let input = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[3]);
    
    let files: Vec<ListItem> = app
        .files
        .iter()
        .map(|f| ListItem::new(Line::from(Span::styled(
            f.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ))))
        .collect();
    let files = List::new(files)
        .block(Block::default().borders(Borders::ALL).title("Files"));
    f.render_widget(files, chunks[4]);
}
