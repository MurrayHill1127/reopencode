//! Model Selector Component
//!
//! A modal dialog for selecting AI models from available providers.
//! Supports favorites, recent selections, search, and keyboard navigation.
//!
//! # Features
//!
//! - Modal behavior with focus capture
//! - Keyboard navigation (j/k, arrows, Home/End)
//! - Search mode with substring matching
//! - Favorites and recent models tracking
//! - Centered positioning
//!
//! # Examples
//!
//! ```rust,ignore
//! let mut selector = ModelSelector::new();
//! selector.show();
//!
//! // In render loop
//! if selector.is_visible() {
//!     selector.render(frame, area);
//! }
//!
//! // Handle input
//! let propagation = selector.handle_input(event);
//!
//! // Get the selected model
//! if let Some((provider_id, model_id)) = selector.selected_model() {
//!     println!("Selected: {}/{}", provider_id, model_id);
//! }
//! ```

use super::{Component, ComponentEvent, ComponentId, EventPropagation};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Server URL for API calls (available for future use)
#[allow(dead_code)]
const SERVER_URL: &str = "http://127.0.0.1:4096";

// ==================== Data Structures ====================

/// Model information for display and selection
///
/// Contains the provider ID, model ID, and display name.
/// Used to represent a selectable model in the UI.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::model_selector::ModelInfo;
///
/// let model = ModelInfo::new("openai", "gpt-4", "GPT-4");
/// assert_eq!(model.provider_id(), "openai");
/// assert_eq!(model.model_id(), "gpt-4");
/// assert_eq!(model.name(), "GPT-4");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// Provider identifier (e.g., "openai", "anthropic")
    provider_id: String,
    /// Model identifier (e.g., "gpt-4", "claude-3-opus")
    model_id: String,
    /// Display name for the model
    name: String,
}

impl ModelInfo {
    /// Create a new ModelInfo
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The provider identifier
    /// * `model_id` - The model identifier
    /// * `name` - The display name
    ///
    /// # Returns
    ///
    /// A new ModelInfo instance.
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            name: name.into(),
        }
    }

    /// Get the provider ID
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Get the model ID
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Get the display name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Create a unique key for this model (provider_id:model_id)
    pub fn key(&self) -> String {
        format!("{}:{}", self.provider_id, self.model_id)
    }
}

/// Provider API response
///
/// Contains provider information including ID, name, and available models.
/// Used when fetching providers from the server API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider identifier
    pub id: String,
    /// Provider display name
    pub name: String,
    /// List of available model IDs
    pub models: Vec<String>,
}

/// Selection state tracking for favorites and recent models
///
/// Maintains lists of favorite and recently selected models,
/// with configurable maximum sizes.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::model_selector::ModelSelectorState;
///
/// let mut state = ModelSelectorState::new();
/// state.add_recent("openai", "gpt-4");
/// state.toggle_favorite("anthropic", "claude-3-opus");
///
/// assert!(state.is_favorite("anthropic", "claude-3-opus"));
/// assert!(state.is_recent("openai", "gpt-4"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ModelSelectorState {
    /// Favorite models as (provider_id, model_id) pairs
    favorites: Vec<(String, String)>,
    /// Recently selected models as (provider_id, model_id) pairs
    recent: Vec<(String, String)>,
    /// Maximum number of recent models to track
    max_recent: usize,
}

impl ModelSelectorState {
    /// Create a new ModelSelectorState with default settings
    ///
    /// Default max_recent is 10.
    pub fn new() -> Self {
        Self {
            favorites: Vec::new(),
            recent: Vec::new(),
            max_recent: 10,
        }
    }

    /// Create a new ModelSelectorState with custom max_recent
    ///
    /// # Arguments
    ///
    /// * `max_recent` - Maximum number of recent models to track
    pub fn with_max_recent(max_recent: usize) -> Self {
        Self {
            favorites: Vec::new(),
            recent: Vec::new(),
            max_recent,
        }
    }

    /// Add a model to recent list
    ///
    /// Moves the model to the front if it already exists.
    /// Removes oldest if list exceeds max_recent.
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The provider identifier
    /// * `model_id` - The model identifier
    pub fn add_recent(&mut self, provider_id: impl Into<String>, model_id: impl Into<String>) {
        let provider = provider_id.into();
        let model = model_id.into();

        // Remove if exists
        self.recent.retain(|(p, m)| p != &provider || m != &model);

        // Add to front
        self.recent.insert(0, (provider, model));

        // Trim to max size
        if self.recent.len() > self.max_recent {
            self.recent.truncate(self.max_recent);
        }
    }

    /// Toggle favorite status for a model
    ///
    /// # Arguments
    ///
    /// * `provider_id` - The provider identifier
    /// * `model_id` - The model identifier
    ///
    /// # Returns
    ///
    /// `true` if the model is now a favorite, `false` if it was removed.
    pub fn toggle_favorite(
        &mut self,
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> bool {
        let provider = provider_id.into();
        let model = model_id.into();

        if let Some(pos) = self
            .favorites
            .iter()
            .position(|(p, m)| p == &provider && m == &model)
        {
            self.favorites.remove(pos);
            false
        } else {
            self.favorites.push((provider, model));
            true
        }
    }

    /// Check if a model is a favorite
    pub fn is_favorite(&self, provider_id: &str, model_id: &str) -> bool {
        self.favorites
            .iter()
            .any(|(p, m)| p == provider_id && m == model_id)
    }

    /// Check if a model is recent
    pub fn is_recent(&self, provider_id: &str, model_id: &str) -> bool {
        self.recent
            .iter()
            .any(|(p, m)| p == provider_id && m == model_id)
    }

    /// Get the favorites list
    pub fn favorites(&self) -> &[(String, String)] {
        &self.favorites
    }

    /// Get the recent list
    pub fn recent(&self) -> &[(String, String)] {
        &self.recent
    }

    /// Clear all favorites
    pub fn clear_favorites(&mut self) {
        self.favorites.clear();
    }

    /// Clear all recent models
    pub fn clear_recent(&mut self) {
        self.recent.clear();
    }
}

// ==================== Model Selector ====================

/// Modal dialog for model selection
///
/// Displays a centered modal with a list of models from available providers.
/// Supports search, favorites, and recent models.
///
/// # Examples
///
/// ```
/// use crate::cli::commands::tui::components::model_selector::ModelSelector;
///
/// let selector = ModelSelector::new();
/// assert!(!selector.is_visible());
/// ```
pub struct ModelSelector {
    /// Unique component identifier
    id: ComponentId,
    /// Selection state (favorites, recent)
    state: ModelSelectorState,
    /// All available models
    models: Vec<ModelInfo>,
    /// Available providers
    providers: Vec<ProviderInfo>,
    /// Search query string
    search_query: String,
    /// Whether search mode is active
    search_mode: bool,
    /// Current list selection state
    list_state: ListState,
    /// Whether the dialog is visible
    visible: bool,
    /// Whether the dialog is focused
    focused: bool,
    /// Currently selected model (provider_id, model_id)
    selected_model: Option<(String, String)>,
    /// Filtered display items (cached for rendering)
    filtered_items: Vec<DisplayItem>,
}

/// Item to display in the model list
#[derive(Debug, Clone)]
struct DisplayItem {
    /// Model information
    model: ModelInfo,
    /// Category label (Favorites, Recent, or provider name)
    category: String,
    /// Whether this is a favorite
    is_favorite: bool,
    /// Whether this is recent
    #[allow(dead_code)]
    is_recent: bool,
}

impl ModelSelector {
    /// Create a new ModelSelector
    ///
    /// # Returns
    ///
    /// A new ModelSelector hidden by default with no providers loaded.
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            id: ComponentId::new(),
            state: ModelSelectorState::new(),
            models: Vec::new(),
            providers: Vec::new(),
            search_query: String::new(),
            search_mode: false,
            list_state,
            visible: false,
            focused: false,
            selected_model: None,
            filtered_items: Vec::new(),
        }
    }

    /// Create a new ModelSelector with providers
    ///
    /// # Arguments
    ///
    /// * `providers` - List of available providers
    pub fn with_providers(providers: Vec<ProviderInfo>) -> Self {
        let mut selector = Self::new();
        selector.set_providers(providers);
        selector
    }

    /// Set the available providers
    ///
    /// Populates the models list from provider information.
    pub fn set_providers(&mut self, providers: Vec<ProviderInfo>) {
        self.providers = providers.clone();
        self.models.clear();

        for provider in &providers {
            for model_id in &provider.models {
                self.models.push(ModelInfo::new(
                    &provider.id,
                    model_id,
                    model_id, // Use model_id as name for now
                ));
            }
        }

        self.update_filtered_items();
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
        self.search_mode = false;
        self.search_query.clear();
        self.update_filtered_items();
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
        self.focused = false;
        self.search_mode = false;
        self.search_query.clear();
    }

    /// Check if the dialog is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the selected model
    pub fn selected_model(&self) -> Option<&(String, String)> {
        self.selected_model.as_ref()
    }

    /// Get the current selection state
    pub fn state(&self) -> &ModelSelectorState {
        &self.state
    }

    /// Get mutable access to selection state
    pub fn state_mut(&mut self) -> &mut ModelSelectorState {
        &mut self.state
    }

    /// Update the filtered items list based on search query
    fn update_filtered_items(&mut self) {
        self.filtered_items.clear();

        let query = self.search_query.to_lowercase();

        // Add favorites first (if no search query)
        if query.is_empty() {
            for (provider_id, model_id) in &self.state.favorites {
                if let Some(model) = self.find_model(provider_id, model_id) {
                    self.filtered_items.push(DisplayItem {
                        model: model.clone(),
                        category: "Favorites".to_string(),
                        is_favorite: true,
                        is_recent: false,
                    });
                }
            }

            // Add recent (excluding favorites)
            for (provider_id, model_id) in &self.state.recent {
                if self.state.is_favorite(provider_id, model_id) {
                    continue;
                }
                if let Some(model) = self.find_model(provider_id, model_id) {
                    self.filtered_items.push(DisplayItem {
                        model: model.clone(),
                        category: "Recent".to_string(),
                        is_favorite: false,
                        is_recent: true,
                    });
                }
            }
        }

        // Add all models grouped by provider
        for provider in &self.providers {
            for model_id in &provider.models {
                let model = ModelInfo::new(&provider.id, model_id, model_id);

                // Apply search filter
                if !query.is_empty() && !self.matches_search(&model, &query) {
                    continue;
                }

                // Skip if already in favorites or recent (when no search)
                if query.is_empty()
                    && (self.state.is_favorite(&provider.id, model_id)
                        || self.state.is_recent(&provider.id, model_id))
                {
                    continue;
                }

                self.filtered_items.push(DisplayItem {
                    model,
                    category: provider.name.clone(),
                    is_favorite: self.state.is_favorite(&provider.id, model_id),
                    is_recent: self.state.is_recent(&provider.id, model_id),
                });
            }
        }

        // Ensure selection is valid
        if !self.filtered_items.is_empty() {
            let current = self.list_state.selected().unwrap_or(0);
            if current >= self.filtered_items.len() {
                self.list_state.select(Some(self.filtered_items.len() - 1));
            }
        } else {
            self.list_state.select(None);
        }
    }

    /// Find a model by provider and model ID
    fn find_model(&self, provider_id: &str, model_id: &str) -> Option<&ModelInfo> {
        self.models
            .iter()
            .find(|m| m.provider_id == provider_id && m.model_id == model_id)
    }

    /// Check if a model matches the search query
    fn matches_search(&self, model: &ModelInfo, query: &str) -> bool {
        let name_lower = model.name.to_lowercase();
        let provider_lower = model.provider_id.to_lowercase();

        // Substring match on name or provider
        name_lower.contains(query) || provider_lower.contains(query)
    }

    /// Navigate to the next item
    fn next_item(&mut self) {
        if self.filtered_items.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let next = if current >= self.filtered_items.len() - 1 {
            0
        } else {
            current + 1
        };
        self.list_state.select(Some(next));
    }

    /// Navigate to the previous item
    fn prev_item(&mut self) {
        if self.filtered_items.is_empty() {
            return;
        }
        let current = self.list_state.selected().unwrap_or(0);
        let prev = if current == 0 {
            self.filtered_items.len() - 1
        } else {
            current - 1
        };
        self.list_state.select(Some(prev));
    }

    /// Navigate to the first item
    fn first_item(&mut self) {
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Navigate to the last item
    fn last_item(&mut self) {
        if !self.filtered_items.is_empty() {
            self.list_state.select(Some(self.filtered_items.len() - 1));
        }
    }

    /// Select the current item
    fn select_current(&mut self) {
        if let Some(item) = self
            .list_state
            .selected()
            .and_then(|idx| self.filtered_items.get(idx))
        {
            let provider_id = item.model.provider_id.clone();
            let model_id = item.model.model_id.clone();

            // Add to recent
            self.state.add_recent(&provider_id, &model_id);

            // Store selection
            self.selected_model = Some((provider_id, model_id));

            // Hide dialog
            self.hide();
        }
    }

    /// Toggle favorite for the current item
    fn toggle_current_favorite(&mut self) {
        if let Some(item) = self
            .list_state
            .selected()
            .and_then(|idx| self.filtered_items.get(idx))
        {
            self.state
                .toggle_favorite(&item.model.provider_id, &item.model.model_id);
            self.update_filtered_items();
        }
    }

    /// Calculate the centered area for the dialog
    fn centered_area(area: Rect, percent_width: u16, percent_height: u16) -> Rect {
        let width = area.width * percent_width / 100;
        let height = area.height * percent_height / 100;
        let x = area.x + (area.width - width) / 2;
        let y = area.y + (area.height - height) / 2;
        Rect::new(x, y, width, height)
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ModelSelector {
    fn id(&self) -> ComponentId {
        self.id
    }

    fn render(&self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        let dialog_area = Self::centered_area(area, 60, 70);

        // Clear the area and render overlay
        frame.render_widget(Clear, dialog_area);

        // Dialog block with border
        let border_style = if self.focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let title = if self.search_mode {
            " Select Model [Search] "
        } else {
            " Select Model "
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);

        let inner = block.inner(dialog_area);
        frame.render_widget(block, dialog_area);

        // Layout: search bar (if in search mode) + list + help
        let constraints = if self.search_mode {
            vec![
                Constraint::Length(1), // Search input
                Constraint::Min(1),    // List
                Constraint::Length(1), // Help
            ]
        } else {
            vec![
                Constraint::Length(0), // No search bar
                Constraint::Min(1),    // List
                Constraint::Length(1), // Help
            ]
        };

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        // Render search bar if in search mode
        if self.search_mode {
            let search_text = if self.search_query.is_empty() {
                Line::from(Span::styled(
                    "Type to search...",
                    Style::default().fg(Color::DarkGray),
                ))
            } else {
                Line::from(vec![
                    Span::styled("Search: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&self.search_query),
                    Span::raw("_"),
                ])
            };
            let search_paragraph = Paragraph::new(search_text);
            frame.render_widget(search_paragraph, layout[0]);
        }

        // Build list items
        let items: Vec<ListItem> = self
            .filtered_items
            .iter()
            .map(|item| {
                let mut spans = vec![];

                // Favorite indicator
                if item.is_favorite {
                    spans.push(Span::styled("* ", Style::default().fg(Color::Yellow)));
                } else {
                    spans.push(Span::raw("  "));
                }

                // Model name
                spans.push(Span::raw(&item.model.name));

                // Category/provider
                spans.push(Span::styled(
                    format!(" ({})", item.category),
                    Style::default().fg(Color::DarkGray),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, layout[1], &mut self.list_state.clone());

        // Help text
        let help_spans = vec![
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::raw(" select | "),
            Span::styled("/", Style::default().fg(Color::Cyan)),
            Span::raw(" search | "),
            Span::styled("f", Style::default().fg(Color::Cyan)),
            Span::raw(" favorite | "),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::raw(" cancel"),
        ];
        let help_paragraph = Paragraph::new(Line::from(help_spans));
        frame.render_widget(help_paragraph, layout[2]);
    }

    fn handle_input(&mut self, event: KeyEvent) -> EventPropagation {
        use crossterm::event::{KeyCode, KeyModifiers};

        if !self.visible {
            return EventPropagation::Continue;
        }

        // Handle search mode separately
        if self.search_mode {
            match event.code {
                KeyCode::Esc => {
                    // Exit search mode
                    if self.search_query.is_empty() {
                        self.search_mode = false;
                    } else {
                        self.search_query.clear();
                        self.update_filtered_items();
                    }
                    EventPropagation::Stop
                }
                KeyCode::Backspace => {
                    if self.search_query.is_empty() {
                        self.search_mode = false;
                    } else {
                        self.search_query.pop();
                        self.update_filtered_items();
                    }
                    EventPropagation::Stop
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.next_item();
                    EventPropagation::Stop
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.prev_item();
                    EventPropagation::Stop
                }
                KeyCode::Enter => {
                    self.select_current();
                    EventPropagation::Stop
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.update_filtered_items();
                    EventPropagation::Stop
                }
                _ => EventPropagation::Stop,
            }
        } else {
            // Normal mode
            match (event.code, event.modifiers) {
                (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                    self.next_item();
                    EventPropagation::Stop
                }
                (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                    self.prev_item();
                    EventPropagation::Stop
                }
                (KeyCode::Home, _) | (KeyCode::Char('g'), _) => {
                    self.first_item();
                    EventPropagation::Stop
                }
                (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                    self.last_item();
                    EventPropagation::Stop
                }
                (KeyCode::Enter, _) => {
                    self.select_current();
                    EventPropagation::Stop
                }
                (KeyCode::Esc, _) => {
                    self.hide();
                    EventPropagation::Stop
                }
                (KeyCode::Char('/'), _) => {
                    self.search_mode = true;
                    self.search_query.clear();
                    EventPropagation::Stop
                }
                (KeyCode::Char('f'), _) => {
                    self.toggle_current_favorite();
                    EventPropagation::Stop
                }
                _ => EventPropagation::Continue,
            }
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

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn create_test_providers() -> Vec<ProviderInfo> {
        vec![
            ProviderInfo {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
            },
            ProviderInfo {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                models: vec!["claude-3-opus".to_string(), "claude-3-sonnet".to_string()],
            },
        ]
    }

    // ModelInfo tests
    #[test]
    fn test_model_info_new() {
        let model = ModelInfo::new("openai", "gpt-4", "GPT-4");
        assert_eq!(model.provider_id(), "openai");
        assert_eq!(model.model_id(), "gpt-4");
        assert_eq!(model.name(), "GPT-4");
    }

    #[test]
    fn test_model_info_key() {
        let model = ModelInfo::new("openai", "gpt-4", "GPT-4");
        assert_eq!(model.key(), "openai:gpt-4");
    }

    #[test]
    fn test_model_info_equality() {
        let m1 = ModelInfo::new("openai", "gpt-4", "GPT-4");
        let m2 = ModelInfo::new("openai", "gpt-4", "GPT-4");
        let m3 = ModelInfo::new("openai", "gpt-3.5-turbo", "GPT-3.5");
        assert_eq!(m1, m2);
        assert_ne!(m1, m3);
    }

    // ModelSelectorState tests
    #[test]
    fn test_state_new() {
        let state = ModelSelectorState::new();
        assert!(state.favorites().is_empty());
        assert!(state.recent().is_empty());
    }

    #[test]
    fn test_state_add_recent() {
        let mut state = ModelSelectorState::new();
        state.add_recent("openai", "gpt-4");
        assert_eq!(state.recent().len(), 1);
        assert!(state.is_recent("openai", "gpt-4"));
    }

    #[test]
    fn test_state_add_recent_moves_to_front() {
        let mut state = ModelSelectorState::new();
        state.add_recent("openai", "gpt-4");
        state.add_recent("anthropic", "claude-3-opus");
        state.add_recent("openai", "gpt-4"); // Add again

        // Should be at front
        assert_eq!(
            state.recent()[0],
            ("openai".to_string(), "gpt-4".to_string())
        );
        assert_eq!(state.recent().len(), 2);
    }

    #[test]
    fn test_state_max_recent() {
        let mut state = ModelSelectorState::with_max_recent(2);
        state.add_recent("a", "1");
        state.add_recent("b", "2");
        state.add_recent("c", "3");

        assert_eq!(state.recent().len(), 2);
        assert!(!state.is_recent("a", "1")); // Oldest removed
    }

    #[test]
    fn test_state_toggle_favorite() {
        let mut state = ModelSelectorState::new();

        // Add favorite
        let result = state.toggle_favorite("openai", "gpt-4");
        assert!(result); // Now a favorite
        assert!(state.is_favorite("openai", "gpt-4"));

        // Remove favorite
        let result = state.toggle_favorite("openai", "gpt-4");
        assert!(!result); // No longer a favorite
        assert!(!state.is_favorite("openai", "gpt-4"));
    }

    #[test]
    fn test_state_clear() {
        let mut state = ModelSelectorState::new();
        state.add_recent("a", "1");
        state.toggle_favorite("b", "2");

        state.clear_recent();
        assert!(state.recent().is_empty());
        assert!(!state.favorites().is_empty());

        state.clear_favorites();
        assert!(state.favorites().is_empty());
    }

    // ModelSelector tests
    #[test]
    fn test_selector_new() {
        let selector = ModelSelector::new();
        assert!(!selector.is_visible());
        assert!(selector.selected_model().is_none());
    }

    #[test]
    fn test_selector_default() {
        let selector: ModelSelector = Default::default();
        assert!(!selector.is_visible());
    }

    #[test]
    fn test_selector_show_hide() {
        let mut selector = ModelSelector::new();
        assert!(!selector.is_visible());

        selector.show();
        assert!(selector.is_visible());
        assert!(selector.focused());

        selector.hide();
        assert!(!selector.is_visible());
        assert!(!selector.focused());
    }

    #[test]
    fn test_selector_set_providers() {
        let mut selector = ModelSelector::new();
        selector.set_providers(create_test_providers());

        // Should have 4 models (2 per provider)
        assert!(!selector.models.is_empty());
    }

    #[test]
    fn test_selector_with_providers() {
        let selector = ModelSelector::with_providers(create_test_providers());
        assert!(!selector.models.is_empty());
    }

    #[test]
    fn test_selector_navigation() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();
        selector.update_filtered_items();

        // Should start at 0
        assert_eq!(selector.list_state.selected(), Some(0));

        // Navigate down
        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        selector.handle_input(event);
        assert_eq!(selector.list_state.selected(), Some(1));

        // Navigate up
        let event = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        selector.handle_input(event);
        assert_eq!(selector.list_state.selected(), Some(0));

        // Navigate with j/k
        selector.handle_input(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
        assert_eq!(selector.list_state.selected(), Some(1));

        selector.handle_input(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
        assert_eq!(selector.list_state.selected(), Some(0));
    }

    #[test]
    fn test_selector_home_end() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();
        selector.update_filtered_items();

        // Go to end
        selector.handle_input(KeyEvent::new(KeyCode::End, KeyModifiers::empty()));
        assert_eq!(
            selector.list_state.selected(),
            Some(selector.filtered_items.len() - 1)
        );

        // Go to start
        selector.handle_input(KeyEvent::new(KeyCode::Home, KeyModifiers::empty()));
        assert_eq!(selector.list_state.selected(), Some(0));
    }

    #[test]
    fn test_selector_home_end_with_keys() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();
        selector.update_filtered_items();

        // Test 'g' for home
        selector.handle_input(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()));
        assert_eq!(selector.list_state.selected(), Some(0));

        // Test 'G' (Shift+g) for end
        selector.handle_input(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
        assert_eq!(
            selector.list_state.selected(),
            Some(selector.filtered_items.len() - 1)
        );
    }

    #[test]
    fn test_selector_esc_cancels() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();
        assert!(selector.is_visible());

        let event = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let result = selector.handle_input(event);
        assert_eq!(result, EventPropagation::Stop);
        assert!(!selector.is_visible());
    }

    #[test]
    fn test_selector_enter_selects() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let result = selector.handle_input(event);
        assert_eq!(result, EventPropagation::Stop);
        assert!(!selector.is_visible());
        assert!(selector.selected_model().is_some());
    }

    #[test]
    fn test_selector_search_mode() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        // Enter search mode
        let event = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty());
        let result = selector.handle_input(event);
        assert_eq!(result, EventPropagation::Stop);
        assert!(selector.search_mode);

        // Type in search
        selector.handle_input(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()));
        assert_eq!(selector.search_query, "g");

        // Escape clears search
        selector.handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(selector.search_query.is_empty());

        // Escape again exits search mode
        selector.handle_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
        assert!(!selector.search_mode);
    }

    #[test]
    fn test_selector_search_filters() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        let initial_count = selector.filtered_items.len();

        // Enter search mode and type
        selector.handle_input(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        selector.handle_input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));

        // Should filter to models containing 'c'
        assert!(selector.filtered_items.len() < initial_count);
        for item in &selector.filtered_items {
            assert!(
                item.model.name.to_lowercase().contains('c')
                    || item.model.provider_id.to_lowercase().contains('c')
            );
        }
    }

    #[test]
    fn test_selector_toggle_favorite() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        // Toggle favorite on first item
        let event = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty());
        let result = selector.handle_input(event);
        assert_eq!(result, EventPropagation::Stop);

        // Check state
        if let Some(item) = selector.filtered_items.first() {
            assert!(selector
                .state
                .is_favorite(&item.model.provider_id, &item.model.model_id));
        }
    }

    #[test]
    fn test_selector_select_adds_to_recent() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        selector.handle_input(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

        // Should have added to recent
        assert!(!selector.state.recent().is_empty());
    }

    #[test]
    fn test_selector_focus() {
        let mut selector = ModelSelector::new();
        assert!(!selector.focused());

        selector.on_focus();
        assert!(selector.focused());

        selector.on_blur();
        assert!(!selector.focused());
    }

    #[test]
    fn test_selector_is_focusable() {
        let selector = ModelSelector::new();
        assert!(selector.is_focusable());
    }

    #[test]
    fn test_selector_component_id_unique() {
        let s1 = ModelSelector::new();
        let s2 = ModelSelector::new();
        assert_ne!(s1.id(), s2.id());
    }

    #[test]
    fn test_selector_handle_input_not_visible() {
        let mut selector = ModelSelector::new();
        assert!(!selector.is_visible());

        let event = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        let result = selector.handle_input(event);
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_selector_handle_event_key() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        let key_event = KeyEvent::from(KeyCode::Down);
        let component_event = ComponentEvent::Key(key_event);

        let result = selector.handle_event(&component_event);
        assert_eq!(result, EventPropagation::Stop);
    }

    #[test]
    fn test_selector_handle_event_non_key() {
        let mut selector = ModelSelector::new();
        selector.show();

        let result = selector.handle_event(&ComponentEvent::Tick);
        assert_eq!(result, EventPropagation::Continue);

        let result = selector.handle_event(&ComponentEvent::Resize(80, 24));
        assert_eq!(result, EventPropagation::Continue);
    }

    #[test]
    fn test_selector_update() {
        let mut selector = ModelSelector::new();
        // Should not panic
        selector.update(Duration::from_millis(16));
    }

    #[test]
    fn test_selector_render_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let selector = ModelSelector::new();

        terminal
            .draw(|frame| {
                let area = frame.area();
                selector.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_selector_render_visible_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        terminal
            .draw(|frame| {
                let area = frame.area();
                selector.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_selector_render_search_mode_does_not_panic() {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();
        selector.search_mode = true;

        terminal
            .draw(|frame| {
                let area = frame.area();
                selector.render(frame, area);
            })
            .unwrap();
    }

    #[test]
    fn test_centered_area_calculation() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = ModelSelector::centered_area(area, 60, 70);

        // 60% of 100 = 60 width
        assert_eq!(centered.width, 60);
        // 70% of 50 = 35 height
        assert_eq!(centered.height, 35);
        // Centered: x = (100 - 60) / 2 = 20
        assert_eq!(centered.x, 20);
        // Centered: y = (50 - 35) / 2 = 7 (integer division)
        assert_eq!(centered.y, 7);
    }

    #[test]
    fn test_state_access() {
        let mut selector = ModelSelector::new();
        selector.state_mut().add_recent("test", "model");

        assert!(selector.state().is_recent("test", "model"));
    }

    // Test backspace in search mode
    #[test]
    fn test_search_backspace() {
        let mut selector = ModelSelector::with_providers(create_test_providers());
        selector.show();

        // Enter search mode and type
        selector.handle_input(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()));
        selector.handle_input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        selector.handle_input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()));
        assert_eq!(selector.search_query, "ab");

        // Backspace removes last char
        selector.handle_input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert_eq!(selector.search_query, "a");

        // Backspace on empty exits search mode
        selector.handle_input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert!(selector.search_query.is_empty());
        assert!(selector.search_mode); // Still in search mode

        selector.handle_input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()));
        assert!(!selector.search_mode);
    }
}
