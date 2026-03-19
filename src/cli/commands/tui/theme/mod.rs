//! Theme System for TUI
//!
//! Provides theme loading, resolution, and runtime switching for the TUI.
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::cli::commands::tui::theme::{ThemeContext, ThemeMode};
//!
//! // Create a theme context with default theme
//! let ctx = ThemeContext::new("opencode", ThemeMode::Dark);
//!
//! // Access resolved colors
//! let primary = ctx.colors().primary;
//!
//! // Switch theme at runtime
//! ctx.set_theme("dracula");
//!
//! // Toggle dark/light mode
//! ctx.toggle_mode();
//! ```

pub mod builtins;
pub mod color;
pub mod resolve;
pub mod schema;

use once_cell::sync::Lazy;
use ratatui::style::Color;
use schema::{ResolvedColors, ThemeJson};
use std::collections::HashMap;
use std::sync::RwLock;

pub use resolve::{ThemeMode, resolve_theme};

/// Global theme registry (lazy-initialized)
static THEME_REGISTRY: Lazy<ThemeRegistry> = Lazy::new(ThemeRegistry::new);

/// Theme registry - manages all available themes
pub struct ThemeRegistry {
    themes: RwLock<HashMap<String, ThemeJson>>,
}

impl ThemeRegistry {
    fn new() -> Self {
        let mut themes = HashMap::new();

        // Load built-in themes
        for (name, theme) in builtins::load_builtin_themes() {
            themes.insert(name.to_string(), theme);
        }

        Self {
            themes: RwLock::new(themes),
        }
    }

    /// Get a theme by name
    pub fn get(&self, name: &str) -> Option<ThemeJson> {
        let themes = self.themes.read().unwrap();
        themes.get(name).cloned()
    }

    /// List all available theme names
    pub fn list(&self) -> Vec<String> {
        let themes = self.themes.read().unwrap();
        let mut names: Vec<String> = themes.keys().cloned().collect();
        names.sort();
        names
    }

    /// Add a custom theme
    pub fn add(&self, name: String, theme: ThemeJson) {
        let mut themes = self.themes.write().unwrap();
        themes.insert(name, theme);
    }

    /// Check if a theme exists
    pub fn contains(&self, name: &str) -> bool {
        let themes = self.themes.read().unwrap();
        themes.contains_key(name)
    }
}

/// Get the global theme registry
pub fn registry() -> &'static ThemeRegistry {
    &THEME_REGISTRY
}

/// Theme context - holds the current theme and mode
#[derive(Debug, Clone)]
pub struct ThemeContext {
    theme_name: String,
    mode: ThemeMode,
    colors: ResolvedColors,
}

impl ThemeContext {
    /// Create a new theme context
    pub fn new(theme_name: &str, mode: ThemeMode) -> Self {
        let colors = Self::resolve(theme_name, mode);
        Self {
            theme_name: theme_name.to_string(),
            mode,
            colors,
        }
    }

    /// Create with default theme (opencode, dark mode)
    pub fn default_theme() -> Self {
        Self::new("opencode", ThemeMode::Dark)
    }

    /// Resolve a theme by name
    fn resolve(name: &str, mode: ThemeMode) -> ResolvedColors {
        match registry().get(name) {
            Some(theme) => resolve_theme(&theme, mode),
            None => {
                // Fallback to opencode theme
                if let Some(fallback) = registry().get("opencode") {
                    resolve_theme(&fallback, mode)
                } else {
                    ResolvedColors::default()
                }
            }
        }
    }

    /// Get the current resolved colors
    pub fn colors(&self) -> &ResolvedColors {
        &self.colors
    }

    /// Get the current theme name
    pub fn theme_name(&self) -> &str {
        &self.theme_name
    }

    /// Get the current mode
    pub fn mode(&self) -> ThemeMode {
        self.mode
    }

    /// Set the theme
    pub fn set_theme(&mut self, name: &str) {
        if registry().contains(name) {
            self.theme_name = name.to_string();
            self.colors = Self::resolve(name, self.mode);
        }
    }

    /// Set the mode
    pub fn set_mode(&mut self, mode: ThemeMode) {
        self.mode = mode;
        self.colors = Self::resolve(&self.theme_name, mode);
    }

    /// Toggle between dark and light mode
    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
        };
        self.colors = Self::resolve(&self.theme_name, self.mode);
    }

    /// Check if currently in dark mode
    pub fn is_dark(&self) -> bool {
        self.mode == ThemeMode::Dark
    }

    /// Get primary color
    pub fn primary(&self) -> Color {
        self.colors.primary
    }

    /// Get secondary color
    pub fn secondary(&self) -> Color {
        self.colors.secondary
    }

    /// Get accent color
    pub fn accent(&self) -> Color {
        self.colors.accent
    }

    /// Get error color
    pub fn error(&self) -> Color {
        self.colors.error
    }

    /// Get warning color
    pub fn warning(&self) -> Color {
        self.colors.warning
    }

    /// Get success color
    pub fn success(&self) -> Color {
        self.colors.success
    }

    /// Get info color
    pub fn info(&self) -> Color {
        self.colors.info
    }

    /// Get text color
    pub fn text(&self) -> Color {
        self.colors.text
    }

    /// Get muted text color
    pub fn text_muted(&self) -> Color {
        self.colors.text_muted
    }

    /// Get background color
    pub fn background(&self) -> Color {
        self.colors.background
    }

    /// Get border color
    pub fn border(&self) -> Color {
        self.colors.border
    }

    /// Get active border color
    pub fn border_active(&self) -> Color {
        self.colors.border_active
    }
}

impl Default for ThemeContext {
    fn default() -> Self {
        Self::default_theme()
    }
}

/// List all available themes
pub fn list_themes() -> Vec<String> {
    registry().list()
}

/// Check if a theme exists
pub fn theme_exists(name: &str) -> bool {
    registry().contains(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_context_default() {
        let ctx = ThemeContext::default();
        assert_eq!(ctx.theme_name(), "opencode");
        assert_eq!(ctx.mode(), ThemeMode::Dark);
    }

    #[test]
    fn test_theme_context_switch() {
        let mut ctx = ThemeContext::default();
        ctx.set_theme("dracula");
        assert_eq!(ctx.theme_name(), "dracula");
    }

    #[test]
    fn test_theme_context_toggle_mode() {
        let mut ctx = ThemeContext::default();
        assert!(ctx.is_dark());
        ctx.toggle_mode();
        assert!(!ctx.is_dark());
    }

    #[test]
    fn test_list_themes() {
        let themes = list_themes();
        assert!(themes.contains(&"opencode".to_string()));
        assert!(themes.contains(&"dracula".to_string()));
    }
}
