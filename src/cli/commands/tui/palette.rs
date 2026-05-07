//! Unified colour palette — Tokyo Night inspired, tuned for Ghostty terminal.
//!
//! All TUI components import from here so the look stays consistent.

use ratatui::style::Color;

// ── Backgrounds ───────────────────────────────────────────────────────────────

/// Main app background — dark blue-grey, not pure black.
pub const BG: Color = Color::Rgb(26, 27, 38);
/// Elevated surface — cards, panels, code blocks.
pub const SURFACE: Color = Color::Rgb(30, 32, 48);
/// Slightly lighter surface for hover / selection.
pub const SURFACE_HI: Color = Color::Rgb(41, 44, 60);
/// Reasoning block warm tint.
pub const SURFACE_REASONING: Color = Color::Rgb(36, 30, 20);

// ── Text ──────────────────────────────────────────────────────────────────────

pub const TEXT: Color = Color::Rgb(192, 202, 245);
pub const TEXT_MUTED: Color = Color::Rgb(132, 140, 180);
pub const TEXT_DIM: Color = Color::Rgb(86, 92, 120);

// ── Accent ────────────────────────────────────────────────────────────────────

pub const PRIMARY: Color = Color::Rgb(122, 162, 247);   // blue
pub const SECONDARY: Color = Color::Rgb(187, 154, 247); // purple
pub const INFO: Color = Color::Rgb(125, 207, 255);      // cyan
pub const SUCCESS: Color = Color::Rgb(158, 206, 106);   // green
pub const WARNING: Color = Color::Rgb(224, 175, 104);   // amber
pub const ERROR: Color = Color::Rgb(247, 118, 142);     // rose

// ── Borders ───────────────────────────────────────────────────────────────────

pub const BORDER: Color = Color::Rgb(56, 58, 78);
pub const BORDER_ACTIVE: Color = Color::Rgb(86, 90, 110);
pub const BORDER_ACCENT: Color = Color::Rgb(122, 162, 247);

// ── Diff ──────────────────────────────────────────────────────────────────────

pub const DIFF_ADD: Color = Color::Rgb(115, 200, 120);
pub const DIFF_DEL: Color = Color::Rgb(235, 100, 120);

// ── Heading ───────────────────────────────────────────────────────────────────

pub const H1: Color = Color::Rgb(255, 158, 100); // warm
pub const H2: Color = Color::Rgb(224, 175, 104); // amber
pub const H3: Color = Color::Rgb(158, 206, 186); // teal-ish green

// ── Quote ─────────────────────────────────────────────────────────────────────

pub const QUOTE_BAR: Color = Color::Rgb(86, 82, 130);
