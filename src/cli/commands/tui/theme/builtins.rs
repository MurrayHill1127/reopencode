//! Built-in themes embedded in the binary
//!
//! Each theme is loaded via include_str! at compile time.

use super::schema::ThemeJson;
use std::collections::HashMap;

/// Load all built-in themes
pub fn load_builtin_themes() -> HashMap<&'static str, ThemeJson> {
    let mut themes = HashMap::new();

    // Load each theme
    themes.insert("opencode", load_theme(include_str!("data/opencode.json")));
    themes.insert(
        "tokyonight",
        load_theme(include_str!("data/tokyonight.json")),
    );
    themes.insert("dracula", load_theme(include_str!("data/dracula.json")));
    themes.insert("nord", load_theme(include_str!("data/nord.json")));
    themes.insert("gruvbox", load_theme(include_str!("data/gruvbox.json")));
    themes.insert(
        "catppuccin",
        load_theme(include_str!("data/catppuccin.json")),
    );
    themes.insert(
        "catppuccin-frappe",
        load_theme(include_str!("data/catppuccin-frappe.json")),
    );
    themes.insert(
        "catppuccin-macchiato",
        load_theme(include_str!("data/catppuccin-macchiato.json")),
    );
    themes.insert("monokai", load_theme(include_str!("data/monokai.json")));
    themes.insert("one-dark", load_theme(include_str!("data/one-dark.json")));
    themes.insert("github", load_theme(include_str!("data/github.json")));
    themes.insert("aura", load_theme(include_str!("data/aura.json")));
    themes.insert("ayu", load_theme(include_str!("data/ayu.json")));
    themes.insert("cobalt2", load_theme(include_str!("data/cobalt2.json")));
    themes.insert("cursor", load_theme(include_str!("data/cursor.json")));
    themes.insert(
        "everforest",
        load_theme(include_str!("data/everforest.json")),
    );
    themes.insert("flexoki", load_theme(include_str!("data/flexoki.json")));
    themes.insert("kanagawa", load_theme(include_str!("data/kanagawa.json")));
    themes.insert("material", load_theme(include_str!("data/material.json")));
    themes.insert("matrix", load_theme(include_str!("data/matrix.json")));
    themes.insert("mercury", load_theme(include_str!("data/mercury.json")));
    themes.insert("nightowl", load_theme(include_str!("data/nightowl.json")));
    themes.insert(
        "osaka-jade",
        load_theme(include_str!("data/osaka-jade.json")),
    );
    themes.insert("orng", load_theme(include_str!("data/orng.json")));
    themes.insert(
        "lucent-orng",
        load_theme(include_str!("data/lucent-orng.json")),
    );
    themes.insert("palenight", load_theme(include_str!("data/palenight.json")));
    themes.insert("rosepine", load_theme(include_str!("data/rosepine.json")));
    themes.insert("solarized", load_theme(include_str!("data/solarized.json")));
    themes.insert(
        "synthwave84",
        load_theme(include_str!("data/synthwave84.json")),
    );
    themes.insert("vesper", load_theme(include_str!("data/vesper.json")));
    themes.insert("vercel", load_theme(include_str!("data/vercel.json")));
    themes.insert("zenburn", load_theme(include_str!("data/zenburn.json")));
    themes.insert("carbonfox", load_theme(include_str!("data/carbonfox.json")));

    themes
}

/// Load a single theme from JSON string
fn load_theme(json: &'static str) -> ThemeJson {
    serde_json::from_str(json).expect("Failed to parse built-in theme JSON")
}

/// List of all built-in theme names
pub const BUILTIN_THEME_NAMES: &[&str] = &[
    "opencode",
    "tokyonight",
    "dracula",
    "nord",
    "gruvbox",
    "catppuccin",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "monokai",
    "one-dark",
    "github",
    "aura",
    "ayu",
    "cobalt2",
    "cursor",
    "everforest",
    "flexoki",
    "kanagawa",
    "material",
    "matrix",
    "mercury",
    "nightowl",
    "osaka-jade",
    "orng",
    "lucent-orng",
    "palenight",
    "rosepine",
    "solarized",
    "synthwave84",
    "vesper",
    "vercel",
    "zenburn",
    "carbonfox",
];
