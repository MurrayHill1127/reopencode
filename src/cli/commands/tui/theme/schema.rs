//! Theme JSON schema definitions
//!
//! These structs map directly to the theme JSON format used by opencode.

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Raw theme JSON structure as loaded from file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeJson {
    /// Optional JSON schema URL
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Color definitions (named references)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub defs: HashMap<String, String>,

    /// Theme color assignments
    pub theme: ThemeColorsJson,
}

/// Raw theme colors from JSON (before resolution)
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ThemeColorsJson {
    // Primary colors
    pub primary: Option<ColorValue>,
    pub secondary: Option<ColorValue>,
    pub accent: Option<ColorValue>,

    // Status colors
    pub error: Option<ColorValue>,
    pub warning: Option<ColorValue>,
    pub success: Option<ColorValue>,
    pub info: Option<ColorValue>,

    // Text colors
    pub text: Option<ColorValue>,
    #[serde(rename = "textMuted")]
    pub text_muted: Option<ColorValue>,
    #[serde(rename = "selectedListItemText")]
    pub selected_list_item_text: Option<ColorValue>,

    // Background colors
    pub background: Option<ColorValue>,
    #[serde(rename = "backgroundPanel")]
    pub background_panel: Option<ColorValue>,
    #[serde(rename = "backgroundElement")]
    pub background_element: Option<ColorValue>,
    #[serde(rename = "backgroundMenu")]
    pub background_menu: Option<ColorValue>,

    // Border colors
    pub border: Option<ColorValue>,
    #[serde(rename = "borderActive")]
    pub border_active: Option<ColorValue>,
    #[serde(rename = "borderSubtle")]
    pub border_subtle: Option<ColorValue>,

    // Diff colors
    #[serde(rename = "diffAdded")]
    pub diff_added: Option<ColorValue>,
    #[serde(rename = "diffRemoved")]
    pub diff_removed: Option<ColorValue>,
    #[serde(rename = "diffContext")]
    pub diff_context: Option<ColorValue>,
    #[serde(rename = "diffHunkHeader")]
    pub diff_hunk_header: Option<ColorValue>,
    #[serde(rename = "diffHighlightAdded")]
    pub diff_highlight_added: Option<ColorValue>,
    #[serde(rename = "diffHighlightRemoved")]
    pub diff_highlight_removed: Option<ColorValue>,
    #[serde(rename = "diffAddedBg")]
    pub diff_added_bg: Option<ColorValue>,
    #[serde(rename = "diffRemovedBg")]
    pub diff_removed_bg: Option<ColorValue>,
    #[serde(rename = "diffContextBg")]
    pub diff_context_bg: Option<ColorValue>,
    #[serde(rename = "diffLineNumber")]
    pub diff_line_number: Option<ColorValue>,
    #[serde(rename = "diffAddedLineNumberBg")]
    pub diff_added_line_number_bg: Option<ColorValue>,
    #[serde(rename = "diffRemovedLineNumberBg")]
    pub diff_removed_line_number_bg: Option<ColorValue>,

    // Markdown colors
    #[serde(rename = "markdownText")]
    pub markdown_text: Option<ColorValue>,
    #[serde(rename = "markdownHeading")]
    pub markdown_heading: Option<ColorValue>,
    #[serde(rename = "markdownLink")]
    pub markdown_link: Option<ColorValue>,
    #[serde(rename = "markdownLinkText")]
    pub markdown_link_text: Option<ColorValue>,
    #[serde(rename = "markdownCode")]
    pub markdown_code: Option<ColorValue>,
    #[serde(rename = "markdownBlockQuote")]
    pub markdown_block_quote: Option<ColorValue>,
    #[serde(rename = "markdownEmph")]
    pub markdown_emph: Option<ColorValue>,
    #[serde(rename = "markdownStrong")]
    pub markdown_strong: Option<ColorValue>,
    #[serde(rename = "markdownHorizontalRule")]
    pub markdown_horizontal_rule: Option<ColorValue>,
    #[serde(rename = "markdownListItem")]
    pub markdown_list_item: Option<ColorValue>,
    #[serde(rename = "markdownListEnumeration")]
    pub markdown_list_enumeration: Option<ColorValue>,
    #[serde(rename = "markdownImage")]
    pub markdown_image: Option<ColorValue>,
    #[serde(rename = "markdownImageText")]
    pub markdown_image_text: Option<ColorValue>,
    #[serde(rename = "markdownCodeBlock")]
    pub markdown_code_block: Option<ColorValue>,

    // Syntax colors
    #[serde(rename = "syntaxComment")]
    pub syntax_comment: Option<ColorValue>,
    #[serde(rename = "syntaxKeyword")]
    pub syntax_keyword: Option<ColorValue>,
    #[serde(rename = "syntaxFunction")]
    pub syntax_function: Option<ColorValue>,
    #[serde(rename = "syntaxVariable")]
    pub syntax_variable: Option<ColorValue>,
    #[serde(rename = "syntaxString")]
    pub syntax_string: Option<ColorValue>,
    #[serde(rename = "syntaxNumber")]
    pub syntax_number: Option<ColorValue>,
    #[serde(rename = "syntaxType")]
    pub syntax_type: Option<ColorValue>,
    #[serde(rename = "syntaxOperator")]
    pub syntax_operator: Option<ColorValue>,
    #[serde(rename = "syntaxPunctuation")]
    pub syntax_punctuation: Option<ColorValue>,

    // Special
    #[serde(rename = "thinkingOpacity")]
    pub thinking_opacity: Option<f32>,
}

/// A color value can be:
/// - A hex string: "#ff0000"
/// - A reference to a def: "darkRed"
/// - A variant with dark/light: { "dark": "#fff", "light": "#000" }
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ColorValue {
    Hex(String),
    Variant(ColorVariant),
}

/// Dark/light variant colors
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColorVariant {
    pub dark: String,
    pub light: String,
}

/// Resolved theme colors with actual Color values
#[derive(Debug, Clone)]
pub struct ResolvedColors {
    // Primary
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,

    // Status
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,

    // Text
    pub text: Color,
    pub text_muted: Color,
    pub selected_list_item_text: Color,

    // Background
    pub background: Color,
    pub background_panel: Color,
    pub background_element: Color,
    pub background_menu: Color,

    // Border
    pub border: Color,
    pub border_active: Color,
    pub border_subtle: Color,

    // Diff
    pub diff_added: Color,
    pub diff_removed: Color,
    pub diff_context: Color,
    pub diff_hunk_header: Color,
    pub diff_highlight_added: Color,
    pub diff_highlight_removed: Color,
    pub diff_added_bg: Color,
    pub diff_removed_bg: Color,
    pub diff_context_bg: Color,
    pub diff_line_number: Color,
    pub diff_added_line_number_bg: Color,
    pub diff_removed_line_number_bg: Color,

    // Markdown
    pub markdown_text: Color,
    pub markdown_heading: Color,
    pub markdown_link: Color,
    pub markdown_link_text: Color,
    pub markdown_code: Color,
    pub markdown_block_quote: Color,
    pub markdown_emph: Color,
    pub markdown_strong: Color,
    pub markdown_horizontal_rule: Color,
    pub markdown_list_item: Color,
    pub markdown_list_enumeration: Color,
    pub markdown_image: Color,
    pub markdown_image_text: Color,
    pub markdown_code_block: Color,

    // Syntax
    pub syntax_comment: Color,
    pub syntax_keyword: Color,
    pub syntax_function: Color,
    pub syntax_variable: Color,
    pub syntax_string: Color,
    pub syntax_number: Color,
    pub syntax_type: Color,
    pub syntax_operator: Color,
    pub syntax_punctuation: Color,

    // Special
    pub thinking_opacity: f32,
}

impl Default for ResolvedColors {
    fn default() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Magenta,
            accent: Color::Cyan,
            error: Color::Red,
            warning: Color::Yellow,
            success: Color::Green,
            info: Color::Cyan,
            text: Color::White,
            text_muted: Color::Gray,
            selected_list_item_text: Color::Black,
            background: Color::Reset,
            background_panel: Color::Black,
            background_element: Color::Black,
            background_menu: Color::Black,
            border: Color::Gray,
            border_active: Color::Cyan,
            border_subtle: Color::DarkGray,
            diff_added: Color::Green,
            diff_removed: Color::Red,
            diff_context: Color::Gray,
            diff_hunk_header: Color::Gray,
            diff_highlight_added: Color::LightGreen,
            diff_highlight_removed: Color::LightRed,
            diff_added_bg: Color::Black,
            diff_removed_bg: Color::Black,
            diff_context_bg: Color::Black,
            diff_line_number: Color::DarkGray,
            diff_added_line_number_bg: Color::Black,
            diff_removed_line_number_bg: Color::Black,
            markdown_text: Color::White,
            markdown_heading: Color::Magenta,
            markdown_link: Color::Cyan,
            markdown_link_text: Color::Cyan,
            markdown_code: Color::Green,
            markdown_block_quote: Color::Yellow,
            markdown_emph: Color::Yellow,
            markdown_strong: Color::White,
            markdown_horizontal_rule: Color::Gray,
            markdown_list_item: Color::Cyan,
            markdown_list_enumeration: Color::Cyan,
            markdown_image: Color::Cyan,
            markdown_image_text: Color::Cyan,
            markdown_code_block: Color::White,
            syntax_comment: Color::Gray,
            syntax_keyword: Color::Magenta,
            syntax_function: Color::Cyan,
            syntax_variable: Color::White,
            syntax_string: Color::Green,
            syntax_number: Color::Yellow,
            syntax_type: Color::Yellow,
            syntax_operator: Color::Cyan,
            syntax_punctuation: Color::White,
            thinking_opacity: 0.6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_theme_json() {
        let json = r##"{
            "defs": {
                "blue": "#0000ff"
            },
            "theme": {
                "primary": { "dark": "#fff", "light": "#000" }
            }
        }"##;

        let theme: ThemeJson = serde_json::from_str(json).unwrap();
        assert_eq!(theme.defs.get("blue"), Some(&"#0000ff".to_string()));
    }

    #[test]
    fn test_default_resolved_colors() {
        let colors = ResolvedColors::default();
        assert_eq!(colors.primary, Color::Cyan);
        assert_eq!(colors.error, Color::Red);
    }
}
