//! TUI Keyboard Bindings
//!
//! Configurable keyboard binding system with support for:
//! - Modifier keys (Ctrl, Alt, Shift, Super)
//! - Leader key prefix
//! - Custom configuration via TOML

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, Instant};

/// Keyboard binding information
/// Corresponds to TypeScript: Info (packages/opencode/src/util/keybind.ts)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct KeybindInfo {
    /// Key name: "c", "return", "escape", "space", "backspace", etc.
    #[serde(rename = "name")]
    pub name: String,

    /// Ctrl modifier
    #[serde(default)]
    pub ctrl: bool,

    /// Alt/Meta/Option modifier
    #[serde(default, alias = "alt")]
    pub meta: bool,

    /// Shift modifier
    #[serde(default)]
    pub shift: bool,

    /// Super/Windows/Cmd modifier (use super_ since super is keyword)
    #[serde(default, rename = "super")]
    pub super_: bool,

    /// Leader key prefix (custom prefix like ctrl+x)
    #[serde(default)]
    pub leader: bool,
}

impl KeybindInfo {
    /// Create a new keybind info with the given key name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Parse a keybind string like "ctrl+c,<leader>q" into a list of KeybindInfo
    /// Returns empty vec for "none"
    ///
    /// # Examples
    /// ```
    /// assert_eq!(KeybindInfo::parse("none"), vec![]);
    /// assert_eq!(KeybindInfo::parse("c"), vec![KeybindInfo::new("c")]);
    /// assert_eq!(KeybindInfo::parse("ctrl+c"), vec![KeybindInfo::new("c").with_ctrl(true)]);
    /// ```
    pub fn parse(key: &str) -> Vec<Self> {
        if key.eq_ignore_ascii_case("none") {
            return vec![];
        }

        key.split(',')
            .map(|combo| {
                // Handle <leader> syntax by replacing with leader+
                let normalized = combo.replace("<leader>", "leader+");
                let normalized_lower = normalized.to_lowercase();
                let parts: Vec<String> =
                    normalized_lower.split('+').map(|s| s.to_string()).collect();

                let mut info = KeybindInfo::default();

                for part in &parts {
                    let part = part.as_str();
                    match part {
                        "ctrl" => info.ctrl = true,
                        "alt" | "meta" | "option" => info.meta = true,
                        "super" => info.super_ = true,
                        "shift" => info.shift = true,
                        "leader" => info.leader = true,
                        "esc" => info.name = "escape".to_string(),
                        " " | "space" => info.name = "space".to_string(),
                        "return" | "enter" => info.name = "return".to_string(),
                        "backspace" => info.name = "backspace".to_string(),
                        "delete" | "del" => info.name = "delete".to_string(),
                        "tab" => info.name = "tab".to_string(),
                        "escape" => info.name = "escape".to_string(),
                        "home" => info.name = "home".to_string(),
                        "end" => info.name = "end".to_string(),
                        "pageup" | "pgup" => info.name = "pageup".to_string(),
                        "pagedown" | "pgdn" => info.name = "pagedown".to_string(),
                        "up" => info.name = "up".to_string(),
                        "down" => info.name = "down".to_string(),
                        "left" => info.name = "left".to_string(),
                        "right" => info.name = "right".to_string(),
                        // Function keys f1-f12 (and beyond)
                        part if part.len() >= 2 && part.starts_with('f') => {
                            info.name = part.to_string();
                        }
                        // Everything else is the key name
                        _ => info.name = part.to_string(),
                    }
                }

                info
            })
            .collect()
    }

    /// Set the ctrl modifier
    pub fn with_ctrl(mut self, ctrl: bool) -> Self {
        self.ctrl = ctrl;
        self
    }

    /// Set the meta (alt) modifier
    pub fn with_meta(mut self, meta: bool) -> Self {
        self.meta = meta;
        self
    }

    /// Set the shift modifier
    pub fn with_shift(mut self, shift: bool) -> Self {
        self.shift = shift;
        self
    }

    /// Set the super modifier
    pub fn with_super(mut self, super_: bool) -> Self {
        self.super_ = super_;
        self
    }

    /// Set the leader flag
    pub fn with_leader(mut self, leader: bool) -> Self {
        self.leader = leader;
        self
    }

    /// Convert a crossterm KeyEvent to KeybindInfo
    /// Note: leader is always false since crossterm doesn't know about leader keys
    pub fn from_crossterm(evt: &KeyEvent) -> Self {
        Self {
            ctrl: evt.modifiers.contains(KeyModifiers::CONTROL),
            meta: evt.modifiers.contains(KeyModifiers::ALT),
            shift: evt.modifiers.contains(KeyModifiers::SHIFT),
            super_: evt.modifiers.contains(KeyModifiers::SUPER),
            name: match evt.code {
                KeyCode::Char(c) => {
                    if c == ' ' {
                        "space".to_string()
                    } else {
                        c.to_lowercase().to_string()
                    }
                }
                KeyCode::Enter => "return".to_string(),
                KeyCode::Backspace => "backspace".to_string(),
                KeyCode::Esc => "escape".to_string(),
                KeyCode::Tab => "tab".to_string(),
                KeyCode::Delete => "delete".to_string(),
                KeyCode::Home => "home".to_string(),
                KeyCode::End => "end".to_string(),
                KeyCode::PageUp => "pageup".to_string(),
                KeyCode::PageDown => "pagedown".to_string(),
                KeyCode::Up => "up".to_string(),
                KeyCode::Down => "down".to_string(),
                KeyCode::Left => "left".to_string(),
                KeyCode::Right => "right".to_string(),
                KeyCode::F(n) => format!("f{}", n),
                KeyCode::Insert => "insert".to_string(),
                KeyCode::Null => String::new(),
                KeyCode::BackTab => "backtab".to_string(),
                _ => String::new(),
            },
            leader: false,
        }
    }

    /// Check if two KeybindInfo match (ignoring super default)
    pub fn matches(&self, other: &Self) -> bool {
        self.name == other.name
            && self.ctrl == other.ctrl
            && self.meta == other.meta
            && self.shift == other.shift
            && self.super_ == other.super_
            && self.leader == other.leader
    }
}

impl fmt::Display for KeybindInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if self.ctrl {
            parts.push("ctrl");
        }
        if self.meta {
            parts.push("alt");
        }
        if self.super_ {
            parts.push("super");
        }
        if self.shift {
            parts.push("shift");
        }

        parts.push(&self.name);

        let result = parts.join("+");

        if self.leader {
            write!(f, "<leader> {}", result)
        } else {
            write!(f, "{}", result)
        }
    }
}

impl KeybindsConfig {
    /// Check if a key event matches any of the alternatives for an action
    pub fn matches(&self, action: &str, evt: &KeybindInfo, leader_active: bool) -> bool {
        let keybind_str = match action {
            "leader" => &self.leader,
            "app_exit" => &self.app_exit,
            "editor_open" => &self.editor_open,
            "theme_list" => &self.theme_list,
            "sidebar_toggle" => &self.sidebar_toggle,
            "scrollbar_toggle" => &self.scrollbar_toggle,
            "username_toggle" => &self.username_toggle,
            "status_view" => &self.status_view,
            "mcp_status_toggle" => &self.mcp_status_toggle,
            "session_export" => &self.session_export,
            "session_new" => &self.session_new,
            "session_list" => &self.session_list,
            "session_timeline" => &self.session_timeline,
            "session_fork" => &self.session_fork,
            "session_rename" => &self.session_rename,
            "session_delete" => &self.session_delete,
            "stash_delete" => &self.stash_delete,
            "model_provider_list" => &self.model_provider_list,
            "model_favorite_toggle" => &self.model_favorite_toggle,
            "session_share" => &self.session_share,
            "session_unshare" => &self.session_unshare,
            "session_interrupt" => &self.session_interrupt,
            "session_compact" => &self.session_compact,
            "messages_page_up" => &self.messages_page_up,
            "messages_page_down" => &self.messages_page_down,
            "messages_line_up" => &self.messages_line_up,
            "messages_line_down" => &self.messages_line_down,
            "messages_half_page_up" => &self.messages_half_page_up,
            "messages_half_page_down" => &self.messages_half_page_down,
            "messages_first" => &self.messages_first,
            "messages_last" => &self.messages_last,
            "messages_next" => &self.messages_next,
            "messages_previous" => &self.messages_previous,
            "messages_last_user" => &self.messages_last_user,
            "messages_copy" => &self.messages_copy,
            "messages_undo" => &self.messages_undo,
            "messages_redo" => &self.messages_redo,
            "messages_toggle_conceal" => &self.messages_toggle_conceal,
            "tool_details" => &self.tool_details,
            "model_list" => &self.model_list,
            "model_cycle_recent" => &self.model_cycle_recent,
            "model_cycle_recent_reverse" => &self.model_cycle_recent_reverse,
            "model_cycle_favorite" => &self.model_cycle_favorite,
            "model_cycle_favorite_reverse" => &self.model_cycle_favorite_reverse,
            "command_list" => &self.command_list,
            "agent_list" => &self.agent_list,
            "agent_cycle" => &self.agent_cycle,
            "agent_cycle_reverse" => &self.agent_cycle_reverse,
            "variant_cycle" => &self.variant_cycle,
            "input_clear" => &self.input_clear,
            "input_paste" => &self.input_paste,
            "input_submit" => &self.input_submit,
            "input_newline" => &self.input_newline,
            "input_move_left" => &self.input_move_left,
            "input_move_right" => &self.input_move_right,
            "input_move_up" => &self.input_move_up,
            "input_move_down" => &self.input_move_down,
            "input_select_left" => &self.input_select_left,
            "input_select_right" => &self.input_select_right,
            "input_select_up" => &self.input_select_up,
            "input_select_down" => &self.input_select_down,
            "input_line_home" => &self.input_line_home,
            "input_line_end" => &self.input_line_end,
            "input_select_line_home" => &self.input_select_line_home,
            "input_select_line_end" => &self.input_select_line_end,
            "input_visual_line_home" => &self.input_visual_line_home,
            "input_visual_line_end" => &self.input_visual_line_end,
            "input_select_visual_line_home" => &self.input_select_visual_line_home,
            "input_select_visual_line_end" => &self.input_select_visual_line_end,
            "input_buffer_home" => &self.input_buffer_home,
            "input_buffer_end" => &self.input_buffer_end,
            "input_select_buffer_home" => &self.input_select_buffer_home,
            "input_select_buffer_end" => &self.input_select_buffer_end,
            "input_delete_line" => &self.input_delete_line,
            "input_delete_to_line_end" => &self.input_delete_to_line_end,
            "input_delete_to_line_start" => &self.input_delete_to_line_start,
            "input_backspace" => &self.input_backspace,
            "input_delete" => &self.input_delete,
            "input_undo" => &self.input_undo,
            "input_redo" => &self.input_redo,
            "input_word_forward" => &self.input_word_forward,
            "input_word_backward" => &self.input_word_backward,
            "input_select_word_forward" => &self.input_select_word_forward,
            "input_select_word_backward" => &self.input_select_word_backward,
            "input_delete_word_forward" => &self.input_delete_word_forward,
            "input_delete_word_backward" => &self.input_delete_word_backward,
            "history_previous" => &self.history_previous,
            "history_next" => &self.history_next,
            "session_child_first" => &self.session_child_first,
            "session_child_cycle" => &self.session_child_cycle,
            "session_child_cycle_reverse" => &self.session_child_cycle_reverse,
            "session_parent" => &self.session_parent,
            "terminal_suspend" => &self.terminal_suspend,
            "terminal_title_toggle" => &self.terminal_title_toggle,
            "tips_toggle" => &self.tips_toggle,
            "display_thinking" => &self.display_thinking,
            _ => return false,
        };

        let alternatives = KeybindInfo::parse(keybind_str);
        for alt in alternatives {
            if alt.leader {
                if leader_active
                    && alt.name == evt.name
                    && alt.ctrl == evt.ctrl
                    && alt.meta == evt.meta
                    && alt.shift == evt.shift
                    && alt.super_ == evt.super_
                {
                    return true;
                }
            } else if !leader_active && alt.matches(evt) {
                return true;
            }
        }
        false
    }
}

fn default_leader() -> String {
    "ctrl+x".to_string()
}

fn default_app_exit() -> String {
    "ctrl+c,ctrl+d,<leader>q".to_string()
}

fn default_editor_open() -> String {
    "<leader>e".to_string()
}

fn default_theme_list() -> String {
    "<leader>t".to_string()
}

fn default_sidebar_toggle() -> String {
    "<leader>b".to_string()
}

fn default_scrollbar_toggle() -> String {
    "none".to_string()
}

fn default_username_toggle() -> String {
    "none".to_string()
}

fn default_status_view() -> String {
    "<leader>s".to_string()
}

fn default_mcp_status_toggle() -> String {
    "ctrl+m".to_string()
}

fn default_session_export() -> String {
    "<leader>x".to_string()
}

fn default_session_new() -> String {
    "<leader>n".to_string()
}

fn default_session_list() -> String {
    "ctrl+p".to_string()
}

fn default_session_timeline() -> String {
    "<leader>g".to_string()
}

fn default_session_fork() -> String {
    "none".to_string()
}

fn default_session_rename() -> String {
    "ctrl+r".to_string()
}

fn default_session_delete() -> String {
    "ctrl+d".to_string()
}

fn default_stash_delete() -> String {
    "ctrl+d".to_string()
}

fn default_model_provider_list() -> String {
    "ctrl+a".to_string()
}

fn default_model_favorite_toggle() -> String {
    "ctrl+f".to_string()
}

fn default_session_share() -> String {
    "none".to_string()
}

fn default_session_unshare() -> String {
    "none".to_string()
}

fn default_session_interrupt() -> String {
    "escape".to_string()
}

fn default_session_compact() -> String {
    "<leader>c".to_string()
}

fn default_messages_page_up() -> String {
    "pageup,ctrl+alt+b".to_string()
}

fn default_messages_page_down() -> String {
    "pagedown,ctrl+alt+f".to_string()
}

fn default_messages_line_up() -> String {
    "ctrl+alt+y".to_string()
}

fn default_messages_line_down() -> String {
    "ctrl+alt+e".to_string()
}

fn default_messages_half_page_up() -> String {
    "ctrl+alt+u".to_string()
}

fn default_messages_half_page_down() -> String {
    "ctrl+alt+d".to_string()
}

fn default_messages_first() -> String {
    "ctrl+g,home".to_string()
}

fn default_messages_last() -> String {
    "ctrl+alt+g,end".to_string()
}

fn default_messages_next() -> String {
    "none".to_string()
}

fn default_messages_previous() -> String {
    "none".to_string()
}

fn default_messages_last_user() -> String {
    "none".to_string()
}

fn default_messages_copy() -> String {
    "<leader>y".to_string()
}

fn default_messages_undo() -> String {
    "<leader>u".to_string()
}

fn default_messages_redo() -> String {
    "<leader>r".to_string()
}

fn default_messages_toggle_conceal() -> String {
    "<leader>h".to_string()
}

fn default_tool_details() -> String {
    "none".to_string()
}

fn default_model_list() -> String {
    "<leader>m".to_string()
}

fn default_model_cycle_recent() -> String {
    "f2".to_string()
}

fn default_model_cycle_recent_reverse() -> String {
    "shift+f2".to_string()
}

fn default_model_cycle_favorite() -> String {
    "none".to_string()
}

fn default_model_cycle_favorite_reverse() -> String {
    "none".to_string()
}

fn default_command_list() -> String {
    "none".to_string()
}

fn default_agent_list() -> String {
    "<leader>a".to_string()
}

fn default_agent_cycle() -> String {
    "tab".to_string()
}

fn default_agent_cycle_reverse() -> String {
    "shift+tab".to_string()
}

fn default_variant_cycle() -> String {
    "ctrl+t".to_string()
}

fn default_input_clear() -> String {
    "ctrl+c".to_string()
}

fn default_input_paste() -> String {
    "ctrl+v".to_string()
}

fn default_input_submit() -> String {
    "return".to_string()
}

fn default_input_newline() -> String {
    "shift+return,ctrl+return,alt+return,ctrl+j".to_string()
}

fn default_input_move_left() -> String {
    "left,ctrl+b".to_string()
}

fn default_input_move_right() -> String {
    "right,ctrl+f".to_string()
}

fn default_input_move_up() -> String {
    "up".to_string()
}

fn default_input_move_down() -> String {
    "down".to_string()
}

fn default_input_select_left() -> String {
    "shift+left".to_string()
}

fn default_input_select_right() -> String {
    "shift+right".to_string()
}

fn default_input_select_up() -> String {
    "shift+up".to_string()
}

fn default_input_select_down() -> String {
    "shift+down".to_string()
}

fn default_input_line_home() -> String {
    "ctrl+a".to_string()
}

fn default_input_line_end() -> String {
    "ctrl+e".to_string()
}

fn default_input_select_line_home() -> String {
    "ctrl+shift+a".to_string()
}

fn default_input_select_line_end() -> String {
    "ctrl+shift+e".to_string()
}

fn default_input_visual_line_home() -> String {
    "alt+a".to_string()
}

fn default_input_visual_line_end() -> String {
    "alt+e".to_string()
}

fn default_input_select_visual_line_home() -> String {
    "alt+shift+a".to_string()
}

fn default_input_select_visual_line_end() -> String {
    "alt+shift+e".to_string()
}

fn default_input_buffer_home() -> String {
    "home".to_string()
}

fn default_input_buffer_end() -> String {
    "end".to_string()
}

fn default_input_select_buffer_home() -> String {
    "shift+home".to_string()
}

fn default_input_select_buffer_end() -> String {
    "shift+end".to_string()
}

fn default_input_delete_line() -> String {
    "ctrl+shift+d".to_string()
}

fn default_input_delete_to_line_end() -> String {
    "ctrl+k".to_string()
}

fn default_input_delete_to_line_start() -> String {
    "ctrl+u".to_string()
}

fn default_input_backspace() -> String {
    "backspace,shift+backspace".to_string()
}

fn default_input_delete() -> String {
    "ctrl+d,delete,shift+delete".to_string()
}

fn default_input_undo() -> String {
    "ctrl+-,super+z".to_string()
}

fn default_input_redo() -> String {
    "ctrl+.,super+shift+z".to_string()
}

fn default_input_word_forward() -> String {
    "alt+f,alt+right,ctrl+right".to_string()
}

fn default_input_word_backward() -> String {
    "alt+b,alt+left,ctrl+left".to_string()
}

fn default_input_select_word_forward() -> String {
    "alt+shift+f,alt+shift+right".to_string()
}

fn default_input_select_word_backward() -> String {
    "alt+shift+b,alt+shift+left".to_string()
}

fn default_input_delete_word_forward() -> String {
    "alt+d,alt+delete,ctrl+delete".to_string()
}

fn default_input_delete_word_backward() -> String {
    "ctrl+w,ctrl+backspace,alt+backspace".to_string()
}

fn default_history_previous() -> String {
    "up".to_string()
}

fn default_history_next() -> String {
    "down".to_string()
}

fn default_session_child_first() -> String {
    "<leader>down".to_string()
}

fn default_session_child_cycle() -> String {
    "right".to_string()
}

fn default_session_child_cycle_reverse() -> String {
    "left".to_string()
}

fn default_session_parent() -> String {
    "up".to_string()
}

fn default_terminal_suspend() -> String {
    "ctrl+z".to_string()
}

fn default_terminal_title_toggle() -> String {
    "none".to_string()
}

fn default_tips_toggle() -> String {
    "<leader>h".to_string()
}

fn default_display_thinking() -> String {
    "none".to_string()
}

/// Configuration for all TUI keyboard bindings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KeybindsConfig {
    // Application-level
    #[serde(default = "default_leader")]
    pub leader: String,
    #[serde(default = "default_app_exit")]
    pub app_exit: String,
    #[serde(default = "default_editor_open")]
    pub editor_open: String,
    #[serde(default = "default_theme_list")]
    pub theme_list: String,
    #[serde(default = "default_sidebar_toggle")]
    pub sidebar_toggle: String,
    #[serde(default = "default_scrollbar_toggle")]
    pub scrollbar_toggle: String,
    #[serde(default = "default_username_toggle")]
    pub username_toggle: String,
    #[serde(default = "default_status_view")]
    pub status_view: String,
    #[serde(default = "default_mcp_status_toggle")]
    pub mcp_status_toggle: String,

    // Session management
    #[serde(default = "default_session_export")]
    pub session_export: String,
    #[serde(default = "default_session_new")]
    pub session_new: String,
    #[serde(default = "default_session_list")]
    pub session_list: String,
    #[serde(default = "default_session_timeline")]
    pub session_timeline: String,
    #[serde(default = "default_session_fork")]
    pub session_fork: String,
    #[serde(default = "default_session_rename")]
    pub session_rename: String,
    #[serde(default = "default_session_delete")]
    pub session_delete: String,
    #[serde(default = "default_stash_delete")]
    pub stash_delete: String,
    #[serde(default = "default_session_share")]
    pub session_share: String,
    #[serde(default = "default_session_unshare")]
    pub session_unshare: String,
    #[serde(default = "default_session_interrupt")]
    pub session_interrupt: String,
    #[serde(default = "default_session_compact")]
    pub session_compact: String,

    // Model selection
    #[serde(default = "default_model_provider_list")]
    pub model_provider_list: String,
    #[serde(default = "default_model_favorite_toggle")]
    pub model_favorite_toggle: String,
    #[serde(default = "default_model_list")]
    pub model_list: String,
    #[serde(default = "default_model_cycle_recent")]
    pub model_cycle_recent: String,
    #[serde(default = "default_model_cycle_recent_reverse")]
    pub model_cycle_recent_reverse: String,
    #[serde(default = "default_model_cycle_favorite")]
    pub model_cycle_favorite: String,
    #[serde(default = "default_model_cycle_favorite_reverse")]
    pub model_cycle_favorite_reverse: String,

    // Message navigation
    #[serde(default = "default_messages_page_up")]
    pub messages_page_up: String,
    #[serde(default = "default_messages_page_down")]
    pub messages_page_down: String,
    #[serde(default = "default_messages_line_up")]
    pub messages_line_up: String,
    #[serde(default = "default_messages_line_down")]
    pub messages_line_down: String,
    #[serde(default = "default_messages_half_page_up")]
    pub messages_half_page_up: String,
    #[serde(default = "default_messages_half_page_down")]
    pub messages_half_page_down: String,
    #[serde(default = "default_messages_first")]
    pub messages_first: String,
    #[serde(default = "default_messages_last")]
    pub messages_last: String,
    #[serde(default = "default_messages_next")]
    pub messages_next: String,
    #[serde(default = "default_messages_previous")]
    pub messages_previous: String,
    #[serde(default = "default_messages_last_user")]
    pub messages_last_user: String,
    #[serde(default = "default_messages_copy")]
    pub messages_copy: String,
    #[serde(default = "default_messages_undo")]
    pub messages_undo: String,
    #[serde(default = "default_messages_redo")]
    pub messages_redo: String,
    #[serde(default = "default_messages_toggle_conceal")]
    pub messages_toggle_conceal: String,
    #[serde(default = "default_tool_details")]
    pub tool_details: String,

    // Command & Agent
    #[serde(default = "default_command_list")]
    pub command_list: String,
    #[serde(default = "default_agent_list")]
    pub agent_list: String,
    #[serde(default = "default_agent_cycle")]
    pub agent_cycle: String,
    #[serde(default = "default_agent_cycle_reverse")]
    pub agent_cycle_reverse: String,
    #[serde(default = "default_variant_cycle")]
    pub variant_cycle: String,

    // Input - Basic
    #[serde(default = "default_input_clear")]
    pub input_clear: String,
    #[serde(default = "default_input_paste")]
    pub input_paste: String,
    #[serde(default = "default_input_submit")]
    pub input_submit: String,
    #[serde(default = "default_input_newline")]
    pub input_newline: String,

    // Input - Movement
    #[serde(default = "default_input_move_left")]
    pub input_move_left: String,
    #[serde(default = "default_input_move_right")]
    pub input_move_right: String,
    #[serde(default = "default_input_move_up")]
    pub input_move_up: String,
    #[serde(default = "default_input_move_down")]
    pub input_move_down: String,

    // Input - Selection
    #[serde(default = "default_input_select_left")]
    pub input_select_left: String,
    #[serde(default = "default_input_select_right")]
    pub input_select_right: String,
    #[serde(default = "default_input_select_up")]
    pub input_select_up: String,
    #[serde(default = "default_input_select_down")]
    pub input_select_down: String,

    // Input - Line navigation
    #[serde(default = "default_input_line_home")]
    pub input_line_home: String,
    #[serde(default = "default_input_line_end")]
    pub input_line_end: String,
    #[serde(default = "default_input_select_line_home")]
    pub input_select_line_home: String,
    #[serde(default = "default_input_select_line_end")]
    pub input_select_line_end: String,
    #[serde(default = "default_input_visual_line_home")]
    pub input_visual_line_home: String,
    #[serde(default = "default_input_visual_line_end")]
    pub input_visual_line_end: String,
    #[serde(default = "default_input_select_visual_line_home")]
    pub input_select_visual_line_home: String,
    #[serde(default = "default_input_select_visual_line_end")]
    pub input_select_visual_line_end: String,

    // Input - Buffer navigation
    #[serde(default = "default_input_buffer_home")]
    pub input_buffer_home: String,
    #[serde(default = "default_input_buffer_end")]
    pub input_buffer_end: String,
    #[serde(default = "default_input_select_buffer_home")]
    pub input_select_buffer_home: String,
    #[serde(default = "default_input_select_buffer_end")]
    pub input_select_buffer_end: String,

    // Input - Delete
    #[serde(default = "default_input_delete_line")]
    pub input_delete_line: String,
    #[serde(default = "default_input_delete_to_line_end")]
    pub input_delete_to_line_end: String,
    #[serde(default = "default_input_delete_to_line_start")]
    pub input_delete_to_line_start: String,
    #[serde(default = "default_input_backspace")]
    pub input_backspace: String,
    #[serde(default = "default_input_delete")]
    pub input_delete: String,

    // Input - Undo/Redo
    #[serde(default = "default_input_undo")]
    pub input_undo: String,
    #[serde(default = "default_input_redo")]
    pub input_redo: String,

    // Input - Word movement
    #[serde(default = "default_input_word_forward")]
    pub input_word_forward: String,
    #[serde(default = "default_input_word_backward")]
    pub input_word_backward: String,
    #[serde(default = "default_input_select_word_forward")]
    pub input_select_word_forward: String,
    #[serde(default = "default_input_select_word_backward")]
    pub input_select_word_backward: String,
    #[serde(default = "default_input_delete_word_forward")]
    pub input_delete_word_forward: String,
    #[serde(default = "default_input_delete_word_backward")]
    pub input_delete_word_backward: String,

    // Input - History
    #[serde(default = "default_history_previous")]
    pub history_previous: String,
    #[serde(default = "default_history_next")]
    pub history_next: String,

    // Session tree navigation
    #[serde(default = "default_session_child_first")]
    pub session_child_first: String,
    #[serde(default = "default_session_child_cycle")]
    pub session_child_cycle: String,
    #[serde(default = "default_session_child_cycle_reverse")]
    pub session_child_cycle_reverse: String,
    #[serde(default = "default_session_parent")]
    pub session_parent: String,

    // Terminal
    #[serde(default = "default_terminal_suspend")]
    pub terminal_suspend: String,
    #[serde(default = "default_terminal_title_toggle")]
    pub terminal_title_toggle: String,

    // Display
    #[serde(default = "default_tips_toggle")]
    pub tips_toggle: String,
    #[serde(default = "default_display_thinking")]
    pub display_thinking: String,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        Self {
            // Application-level
            leader: default_leader(),
            app_exit: default_app_exit(),
            editor_open: default_editor_open(),
            theme_list: default_theme_list(),
            sidebar_toggle: default_sidebar_toggle(),
            scrollbar_toggle: default_scrollbar_toggle(),
            username_toggle: default_username_toggle(),
            status_view: default_status_view(),
            mcp_status_toggle: default_mcp_status_toggle(),

            // Session management
            session_export: default_session_export(),
            session_new: default_session_new(),
            session_list: default_session_list(),
            session_timeline: default_session_timeline(),
            session_fork: default_session_fork(),
            session_rename: default_session_rename(),
            session_delete: default_session_delete(),
            stash_delete: default_stash_delete(),
            session_share: default_session_share(),
            session_unshare: default_session_unshare(),
            session_interrupt: default_session_interrupt(),
            session_compact: default_session_compact(),

            // Model selection
            model_provider_list: default_model_provider_list(),
            model_favorite_toggle: default_model_favorite_toggle(),
            model_list: default_model_list(),
            model_cycle_recent: default_model_cycle_recent(),
            model_cycle_recent_reverse: default_model_cycle_recent_reverse(),
            model_cycle_favorite: default_model_cycle_favorite(),
            model_cycle_favorite_reverse: default_model_cycle_favorite_reverse(),

            // Message navigation
            messages_page_up: default_messages_page_up(),
            messages_page_down: default_messages_page_down(),
            messages_line_up: default_messages_line_up(),
            messages_line_down: default_messages_line_down(),
            messages_half_page_up: default_messages_half_page_up(),
            messages_half_page_down: default_messages_half_page_down(),
            messages_first: default_messages_first(),
            messages_last: default_messages_last(),
            messages_next: default_messages_next(),
            messages_previous: default_messages_previous(),
            messages_last_user: default_messages_last_user(),
            messages_copy: default_messages_copy(),
            messages_undo: default_messages_undo(),
            messages_redo: default_messages_redo(),
            messages_toggle_conceal: default_messages_toggle_conceal(),
            tool_details: default_tool_details(),

            // Command & Agent
            command_list: default_command_list(),
            agent_list: default_agent_list(),
            agent_cycle: default_agent_cycle(),
            agent_cycle_reverse: default_agent_cycle_reverse(),
            variant_cycle: default_variant_cycle(),

            // Input - Basic
            input_clear: default_input_clear(),
            input_paste: default_input_paste(),
            input_submit: default_input_submit(),
            input_newline: default_input_newline(),

            // Input - Movement
            input_move_left: default_input_move_left(),
            input_move_right: default_input_move_right(),
            input_move_up: default_input_move_up(),
            input_move_down: default_input_move_down(),

            // Input - Selection
            input_select_left: default_input_select_left(),
            input_select_right: default_input_select_right(),
            input_select_up: default_input_select_up(),
            input_select_down: default_input_select_down(),

            // Input - Line navigation
            input_line_home: default_input_line_home(),
            input_line_end: default_input_line_end(),
            input_select_line_home: default_input_select_line_home(),
            input_select_line_end: default_input_select_line_end(),
            input_visual_line_home: default_input_visual_line_home(),
            input_visual_line_end: default_input_visual_line_end(),
            input_select_visual_line_home: default_input_select_visual_line_home(),
            input_select_visual_line_end: default_input_select_visual_line_end(),

            // Input - Buffer navigation
            input_buffer_home: default_input_buffer_home(),
            input_buffer_end: default_input_buffer_end(),
            input_select_buffer_home: default_input_select_buffer_home(),
            input_select_buffer_end: default_input_select_buffer_end(),

            // Input - Delete
            input_delete_line: default_input_delete_line(),
            input_delete_to_line_end: default_input_delete_to_line_end(),
            input_delete_to_line_start: default_input_delete_to_line_start(),
            input_backspace: default_input_backspace(),
            input_delete: default_input_delete(),

            // Input - Undo/Redo
            input_undo: default_input_undo(),
            input_redo: default_input_redo(),

            // Input - Word movement
            input_word_forward: default_input_word_forward(),
            input_word_backward: default_input_word_backward(),
            input_select_word_forward: default_input_select_word_forward(),
            input_select_word_backward: default_input_select_word_backward(),
            input_delete_word_forward: default_input_delete_word_forward(),
            input_delete_word_backward: default_input_delete_word_backward(),

            // Input - History
            history_previous: default_history_previous(),
            history_next: default_history_next(),

            // Session tree navigation
            session_child_first: default_session_child_first(),
            session_child_cycle: default_session_child_cycle(),
            session_child_cycle_reverse: default_session_child_cycle_reverse(),
            session_parent: default_session_parent(),

            // Terminal
            terminal_suspend: default_terminal_suspend(),
            terminal_title_toggle: default_terminal_title_toggle(),

            // Display
            tips_toggle: default_tips_toggle(),
            display_thinking: default_display_thinking(),
        }
    }
}

/// Default leader timeout duration (2 seconds)
pub const DEFAULT_LEADER_TIMEOUT: Duration = Duration::from_secs(2);

/// Leader key state management with timeout
///
/// The leader key works like a prefix key in vim:
/// 1. Press leader key (e.g., ctrl+x) to enter leader mode
/// 2. Has 2-second timeout to automatically cancel
/// 3. Press next key to complete the sequence (e.g., q for quit)
#[derive(Debug, Clone)]
pub struct LeaderState {
    /// Whether leader mode is currently active
    active: bool,

    /// When leader mode was activated (for timeout)
    activated_at: Option<Instant>,

    /// Timeout duration (default 2 seconds)
    timeout: Duration,

    /// The leader key binding (e.g., parsed from "ctrl+x")
    leader_key: KeybindInfo,
}

impl LeaderState {
    /// Create new LeaderState with given leader key string
    pub fn new(leader_key_str: &str) -> Self {
        let leader_key = KeybindInfo::parse(leader_key_str)
            .into_iter()
            .next()
            .unwrap_or_else(|| KeybindInfo::new("x").with_ctrl(true));

        Self {
            active: false,
            activated_at: None,
            timeout: DEFAULT_LEADER_TIMEOUT,
            leader_key,
        }
    }

    /// Create new LeaderState with custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if leader mode is currently active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Check if a key event is the leader key
    pub fn is_leader_key(&self, evt: &KeybindInfo) -> bool {
        // Compare all fields except leader flag
        evt.name == self.leader_key.name
            && evt.ctrl == self.leader_key.ctrl
            && evt.meta == self.leader_key.meta
            && evt.shift == self.leader_key.shift
            && evt.super_ == self.leader_key.super_
    }

    /// Activate leader mode, returns true if just activated
    /// Returns false if already active (no state change)
    pub fn activate(&mut self) -> bool {
        if self.active {
            return false;
        }
        self.active = true;
        self.activated_at = Some(Instant::now());
        true
    }

    /// Deactivate leader mode
    pub fn deactivate(&mut self) {
        self.active = false;
        self.activated_at = None;
    }

    /// Update state (check for timeout), returns true if timed out
    /// Automatically deactivates if timed out
    pub fn update(&mut self) -> bool {
        if !self.active {
            return false;
        }

        match self.activated_at {
            Some(activated) => {
                if activated.elapsed() >= self.timeout {
                    self.deactivate();
                    return true;
                }
                false
            }
            None => {
                // Should not happen, but deactivate if no activation time
                self.deactivate();
                true
            }
        }
    }

    /// Check if leader key was pressed and handle it
    /// Returns true if the event was consumed as leader key
    pub fn check_and_handle(&mut self, evt: &KeybindInfo) -> bool {
        if self.is_leader_key(evt) {
            self.activate();
            return true;
        }
        false
    }

    /// Get the leader key info
    pub fn leader_key(&self) -> &KeybindInfo {
        &self.leader_key
    }

    /// Get the timeout duration
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Get remaining time before timeout
    /// Returns None if not active
    pub fn remaining(&self) -> Option<Duration> {
        if !self.active {
            return None;
        }

        self.activated_at.map(|activated| {
            let elapsed = activated.elapsed();
            if elapsed >= self.timeout {
                Duration::ZERO
            } else {
                self.timeout - elapsed
            }
        })
    }
}

impl Default for LeaderState {
    fn default() -> Self {
        Self::new("ctrl+x") // Default leader key
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keybind_info_default() {
        let info = KeybindInfo::default();
        assert_eq!(info.name, "");
        assert!(!info.ctrl);
        assert!(!info.meta);
        assert!(!info.shift);
        assert!(!info.super_);
        assert!(!info.leader);
    }

    #[test]
    fn test_keybind_info_builder() {
        let info = KeybindInfo::new("c").with_ctrl(true).with_shift(true);

        assert_eq!(info.name, "c");
        assert!(info.ctrl);
        assert!(!info.meta);
        assert!(info.shift);
        assert!(!info.super_);
        assert!(!info.leader);
    }

    #[test]
    fn test_keybind_info_to_string_simple() {
        let info = KeybindInfo::new("c");
        assert_eq!(info.to_string(), "c");
    }

    #[test]
    fn test_keybind_info_to_string_with_modifiers() {
        let info = KeybindInfo::new("c").with_ctrl(true).with_shift(true);
        assert_eq!(info.to_string(), "ctrl+shift+c");
    }

    #[test]
    fn test_keybind_info_to_string_leader() {
        let info = KeybindInfo::new("q").with_leader(true);
        assert_eq!(info.to_string(), "<leader> q");
    }

    #[test]
    fn test_keybind_info_to_string_leader_with_modifiers() {
        let info = KeybindInfo::new("w").with_ctrl(true).with_leader(true);
        assert_eq!(info.to_string(), "<leader> ctrl+w");
    }

    #[test]
    fn test_keybind_info_serde_serialization() {
        let info = KeybindInfo::new("return").with_ctrl(true).with_meta(true);

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"name\":\"return\""));
        assert!(json.contains("\"ctrl\":true"));
        assert!(json.contains("\"meta\":true"));
        // super should serialize as "super"
        assert!(json.contains("\"super\":false"));
    }

    #[test]
    fn test_keybind_info_serde_deserialization() {
        let json = r#"{"name": "space", "ctrl": true, "shift": true}"#;
        let info: KeybindInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.name, "space");
        assert!(info.ctrl);
        assert!(info.shift);
        assert!(!info.meta);
        assert!(!info.super_);
        assert!(!info.leader);
    }

    #[test]
    fn test_keybind_info_serde_super_alias() {
        // Test that "super" serializes correctly
        let info = KeybindInfo::new("t").with_super(true);
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"super\":true"));
    }

    #[test]
    fn test_keybind_info_equality() {
        let info1 = KeybindInfo::new("c").with_ctrl(true).with_shift(true);

        let info2 = KeybindInfo::new("c").with_shift(true).with_ctrl(true);

        assert_eq!(info1, info2);
    }

    #[test]
    fn test_keybind_info_clone() {
        let info = KeybindInfo::new("a")
            .with_ctrl(true)
            .with_meta(true)
            .with_shift(true)
            .with_super(true)
            .with_leader(true);

        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn test_keybind_info_meta_alias() {
        // Test that we can deserialize with "alt" as alias for "meta"
        let json = r#"{"name": "e", "alt": true}"#;
        let info: KeybindInfo = serde_json::from_str(json).unwrap();

        assert_eq!(info.name, "e");
        assert!(info.meta);
    }

    #[test]
    fn test_default_keybinds() {
        let config = KeybindsConfig::default();

        assert_eq!(config.leader, "ctrl+x");
        assert_eq!(config.app_exit, "ctrl+c,ctrl+d,<leader>q");
        assert_eq!(config.editor_open, "<leader>e");
        assert_eq!(config.theme_list, "<leader>t");
        assert_eq!(config.sidebar_toggle, "<leader>b");
        assert_eq!(config.scrollbar_toggle, "none");
        assert_eq!(config.status_view, "<leader>s");

        assert_eq!(config.session_export, "<leader>x");
        assert_eq!(config.session_new, "<leader>n");
        assert_eq!(config.session_list, "ctrl+p");
        assert_eq!(config.session_timeline, "<leader>g");
        assert_eq!(config.session_fork, "none");
        assert_eq!(config.session_rename, "ctrl+r");
        assert_eq!(config.session_delete, "ctrl+d");
        assert_eq!(config.session_interrupt, "escape");
        assert_eq!(config.session_compact, "<leader>c");

        assert_eq!(config.model_provider_list, "ctrl+a");
        assert_eq!(config.model_favorite_toggle, "ctrl+f");
        assert_eq!(config.model_list, "<leader>m");
        assert_eq!(config.model_cycle_recent, "f2");
        assert_eq!(config.model_cycle_recent_reverse, "shift+f2");

        assert_eq!(config.messages_page_up, "pageup,ctrl+alt+b");
        assert_eq!(config.messages_page_down, "pagedown,ctrl+alt+f");
        assert_eq!(config.messages_line_up, "ctrl+alt+y");
        assert_eq!(config.messages_line_down, "ctrl+alt+e");
        assert_eq!(config.messages_first, "ctrl+g,home");
        assert_eq!(config.messages_last, "ctrl+alt+g,end");
        assert_eq!(config.messages_copy, "<leader>y");
        assert_eq!(config.messages_undo, "<leader>u");
        assert_eq!(config.messages_redo, "<leader>r");

        assert_eq!(config.input_submit, "return");
        assert_eq!(
            config.input_newline,
            "shift+return,ctrl+return,alt+return,ctrl+j"
        );
        assert_eq!(config.input_move_left, "left,ctrl+b");
        assert_eq!(config.input_move_right, "right,ctrl+f");
        assert_eq!(config.input_backspace, "backspace,shift+backspace");
        assert_eq!(config.input_delete, "ctrl+d,delete,shift+delete");
        assert_eq!(config.input_undo, "ctrl+-,super+z");
        assert_eq!(config.input_redo, "ctrl+.,super+shift+z");

        assert_eq!(config.command_list, "none");
        assert_eq!(config.agent_list, "<leader>a");
        assert_eq!(config.agent_cycle, "tab");
        assert_eq!(config.agent_cycle_reverse, "shift+tab");

        assert_eq!(config.terminal_suspend, "ctrl+z");
    }

    #[test]
    fn test_keybinds_serde() {
        let config = KeybindsConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"leader\":\"ctrl+x\""));
        assert!(json.contains("\"input_submit\":\"return\""));

        let deserialized: KeybindsConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.leader, config.leader);
        assert_eq!(deserialized.input_submit, config.input_submit);
    }

    #[test]
    fn test_keybinds_partial_deserialization() {
        let json = r#"{"leader": "ctrl+z", "input_submit": "enter"}"#;
        let config: KeybindsConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.leader, "ctrl+z");
        assert_eq!(config.input_submit, "enter");

        assert_eq!(config.app_exit, "ctrl+c,ctrl+d,<leader>q");
        assert_eq!(config.editor_open, "<leader>e");
    }

    #[test]
    fn test_keybinds_field_count() {
        let config = KeybindsConfig::default();
        let json = serde_json::to_value(&config).unwrap();
        let obj = json.as_object().unwrap();

        assert_eq!(
            obj.len(),
            96,
            "Expected 96 keybind fields, got {}",
            obj.len()
        );
    }

    #[test]
    fn test_from_crossterm_simple_char() {
        let evt = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "a");
        assert!(!info.ctrl);
    }

    #[test]
    fn test_from_crossterm_ctrl_char() {
        let evt = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "c");
        assert!(info.ctrl);
    }

    #[test]
    fn test_from_crossterm_enter() {
        let evt = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "return");
    }

    #[test]
    fn test_from_crossterm_escape() {
        let evt = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "escape");
    }

    #[test]
    fn test_from_crossterm_space() {
        let evt = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty());
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "space");
    }

    #[test]
    fn test_from_crossterm_multiple_modifiers() {
        let evt = KeyEvent::new(
            KeyCode::Char('f'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "f");
        assert!(info.ctrl);
        assert!(info.shift);
    }

    #[test]
    fn test_from_crossterm_function_key() {
        let evt = KeyEvent::new(KeyCode::F(2), KeyModifiers::empty());
        let info = KeybindInfo::from_crossterm(&evt);
        assert_eq!(info.name, "f2");
    }

    #[test]
    fn test_parse_single_key() {
        let result = KeybindInfo::parse("c");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "c");
        assert!(!result[0].ctrl);
    }

    #[test]
    fn test_parse_with_ctrl() {
        let result = KeybindInfo::parse("ctrl+c");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "c");
        assert!(result[0].ctrl);
    }

    #[test]
    fn test_parse_with_multiple_modifiers() {
        let result = KeybindInfo::parse("ctrl+shift+c");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "c");
        assert!(result[0].ctrl);
        assert!(result[0].shift);
    }

    #[test]
    fn test_parse_multiple_alternatives() {
        let result = KeybindInfo::parse("ctrl+c,ctrl+d");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "c");
        assert!(result[0].ctrl);
        assert_eq!(result[1].name, "d");
        assert!(result[1].ctrl);
    }

    #[test]
    fn test_parse_leader_key() {
        let result = KeybindInfo::parse("<leader>q");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "q");
        assert!(result[0].leader);
    }

    #[test]
    fn test_parse_none_keyword() {
        let result = KeybindInfo::parse("none");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_esc_to_escape() {
        let result = KeybindInfo::parse("esc");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "escape");
    }

    #[test]
    fn test_parse_case_insensitive() {
        let result = KeybindInfo::parse("CTRL+C");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "c");
        assert!(result[0].ctrl);
    }

    #[test]
    fn test_parse_leader_with_modifiers() {
        let result = KeybindInfo::parse("<leader>ctrl+w");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "w");
        assert!(result[0].leader);
        assert!(result[0].ctrl);
    }

    #[test]
    fn test_leader_state_default() {
        let state = LeaderState::default();
        assert!(!state.is_active());
        assert_eq!(state.leader_key().name, "x");
        assert!(state.leader_key().ctrl);
    }

    #[test]
    fn test_leader_state_activation() {
        let mut state = LeaderState::new("ctrl+x");
        assert!(!state.is_active());

        assert!(state.activate());
        assert!(state.is_active());

        assert!(!state.activate());
    }

    #[test]
    fn test_leader_state_deactivation() {
        let mut state = LeaderState::new("ctrl+x");
        state.activate();
        assert!(state.is_active());

        state.deactivate();
        assert!(!state.is_active());
    }

    #[test]
    fn test_leader_state_is_leader_key() {
        let state = LeaderState::new("ctrl+x");

        let matching = KeybindInfo::new("x").with_ctrl(true);
        assert!(state.is_leader_key(&matching));

        let non_matching = KeybindInfo::new("y").with_ctrl(true);
        assert!(!state.is_leader_key(&non_matching));
    }

    #[test]
    fn test_leader_state_check_and_handle() {
        let mut state = LeaderState::new("ctrl+x");

        let leader_evt = KeybindInfo::new("x").with_ctrl(true);
        assert!(state.check_and_handle(&leader_evt));
        assert!(state.is_active());

        let other_evt = KeybindInfo::new("a");
        assert!(!state.check_and_handle(&other_evt));
    }

    #[test]
    fn test_leader_state_timeout() {
        let mut state = LeaderState::new("ctrl+x").with_timeout(Duration::from_millis(50));

        state.activate();
        assert!(state.is_active());

        std::thread::sleep(Duration::from_millis(60));
        assert!(state.update());
        assert!(!state.is_active());
    }

    #[test]
    fn test_leader_state_remaining() {
        let mut state = LeaderState::new("ctrl+x").with_timeout(Duration::from_secs(10));

        assert!(state.remaining().is_none());

        state.activate();
        let remaining = state.remaining();
        assert!(remaining.is_some());
        assert!(remaining.unwrap() > Duration::from_secs(5));
    }

    // KeybindInfo::matches() tests

    #[test]
    fn test_keybind_matches_basic() {
        let a = KeybindInfo::new("c").with_ctrl(true);
        let b = KeybindInfo::new("c").with_ctrl(true);
        assert!(a.matches(&b));
    }

    #[test]
    fn test_keybind_matches_different_key() {
        let a = KeybindInfo::new("c").with_ctrl(true);
        let b = KeybindInfo::new("d").with_ctrl(true);
        assert!(!a.matches(&b));
    }

    #[test]
    fn test_keybind_matches_different_modifier() {
        let a = KeybindInfo::new("c").with_ctrl(true);
        let b = KeybindInfo::new("c").with_meta(true);
        assert!(!a.matches(&b));
    }

    #[test]
    fn test_keybind_matches_multiple_modifiers() {
        let a = KeybindInfo::new("c").with_ctrl(true).with_shift(true);
        let b = KeybindInfo::new("c").with_ctrl(true).with_shift(true);
        assert!(a.matches(&b));
    }

    #[test]
    fn test_keybind_matches_leader_flag() {
        let a = KeybindInfo::new("q").with_leader(true);
        let b = KeybindInfo::new("q").with_leader(true);
        assert!(a.matches(&b));

        let c = KeybindInfo::new("q");
        assert!(!a.matches(&c));
    }

    #[test]
    fn test_keybind_matches_all_modifiers() {
        let a = KeybindInfo::new("x")
            .with_ctrl(true)
            .with_meta(true)
            .with_shift(true)
            .with_super(true);
        let b = KeybindInfo::new("x")
            .with_ctrl(true)
            .with_meta(true)
            .with_shift(true)
            .with_super(true);
        assert!(a.matches(&b));
    }

    // KeybindsConfig::matches() tests

    #[test]
    fn test_config_matches_simple_keybind() {
        let config = KeybindsConfig::default();
        let ctrl_c = KeybindInfo::new("c").with_ctrl(true);
        assert!(config.matches("app_exit", &ctrl_c, false));

        let ctrl_d = KeybindInfo::new("d").with_ctrl(true);
        assert!(config.matches("app_exit", &ctrl_d, false));
    }

    #[test]
    fn test_config_matches_leader_keybind() {
        let config = KeybindsConfig::default();
        let leader_q = KeybindInfo::new("q").with_leader(true);

        assert!(!config.matches("app_exit", &leader_q, false));

        let evt_q = KeybindInfo::new("q");
        assert!(config.matches("app_exit", &evt_q, true));
    }

    #[test]
    fn test_config_matches_no_match() {
        let config = KeybindsConfig::default();

        let random_key = KeybindInfo::new("z").with_ctrl(true);
        assert!(!config.matches("app_exit", &random_key, false));
    }

    #[test]
    fn test_config_matches_unknown_action() {
        let config = KeybindsConfig::default();

        let evt = KeybindInfo::new("c").with_ctrl(true);
        assert!(!config.matches("nonexistent_action", &evt, false));
    }

    #[test]
    fn test_config_matches_editor_open() {
        let config = KeybindsConfig::default();
        let evt_e = KeybindInfo::new("e");

        assert!(!config.matches("editor_open", &evt_e, false));
        assert!(config.matches("editor_open", &evt_e, true));
    }

    #[test]
    fn test_config_matches_leader_action() {
        let config = KeybindsConfig::default();

        let ctrl_x = KeybindInfo::new("x").with_ctrl(true);
        assert!(config.matches("leader", &ctrl_x, false));

        let ctrl_y = KeybindInfo::new("y").with_ctrl(true);
        assert!(!config.matches("leader", &ctrl_y, false));
    }

    #[test]
    fn test_config_matches_input_navigation() {
        let config = KeybindsConfig::default();

        let left = KeybindInfo::new("left");
        assert!(config.matches("input_move_left", &left, false));

        let right = KeybindInfo::new("right");
        assert!(config.matches("input_move_right", &right, false));
    }
}
