//! Help Dialog Component
//!
//! A modal dialog that displays all keyboard bindings organized by category.
//! Features include search/filter functionality, scrollable list, and scrollbar.
//!
//! # Features
//!
//! - All ~95 keyboard bindings grouped by 8 categories
//! - Search/filter functionality (activated with `/`)
//! - Scrollable list with scrollbar
//! - Case-insensitive substring matching
//! - Keyboard navigation (Up/Down, PgUp/PgDn, Home/End, j/k, g/G)
//!
//! # Categories
//!
//! 1. Application - General app controls
//! 2. Session Management - Session operations
//! 3. Model Selection - Model switching
//! 4. Message Navigation - Message list navigation
//! 5. Command & Agent - Command palette and agent controls
//! 6. Input Controls - Text input operations (with subcategories)
//! 7. Session Tree - Tree navigation
//! 8. Display - Display toggles

use super::{Component, ComponentId, EventPropagation};
use crate::cli::commands::tui::keybindings::KeybindsConfig;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};
use std::time::Duration;
use tui_input::{Input, InputRequest};

/// Help category for organizing keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HelpCategory {
    Application,
    SessionManagement,
    ModelSelection,
    MessageNavigation,
    CommandAgent,
    InputControls,
    SessionTree,
    Display,
}

impl HelpCategory {
    /// Get the display name for this category
    pub fn display_name(&self) -> &'static str {
        match self {
            HelpCategory::Application => "Application",
            HelpCategory::SessionManagement => "Session Management",
            HelpCategory::ModelSelection => "Model Selection",
            HelpCategory::MessageNavigation => "Message Navigation",
            HelpCategory::CommandAgent => "Command & Agent",
            HelpCategory::InputControls => "Input Controls",
            HelpCategory::SessionTree => "Session Tree",
            HelpCategory::Display => "Display",
        }
    }

    /// Get all categories in display order
    pub fn all() -> &'static [HelpCategory] {
        &[
            HelpCategory::Application,
            HelpCategory::SessionManagement,
            HelpCategory::ModelSelection,
            HelpCategory::MessageNavigation,
            HelpCategory::CommandAgent,
            HelpCategory::InputControls,
            HelpCategory::SessionTree,
            HelpCategory::Display,
        ]
    }
}

impl std::fmt::Display for HelpCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// A single help item representing a keybinding
#[derive(Debug, Clone)]
pub struct HelpItem {
    /// The key combination display string (formatted)
    pub key_display: String,
    /// The raw key string from config
    pub key_raw: String,
    /// Human-readable description of the action
    pub description: String,
    /// Category this item belongs to
    pub category: HelpCategory,
    /// Optional subcategory (used for Input Controls)
    pub subcategory: Option<&'static str>,
}

impl HelpItem {
    /// Create a new help item
    pub fn new(
        key_raw: impl Into<String>,
        description: impl Into<String>,
        category: HelpCategory,
    ) -> Self {
        let key_raw = key_raw.into();
        let key_display = format_key_display(&key_raw);
        Self {
            key_display,
            key_raw,
            description: description.into(),
            category,
            subcategory: None,
        }
    }

    /// Builder: Add a subcategory
    pub fn with_subcategory(mut self, subcategory: &'static str) -> Self {
        self.subcategory = Some(subcategory);
        self
    }
}

/// Format a raw keybinding string for display
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::help_dialog::format_key_display;
///
/// assert_eq!(format_key_display("ctrl+c"), "Ctrl+C");
/// assert_eq!(format_key_display("<leader>q"), "<Leader> Q");
/// assert_eq!(format_key_display("return"), "Enter");
/// assert_eq!(format_key_display("none"), "—");
/// assert_eq!(format_key_display("pageup,ctrl+alt+b"), "PgUp or Ctrl+Alt+B");
/// ```
pub fn format_key_display(key: &str) -> String {
    if key.eq_ignore_ascii_case("none") {
        return "—".to_string();
    }

    key.split(',')
        .map(|combo| {
            let combo = combo.trim();
            // Handle <leader> syntax
            let has_leader = combo.to_lowercase().starts_with("<leader>");
            let normalized = combo.replace("<leader>", "").replace("<Leader>", "");
            let normalized = normalized.trim_start_matches('+').trim();

            // Split by + to process each part
            let parts: Vec<&str> = normalized.split('+').collect();
            let formatted_parts: Vec<String> = parts
                .iter()
                .map(|p| {
                    let p_lower = p.to_lowercase();
                    match p_lower.as_str() {
                        "ctrl" => "Ctrl".to_string(),
                        "alt" | "meta" => "Alt".to_string(),
                        "shift" => "Shift".to_string(),
                        "super" => "Super".to_string(),
                        "return" | "enter" => "Enter".to_string(),
                        "escape" | "esc" => "Esc".to_string(),
                        "backspace" => "Bksp".to_string(),
                        "delete" | "del" => "Del".to_string(),
                        "pageup" | "pgup" => "PgUp".to_string(),
                        "pagedown" | "pgdn" => "PgDn".to_string(),
                        "home" => "Home".to_string(),
                        "end" => "End".to_string(),
                        "up" => "↑".to_string(),
                        "down" => "↓".to_string(),
                        "left" => "←".to_string(),
                        "right" => "→".to_string(),
                        "space" => "Space".to_string(),
                        "tab" => "Tab".to_string(),
                        // Single character keys - uppercase
                        c if c.len() == 1 => c.to_uppercase(),
                        // Function keys and other named keys
                        _ => {
                            // Capitalize first letter
                            let mut chars = p.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        }
                    }
                })
                .collect();

            let result = formatted_parts.join("+");
            if has_leader {
                format!("<Leader> {}", result)
            } else {
                result
            }
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

/// Build all help items from the keybinds configuration
#[allow(clippy::vec_init_then_push)]
fn build_items(config: &KeybindsConfig) -> Vec<HelpItem> {
    let mut items = Vec::new();

    // === Application ===
    items.push(HelpItem::new(
        &config.leader,
        "Leader key prefix",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.app_exit,
        "Exit application",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.editor_open,
        "Open in editor",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.theme_list,
        "Theme selector",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.sidebar_toggle,
        "Toggle sidebar",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.scrollbar_toggle,
        "Toggle scrollbar",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.username_toggle,
        "Toggle username",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.status_view,
        "Status view",
        HelpCategory::Application,
    ));

    // === Session Management ===
    items.push(HelpItem::new(
        &config.session_export,
        "Export session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_new,
        "New session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_list,
        "Session list",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_timeline,
        "Session timeline",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_fork,
        "Fork session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_rename,
        "Rename session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_delete,
        "Delete session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.stash_delete,
        "Delete stash",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_share,
        "Share session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_unshare,
        "Unshare session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_interrupt,
        "Interrupt session",
        HelpCategory::SessionManagement,
    ));
    items.push(HelpItem::new(
        &config.session_compact,
        "Compact session",
        HelpCategory::SessionManagement,
    ));

    // === Model Selection ===
    items.push(HelpItem::new(
        &config.model_provider_list,
        "Provider list",
        HelpCategory::ModelSelection,
    ));
    items.push(HelpItem::new(
        &config.model_favorite_toggle,
        "Toggle favorite",
        HelpCategory::ModelSelection,
    ));
    items.push(HelpItem::new(
        &config.model_list,
        "Model selector",
        HelpCategory::ModelSelection,
    ));
    items.push(HelpItem::new(
        &config.model_cycle_recent,
        "Cycle recent models",
        HelpCategory::ModelSelection,
    ));
    items.push(HelpItem::new(
        &config.model_cycle_recent_reverse,
        "Cycle recent (reverse)",
        HelpCategory::ModelSelection,
    ));
    items.push(HelpItem::new(
        &config.model_cycle_favorite,
        "Cycle favorites",
        HelpCategory::ModelSelection,
    ));
    items.push(HelpItem::new(
        &config.model_cycle_favorite_reverse,
        "Cycle favorites (reverse)",
        HelpCategory::ModelSelection,
    ));

    // === Message Navigation ===
    items.push(HelpItem::new(
        &config.messages_page_up,
        "Page up",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_page_down,
        "Page down",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_line_up,
        "Line up",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_line_down,
        "Line down",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_half_page_up,
        "Half page up",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_half_page_down,
        "Half page down",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_first,
        "First message",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_last,
        "Last message",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_next,
        "Next message",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_previous,
        "Previous message",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_last_user,
        "Last user message",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_copy,
        "Copy message",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_undo,
        "Undo",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_redo,
        "Redo",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.messages_toggle_conceal,
        "Toggle conceal",
        HelpCategory::MessageNavigation,
    ));
    items.push(HelpItem::new(
        &config.tool_details,
        "Tool details",
        HelpCategory::MessageNavigation,
    ));

    // === Command & Agent ===
    items.push(HelpItem::new(
        &config.command_list,
        "Command palette",
        HelpCategory::CommandAgent,
    ));
    items.push(HelpItem::new(
        &config.agent_list,
        "Agent selector",
        HelpCategory::CommandAgent,
    ));
    items.push(HelpItem::new(
        &config.agent_cycle,
        "Cycle agents",
        HelpCategory::CommandAgent,
    ));
    items.push(HelpItem::new(
        &config.agent_cycle_reverse,
        "Cycle agents (reverse)",
        HelpCategory::CommandAgent,
    ));
    items.push(HelpItem::new(
        &config.variant_cycle,
        "Cycle variants",
        HelpCategory::CommandAgent,
    ));

    // === Input Controls - Basic ===
    items.push(
        HelpItem::new(
            &config.input_clear,
            "Clear input",
            HelpCategory::InputControls,
        )
        .with_subcategory("Basic"),
    );
    items.push(
        HelpItem::new(&config.input_paste, "Paste", HelpCategory::InputControls)
            .with_subcategory("Basic"),
    );
    items.push(
        HelpItem::new(&config.input_submit, "Submit", HelpCategory::InputControls)
            .with_subcategory("Basic"),
    );
    items.push(
        HelpItem::new(
            &config.input_newline,
            "New line",
            HelpCategory::InputControls,
        )
        .with_subcategory("Basic"),
    );

    // === Input Controls - Movement ===
    items.push(
        HelpItem::new(
            &config.input_move_left,
            "Move left",
            HelpCategory::InputControls,
        )
        .with_subcategory("Movement"),
    );
    items.push(
        HelpItem::new(
            &config.input_move_right,
            "Move right",
            HelpCategory::InputControls,
        )
        .with_subcategory("Movement"),
    );
    items.push(
        HelpItem::new(
            &config.input_move_up,
            "Move up",
            HelpCategory::InputControls,
        )
        .with_subcategory("Movement"),
    );
    items.push(
        HelpItem::new(
            &config.input_move_down,
            "Move down",
            HelpCategory::InputControls,
        )
        .with_subcategory("Movement"),
    );

    // === Input Controls - Selection ===
    items.push(
        HelpItem::new(
            &config.input_select_left,
            "Select left",
            HelpCategory::InputControls,
        )
        .with_subcategory("Selection"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_right,
            "Select right",
            HelpCategory::InputControls,
        )
        .with_subcategory("Selection"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_up,
            "Select up",
            HelpCategory::InputControls,
        )
        .with_subcategory("Selection"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_down,
            "Select down",
            HelpCategory::InputControls,
        )
        .with_subcategory("Selection"),
    );

    // === Input Controls - Line Navigation ===
    items.push(
        HelpItem::new(
            &config.input_line_home,
            "Line start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_line_end,
            "Line end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_line_home,
            "Select to line start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_line_end,
            "Select to line end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_visual_line_home,
            "Visual line start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_visual_line_end,
            "Visual line end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_visual_line_home,
            "Select visual line start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_visual_line_end,
            "Select visual line end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Line Navigation"),
    );

    // === Input Controls - Buffer Navigation ===
    items.push(
        HelpItem::new(
            &config.input_buffer_home,
            "Buffer start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Buffer Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_buffer_end,
            "Buffer end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Buffer Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_buffer_home,
            "Select to buffer start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Buffer Navigation"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_buffer_end,
            "Select to buffer end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Buffer Navigation"),
    );

    // === Input Controls - Delete ===
    items.push(
        HelpItem::new(
            &config.input_delete_line,
            "Delete line",
            HelpCategory::InputControls,
        )
        .with_subcategory("Delete"),
    );
    items.push(
        HelpItem::new(
            &config.input_delete_to_line_end,
            "Delete to line end",
            HelpCategory::InputControls,
        )
        .with_subcategory("Delete"),
    );
    items.push(
        HelpItem::new(
            &config.input_delete_to_line_start,
            "Delete to line start",
            HelpCategory::InputControls,
        )
        .with_subcategory("Delete"),
    );
    items.push(
        HelpItem::new(
            &config.input_backspace,
            "Backspace",
            HelpCategory::InputControls,
        )
        .with_subcategory("Delete"),
    );
    items.push(
        HelpItem::new(&config.input_delete, "Delete", HelpCategory::InputControls)
            .with_subcategory("Delete"),
    );

    // === Input Controls - Undo/Redo ===
    items.push(
        HelpItem::new(&config.input_undo, "Undo", HelpCategory::InputControls)
            .with_subcategory("Undo/Redo"),
    );
    items.push(
        HelpItem::new(&config.input_redo, "Redo", HelpCategory::InputControls)
            .with_subcategory("Undo/Redo"),
    );

    // === Input Controls - Word ===
    items.push(
        HelpItem::new(
            &config.input_word_forward,
            "Word forward",
            HelpCategory::InputControls,
        )
        .with_subcategory("Word"),
    );
    items.push(
        HelpItem::new(
            &config.input_word_backward,
            "Word backward",
            HelpCategory::InputControls,
        )
        .with_subcategory("Word"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_word_forward,
            "Select word forward",
            HelpCategory::InputControls,
        )
        .with_subcategory("Word"),
    );
    items.push(
        HelpItem::new(
            &config.input_select_word_backward,
            "Select word backward",
            HelpCategory::InputControls,
        )
        .with_subcategory("Word"),
    );
    items.push(
        HelpItem::new(
            &config.input_delete_word_forward,
            "Delete word forward",
            HelpCategory::InputControls,
        )
        .with_subcategory("Word"),
    );
    items.push(
        HelpItem::new(
            &config.input_delete_word_backward,
            "Delete word backward",
            HelpCategory::InputControls,
        )
        .with_subcategory("Word"),
    );

    // === Input Controls - History ===
    items.push(
        HelpItem::new(
            &config.history_previous,
            "Previous history",
            HelpCategory::InputControls,
        )
        .with_subcategory("History"),
    );
    items.push(
        HelpItem::new(
            &config.history_next,
            "Next history",
            HelpCategory::InputControls,
        )
        .with_subcategory("History"),
    );

    // === Session Tree ===
    items.push(HelpItem::new(
        &config.session_child_first,
        "First child",
        HelpCategory::SessionTree,
    ));
    items.push(HelpItem::new(
        &config.session_child_cycle,
        "Cycle children",
        HelpCategory::SessionTree,
    ));
    items.push(HelpItem::new(
        &config.session_child_cycle_reverse,
        "Cycle children (reverse)",
        HelpCategory::SessionTree,
    ));
    items.push(HelpItem::new(
        &config.session_parent,
        "Parent session",
        HelpCategory::SessionTree,
    ));

    // === Terminal (grouped with Application for display) ===
    items.push(HelpItem::new(
        &config.terminal_suspend,
        "Suspend terminal",
        HelpCategory::Application,
    ));
    items.push(HelpItem::new(
        &config.terminal_title_toggle,
        "Toggle terminal title",
        HelpCategory::Application,
    ));

    // === Display ===
    items.push(HelpItem::new(
        &config.tips_toggle,
        "Toggle tips",
        HelpCategory::Display,
    ));
    items.push(HelpItem::new(
        &config.display_thinking,
        "Toggle thinking display",
        HelpCategory::Display,
    ));

    items
}

/// Help dialog state for tracking what to display
#[derive(Debug, Clone, Default)]
struct DisplayLine {
    /// Line type
    line_type: DisplayLineType,
    /// Original item index (for items only)
    item_index: Option<usize>,
}

/// Type of display line in the help dialog
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum DisplayLineType {
    #[default]
    Item,
    CategoryHeader,
    SubcategoryHeader,
    BlankLine,
}

/// Modal dialog displaying keyboard bindings help
///
/// # Features
///
/// - Categorized keyboard bindings
/// - Search/filter functionality
/// - Scrollable list with scrollbar
/// - Modal overlay behavior
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::help_dialog::HelpDialog;
/// use crate::cli::commands::tui::keybindings::KeybindsConfig;
///
/// let config = KeybindsConfig::default();
/// let mut dialog = HelpDialog::new(config);
///
/// dialog.show();
/// assert!(dialog.is_visible());
///
/// dialog.hide();
/// assert!(!dialog.is_visible());
/// ```
pub struct HelpDialog {
    /// Unique component identifier
    id: ComponentId,
    /// All help items (unfiltered)
    all_items: Vec<HelpItem>,
    /// Filtered display lines
    display_lines: Vec<DisplayLine>,
    /// Search input state
    search_input: Input,
    /// Whether search mode is active
    search_active: bool,
    /// Scroll offset
    scroll_offset: usize,
    /// Content height for scrolling
    content_height: usize,
    /// Whether the dialog is currently visible
    visible: bool,
    /// Whether the dialog currently has focus
    focused: bool,
}

impl HelpDialog {
    /// Create a new HelpDialog with the given keybinds configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The keybinds configuration to display
    ///
    /// # Returns
    ///
    /// A new HelpDialog, hidden by default.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::cli::commands::tui::components::help_dialog::HelpDialog;
    /// use crate::cli::commands::tui::keybindings::KeybindsConfig;
    ///
    /// let config = KeybindsConfig::default();
    /// let dialog = HelpDialog::new(config);
    /// assert!(!dialog.is_visible());
    /// ```
    pub fn new(config: KeybindsConfig) -> Self {
        let all_items = build_items(&config);
        let mut dialog = Self {
            id: ComponentId::new(),
            all_items,
            display_lines: Vec::new(),
            search_input: Input::default(),
            search_active: false,
            scroll_offset: 0,
            content_height: 0,
            visible: false,
            focused: false,
        };
        dialog.rebuild_display_lines();
        dialog
    }

    /// Check if the dialog is currently visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
        self.scroll_offset = 0;
        self.search_active = false;
        self.search_input.reset();
        self.rebuild_display_lines();
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
        self.search_active = false;
    }

    /// Check if the dialog is focused
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Check if search mode is active
    pub fn is_search_active(&self) -> bool {
        self.search_active
    }

    /// Get the current search query
    pub fn search_query(&self) -> &str {
        self.search_input.value()
    }

    /// Get the number of items (before filtering)
    pub fn item_count(&self) -> usize {
        self.all_items.len()
    }

    /// Calculate the centered area for the dialog
    ///
    /// Dialog size: 85% width, 90% height, capped at 120×40
    fn centered_area(&self, frame_area: Rect) -> Rect {
        // Calculate percentages
        let width_percent = 85;
        let height_percent = 90;

        // Calculate dimensions with caps
        let max_width = 120u16;
        let max_height = 40u16;

        let width = ((frame_area.width as u32 * width_percent as u32 / 100) as u16).min(max_width);
        let height =
            ((frame_area.height as u32 * height_percent as u32 / 100) as u16).min(max_height);

        // Center the dialog
        let x = frame_area.x + (frame_area.width.saturating_sub(width)) / 2;
        let y = frame_area.y + (frame_area.height.saturating_sub(height)) / 2;

        Rect::new(x, y, width, height)
    }

    /// Rebuild display lines based on current filter
    fn rebuild_display_lines(&mut self) {
        self.display_lines.clear();
        let query = self.search_input.value().to_lowercase();

        // Group items by category
        let mut current_category: Option<HelpCategory> = None;
        let mut current_subcategory: Option<&str> = None;

        // If searching, flatten the display without category headers
        let filtered_items: Vec<(usize, &HelpItem)> = if query.is_empty() {
            self.all_items.iter().enumerate().collect()
        } else {
            self.all_items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    // Case-insensitive substring match on key, description, and category
                    let key_match = item.key_display.to_lowercase().contains(&query);
                    let desc_match = item.description.to_lowercase().contains(&query);
                    let cat_match = item.category.display_name().to_lowercase().contains(&query);
                    key_match || desc_match || cat_match
                })
                .collect()
        };

        for (idx, item) in filtered_items {
            // Add category header if new category (only when not searching)
            if query.is_empty() && current_category != Some(item.category) {
                // Add blank line between categories (except first)
                if current_category.is_some() {
                    self.display_lines.push(DisplayLine {
                        line_type: DisplayLineType::BlankLine,
                        item_index: None,
                    });
                }
                current_category = Some(item.category);
                current_subcategory = None;
                self.display_lines.push(DisplayLine {
                    line_type: DisplayLineType::CategoryHeader,
                    item_index: None,
                });
            }

            // Add subcategory header if new subcategory
            if item.subcategory != current_subcategory {
                current_subcategory = item.subcategory;
                if item.subcategory.is_some() {
                    self.display_lines.push(DisplayLine {
                        line_type: DisplayLineType::SubcategoryHeader,
                        item_index: None,
                    });
                }
            }

            // Add the item
            self.display_lines.push(DisplayLine {
                line_type: DisplayLineType::Item,
                item_index: Some(idx),
            });
        }

        self.content_height = self.display_lines.len();
        // Clamp scroll offset
        self.clamp_scroll();
    }

    /// Clamp scroll offset to valid range
    fn clamp_scroll(&mut self) {
        let max_scroll = self.content_height.saturating_sub(1);
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }

    /// Scroll up by one line
    fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down by one line
    fn scroll_down(&mut self) {
        let max_scroll = self.content_height.saturating_sub(1);
        if self.scroll_offset < max_scroll {
            self.scroll_offset += 1;
        }
    }

    /// Scroll up by one page
    fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    /// Scroll down by one page
    fn scroll_page_down(&mut self, page_size: usize) {
        let max_scroll = self.content_height.saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + page_size).min(max_scroll);
    }

    /// Scroll to top
    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to bottom
    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.content_height.saturating_sub(1);
    }

    /// Handle search input character
    fn handle_search_char(&mut self, c: char) {
        self.search_input.handle(InputRequest::InsertChar(c));
        self.rebuild_display_lines();
    }

    /// Handle search backspace
    fn handle_search_backspace(&mut self) {
        self.search_input.handle(InputRequest::DeletePrevChar);
        self.rebuild_display_lines();
    }

    /// Enter search mode
    fn enter_search_mode(&mut self) {
        self.search_active = true;
    }

    /// Exit search mode (keep results)
    fn exit_search_mode(&mut self) {
        self.search_active = false;
    }

    /// Cancel search mode (reset filter)
    fn cancel_search(&mut self) {
        self.search_active = false;
        self.search_input.reset();
        self.rebuild_display_lines();
    }
}

impl Component for HelpDialog {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = self.centered_area(area);

        // Clear the area for modal overlay effect
        frame.render_widget(Clear, dialog_area);

        // Dialog block with border
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };

        let title = if self.search_active {
            format!(" Help - Search: {}_", self.search_input.value())
        } else if !self.search_input.value().is_empty() {
            format!(" Help - Filter: {}", self.search_input.value())
        } else {
            " Help - Press / to search, Esc to close ".to_string()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner_area = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Calculate layout: content area + footer
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner_area);

        let content_area = layout[0];
        let footer_area = layout[1];

        // Render content
        let visible_height = content_area.height as usize;
        let mut lines_to_render = Vec::new();

        // Calculate which lines to show
        let end_idx = (self.scroll_offset + visible_height).min(self.display_lines.len());

        for line_idx in self.scroll_offset..end_idx {
            let display_line = &self.display_lines[line_idx];

            let line = match display_line.line_type {
                DisplayLineType::CategoryHeader => {
                    if let Some(item_idx) = self
                        .display_lines
                        .get(line_idx + 1)
                        .and_then(|l| l.item_index)
                    {
                        if let Some(item) = self.all_items.get(item_idx) {
                            Line::from(vec![
                                Span::styled("━━ ", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    item.category.display_name(),
                                    Style::default()
                                        .fg(Color::Yellow)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    " ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━",
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ])
                        } else {
                            Line::default()
                        }
                    } else {
                        Line::default()
                    }
                }
                DisplayLineType::SubcategoryHeader => {
                    if let Some(item_idx) = self
                        .display_lines
                        .get(line_idx + 1)
                        .and_then(|l| l.item_index)
                    {
                        if let Some(item) = self.all_items.get(item_idx) {
                            Line::from(vec![
                                Span::styled("  ▸ ", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    item.subcategory.unwrap_or(""),
                                    Style::default()
                                        .fg(Color::LightCyan)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ])
                        } else {
                            Line::default()
                        }
                    } else {
                        Line::default()
                    }
                }
                DisplayLineType::Item => {
                    if let Some(item_idx) = display_line.item_index {
                        if let Some(item) = self.all_items.get(item_idx) {
                            // Format: "  Key          Description"
                            let key_padded = format!("{:20}", item.key_display);
                            Line::from(vec![
                                Span::styled("  ", Style::default()),
                                Span::styled(
                                    key_padded,
                                    Style::default()
                                        .fg(Color::Green)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(&item.description, Style::default().fg(Color::White)),
                            ])
                        } else {
                            Line::default()
                        }
                    } else {
                        Line::default()
                    }
                }
                DisplayLineType::BlankLine => Line::default(),
            };

            lines_to_render.push(line);
        }

        // Pad with empty lines if needed
        while lines_to_render.len() < visible_height {
            lines_to_render.push(Line::default());
        }

        let content = Paragraph::new(lines_to_render);
        frame.render_widget(content, content_area);

        // Render scrollbar
        if self.content_height > visible_height {
            let scrollbar_area = Rect::new(
                content_area.x + content_area.width.saturating_sub(1),
                content_area.y,
                1,
                content_area.height,
            );

            let scrollbar_state = ScrollbarState::new(self.content_height)
                .position(self.scroll_offset)
                .viewport_content_length(visible_height);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");

            frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state.clone());
        }

        // Render footer with navigation hints
        let footer_text = if self.search_active {
            " Type to search | Esc to cancel | Enter to apply"
        } else {
            " ↑/↓ or j/k: scroll | PgUp/PgDn: page | Home/End: jump | /: search | Esc: close "
        };

        let footer = Paragraph::new(Line::from(Span::styled(
            footer_text,
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(footer, footer_area);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        if !self.visible {
            return EventPropagation::Continue;
        }

        // If in search mode, handle search input
        if self.search_active {
            match event.code {
                KeyCode::Esc => {
                    self.cancel_search();
                }
                KeyCode::Enter => {
                    self.exit_search_mode();
                }
                KeyCode::Char(c) => {
                    self.handle_search_char(c);
                }
                KeyCode::Backspace => {
                    self.handle_search_backspace();
                }
                _ => {}
            }
            return EventPropagation::Stop;
        }

        // Normal mode key handling
        match event.code {
            KeyCode::Esc => {
                self.hide();
            }
            KeyCode::Char('/') => {
                self.enter_search_mode();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_down();
            }
            KeyCode::PageUp => {
                // Use a reasonable page size
                self.scroll_page_up(10);
            }
            KeyCode::PageDown => {
                self.scroll_page_down(10);
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll_to_top();
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll_to_bottom();
            }
            _ => {}
        }

        EventPropagation::Stop
    }

    fn update(&mut self, _delta: Duration) {
        // No periodic updates needed
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn on_focus(&mut self) {
        self.focused = true;
    }

    fn on_blur(&mut self) {
        self.focused = false;
    }
}

impl Default for HelpDialog {
    fn default() -> Self {
        Self::new(KeybindsConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_key_display_simple() {
        assert_eq!(format_key_display("c"), "C");
        assert_eq!(format_key_display("return"), "Enter");
        assert_eq!(format_key_display("escape"), "Esc");
    }

    #[test]
    fn test_format_key_display_ctrl() {
        assert_eq!(format_key_display("ctrl+c"), "Ctrl+C");
        assert_eq!(format_key_display("ctrl+shift+s"), "Ctrl+Shift+S");
    }

    #[test]
    fn test_format_key_display_leader() {
        assert_eq!(format_key_display("<leader>q"), "<Leader> Q");
        assert_eq!(format_key_display("<leader>ctrl+w"), "<Leader> Ctrl+W");
    }

    #[test]
    fn test_format_key_display_none() {
        assert_eq!(format_key_display("none"), "—");
        assert_eq!(format_key_display("NONE"), "—");
    }

    #[test]
    fn test_format_key_display_alternatives() {
        assert_eq!(format_key_display("ctrl+c,ctrl+d"), "Ctrl+C or Ctrl+D");
        assert_eq!(
            format_key_display("pageup,ctrl+alt+b"),
            "PgUp or Ctrl+Alt+B"
        );
    }

    #[test]
    fn test_format_key_display_arrows() {
        assert_eq!(format_key_display("up"), "↑");
        assert_eq!(format_key_display("down"), "↓");
        assert_eq!(format_key_display("left"), "←");
        assert_eq!(format_key_display("right"), "→");
    }

    #[test]
    fn test_format_key_display_special_keys() {
        assert_eq!(format_key_display("backspace"), "Bksp");
        assert_eq!(format_key_display("delete"), "Del");
        assert_eq!(format_key_display("pageup"), "PgUp");
        assert_eq!(format_key_display("pagedown"), "PgDn");
        assert_eq!(format_key_display("home"), "Home");
        assert_eq!(format_key_display("end"), "End");
        assert_eq!(format_key_display("space"), "Space");
        assert_eq!(format_key_display("tab"), "Tab");
    }

    #[test]
    fn test_help_category_display_names() {
        assert_eq!(HelpCategory::Application.display_name(), "Application");
        assert_eq!(
            HelpCategory::SessionManagement.display_name(),
            "Session Management"
        );
        assert_eq!(
            HelpCategory::ModelSelection.display_name(),
            "Model Selection"
        );
        assert_eq!(
            HelpCategory::MessageNavigation.display_name(),
            "Message Navigation"
        );
        assert_eq!(HelpCategory::CommandAgent.display_name(), "Command & Agent");
        assert_eq!(HelpCategory::InputControls.display_name(), "Input Controls");
        assert_eq!(HelpCategory::SessionTree.display_name(), "Session Tree");
        assert_eq!(HelpCategory::Display.display_name(), "Display");
    }

    #[test]
    fn test_help_category_all() {
        let all = HelpCategory::all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0], HelpCategory::Application);
        assert_eq!(all[7], HelpCategory::Display);
    }

    #[test]
    fn test_help_dialog_new() {
        let dialog = HelpDialog::new(KeybindsConfig::default());
        assert!(!dialog.is_visible());
        assert!(!dialog.is_search_active());
        assert!(dialog.search_query().is_empty());
    }

    #[test]
    fn test_help_dialog_show_hide() {
        let mut dialog = HelpDialog::new(KeybindsConfig::default());

        dialog.show();
        assert!(dialog.is_visible());
        assert!(dialog.is_focused());

        dialog.hide();
        assert!(!dialog.is_visible());
        assert!(!dialog.is_focused());
    }

    #[test]
    fn test_help_dialog_item_count() {
        let dialog = HelpDialog::new(KeybindsConfig::default());
        // Should have 95 items
        assert_eq!(dialog.item_count(), 95);
    }

    #[test]
    fn test_help_dialog_search_mode() {
        let mut dialog = HelpDialog::new(KeybindsConfig::default());
        dialog.show();

        assert!(!dialog.is_search_active());

        // Simulate entering search mode
        dialog.enter_search_mode();
        assert!(dialog.is_search_active());

        // Simulate typing
        dialog.handle_search_char('c');
        assert_eq!(dialog.search_query(), "c");

        // Cancel search
        dialog.cancel_search();
        assert!(!dialog.is_search_active());
        assert!(dialog.search_query().is_empty());
    }

    #[test]
    fn test_help_dialog_filter() {
        let mut dialog = HelpDialog::new(KeybindsConfig::default());
        dialog.show();

        // Filter should work
        dialog.handle_search_char('e');
        dialog.handle_search_char('x');
        dialog.handle_search_char('i');
        dialog.handle_search_char('t');

        // After typing "exit", should have some filtered results
        // (The exact count depends on the keybindings)
        assert!(!dialog.search_query().is_empty());
    }

    #[test]
    fn test_help_item_new() {
        let item = HelpItem::new("ctrl+c", "Exit application", HelpCategory::Application);
        assert_eq!(item.key_display, "Ctrl+C");
        assert_eq!(item.key_raw, "ctrl+c");
        assert_eq!(item.description, "Exit application");
        assert_eq!(item.category, HelpCategory::Application);
        assert!(item.subcategory.is_none());
    }

    #[test]
    fn test_help_item_with_subcategory() {
        let item = HelpItem::new("return", "Submit", HelpCategory::InputControls)
            .with_subcategory("Basic");
        assert_eq!(item.subcategory, Some("Basic"));
    }

    #[test]
    fn test_build_items() {
        let config = KeybindsConfig::default();
        let items = build_items(&config);

        // Should have 95 items
        assert_eq!(items.len(), 95);

        // Check that all categories are represented
        let categories: std::collections::HashSet<HelpCategory> =
            items.iter().map(|i| i.category).collect();

        assert!(categories.contains(&HelpCategory::Application));
        assert!(categories.contains(&HelpCategory::SessionManagement));
        assert!(categories.contains(&HelpCategory::ModelSelection));
        assert!(categories.contains(&HelpCategory::MessageNavigation));
        assert!(categories.contains(&HelpCategory::CommandAgent));
        assert!(categories.contains(&HelpCategory::InputControls));
        assert!(categories.contains(&HelpCategory::SessionTree));
        assert!(categories.contains(&HelpCategory::Display));
    }
}
