//! Theme color resolution
//!
//! Resolves color references and dark/light variants from raw theme JSON.

use super::color::parse_hex;
use super::schema::{ColorValue, ResolvedColors, ThemeJson};
use ratatui::style::Color;
use std::collections::{HashMap, HashSet};

/// Theme mode (dark or light)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Resolve a theme JSON to resolved colors
pub fn resolve_theme(theme: &ThemeJson, mode: ThemeMode) -> ResolvedColors {
    let defs = &theme.defs;
    let colors = &theme.theme;

    let resolve = |value: Option<&ColorValue>| -> Color {
        match value {
            None => Color::Reset,
            Some(ColorValue::Hex(hex)) => resolve_color_value(hex, defs, mode),
            Some(ColorValue::Variant(variant)) => {
                let hex = match mode {
                    ThemeMode::Dark => &variant.dark,
                    ThemeMode::Light => &variant.light,
                };
                resolve_color_value(hex, defs, mode)
            }
        }
    };

    ResolvedColors {
        primary: resolve(colors.primary.as_ref()),
        secondary: resolve(colors.secondary.as_ref()),
        accent: resolve(colors.accent.as_ref()),
        error: resolve(colors.error.as_ref()),
        warning: resolve(colors.warning.as_ref()),
        success: resolve(colors.success.as_ref()),
        info: resolve(colors.info.as_ref()),
        text: resolve(colors.text.as_ref()),
        text_muted: resolve(colors.text_muted.as_ref()),
        selected_list_item_text: resolve(colors.selected_list_item_text.as_ref()),
        background: resolve(colors.background.as_ref()),
        background_panel: resolve(colors.background_panel.as_ref()),
        background_element: resolve(colors.background_element.as_ref()),
        background_menu: resolve(colors.background_menu.as_ref()),
        border: resolve(colors.border.as_ref()),
        border_active: resolve(colors.border_active.as_ref()),
        border_subtle: resolve(colors.border_subtle.as_ref()),
        diff_added: resolve(colors.diff_added.as_ref()),
        diff_removed: resolve(colors.diff_removed.as_ref()),
        diff_context: resolve(colors.diff_context.as_ref()),
        diff_hunk_header: resolve(colors.diff_hunk_header.as_ref()),
        diff_highlight_added: resolve(colors.diff_highlight_added.as_ref()),
        diff_highlight_removed: resolve(colors.diff_highlight_removed.as_ref()),
        diff_added_bg: resolve(colors.diff_added_bg.as_ref()),
        diff_removed_bg: resolve(colors.diff_removed_bg.as_ref()),
        diff_context_bg: resolve(colors.diff_context_bg.as_ref()),
        diff_line_number: resolve(colors.diff_line_number.as_ref()),
        diff_added_line_number_bg: resolve(colors.diff_added_line_number_bg.as_ref()),
        diff_removed_line_number_bg: resolve(colors.diff_removed_line_number_bg.as_ref()),
        markdown_text: resolve(colors.markdown_text.as_ref()),
        markdown_heading: resolve(colors.markdown_heading.as_ref()),
        markdown_link: resolve(colors.markdown_link.as_ref()),
        markdown_link_text: resolve(colors.markdown_link_text.as_ref()),
        markdown_code: resolve(colors.markdown_code.as_ref()),
        markdown_block_quote: resolve(colors.markdown_block_quote.as_ref()),
        markdown_emph: resolve(colors.markdown_emph.as_ref()),
        markdown_strong: resolve(colors.markdown_strong.as_ref()),
        markdown_horizontal_rule: resolve(colors.markdown_horizontal_rule.as_ref()),
        markdown_list_item: resolve(colors.markdown_list_item.as_ref()),
        markdown_list_enumeration: resolve(colors.markdown_list_enumeration.as_ref()),
        markdown_image: resolve(colors.markdown_image.as_ref()),
        markdown_image_text: resolve(colors.markdown_image_text.as_ref()),
        markdown_code_block: resolve(colors.markdown_code_block.as_ref()),
        syntax_comment: resolve(colors.syntax_comment.as_ref()),
        syntax_keyword: resolve(colors.syntax_keyword.as_ref()),
        syntax_function: resolve(colors.syntax_function.as_ref()),
        syntax_variable: resolve(colors.syntax_variable.as_ref()),
        syntax_string: resolve(colors.syntax_string.as_ref()),
        syntax_number: resolve(colors.syntax_number.as_ref()),
        syntax_type: resolve(colors.syntax_type.as_ref()),
        syntax_operator: resolve(colors.syntax_operator.as_ref()),
        syntax_punctuation: resolve(colors.syntax_punctuation.as_ref()),
        thinking_opacity: colors.thinking_opacity.unwrap_or(0.6),
    }
}

/// Resolve a single color value (hex string or reference)
fn resolve_color_value(value: &str, defs: &HashMap<String, String>, mode: ThemeMode) -> Color {
    // Handle special values
    match value {
        "transparent" | "none" => return Color::Reset,
        _ => {}
    }

    // Direct hex color
    if value.starts_with('#') {
        return parse_hex(value).unwrap_or(Color::Reset);
    }

    // Reference to a def
    if let Some(def_value) = defs.get(value) {
        // Prevent circular references
        let mut visited = HashSet::new();
        visited.insert(value.to_string());
        return resolve_def_recursive(def_value, defs, mode, visited);
    }

    // Fallback
    Color::Reset
}

/// Recursively resolve a def value, detecting cycles
fn resolve_def_recursive(
    value: &str,
    defs: &HashMap<String, String>,
    _mode: ThemeMode,
    mut visited: HashSet<String>,
) -> Color {
    // Handle special values
    match value {
        "transparent" | "none" => return Color::Reset,
        _ => {}
    }

    // Direct hex color
    if value.starts_with('#') {
        return parse_hex(value).unwrap_or(Color::Reset);
    }

    // Check for circular reference
    if visited.contains(value) {
        return Color::Reset; // Cycle detected, return fallback
    }

    // Try to resolve from defs
    if let Some(def_value) = defs.get(value) {
        visited.insert(value.to_string());
        return resolve_def_recursive(def_value, defs, _mode, visited);
    }

    // Fallback
    Color::Reset
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_resolve_simple_hex() {
        let json = r##"{
            "theme": {
                "primary": "#ff0000"
            }
        }"##;
        let theme: ThemeJson = serde_json::from_str(json).unwrap();
        let resolved = resolve_theme(&theme, ThemeMode::Dark);
        assert_eq!(resolved.primary, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_resolve_variant() {
        let json = r##"{
            "theme": {
                "primary": { "dark": "#ffffff", "light": "#000000" }
            }
        }"##;
        let theme: ThemeJson = serde_json::from_str(json).unwrap();

        let dark = resolve_theme(&theme, ThemeMode::Dark);
        assert_eq!(dark.primary, Color::Rgb(255, 255, 255));

        let light = resolve_theme(&theme, ThemeMode::Light);
        assert_eq!(light.primary, Color::Rgb(0, 0, 0));
    }

    #[test]
    fn test_resolve_def_reference() {
        let json = r##"{
            "defs": {
                "myBlue": "#0000ff"
            },
            "theme": {
                "primary": "myBlue"
            }
        }"##;
        let theme: ThemeJson = serde_json::from_str(json).unwrap();
        let resolved = resolve_theme(&theme, ThemeMode::Dark);
        assert_eq!(resolved.primary, Color::Rgb(0, 0, 255));
    }
}
