//! Color parsing utilities for theme system
//!
//! Converts hex colors to ratatui::Color and handles color manipulation.

use ratatui::style::Color;
use std::fmt;

/// Parse a hex color string to ratatui::Color
///
/// Supports formats: #RGB, #RRGGBB, #RRGGBBAA
/// Returns None if parsing fails.
pub fn parse_hex(s: &str) -> Option<Color> {
    let s = s.strip_prefix('#')?;

    match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&s[0..2], 16).ok()?;
            let g = u8::from_str_radix(&s[2..4], 16).ok()?;
            let b = u8::from_str_radix(&s[4..6], 16).ok()?;
            let _a = u8::from_str_radix(&s[6..8], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => None,
    }
}

/// Convert an ANSI color code (0-255) to ratatui::Color
pub fn ansi_to_color(code: u8) -> Color {
    if code < 16 {
        let ansi_colors: [Color; 16] = [
            Color::Black,        // 0
            Color::Red,          // 1
            Color::Green,        // 2
            Color::Yellow,       // 3
            Color::Blue,         // 4
            Color::Magenta,      // 5
            Color::Cyan,         // 6
            Color::White,        // 7
            Color::DarkGray,     // 8 - Bright Black
            Color::LightRed,     // 9 - Bright Red
            Color::LightGreen,   // 10 - Bright Green
            Color::LightYellow,  // 11 - Bright Yellow
            Color::LightBlue,    // 12 - Bright Blue
            Color::LightMagenta, // 13 - Bright Magenta
            Color::LightCyan,    // 14 - Bright Cyan
            Color::White,        // 15 - Bright White
        ];
        ansi_colors[code as usize]
    } else if code < 232 {
        let index = code - 16;
        let b = index % 6;
        let g = (index / 6) % 6;
        let r = index / 36;

        let val = |x: u8| if x == 0 { 0 } else { x * 40 + 55 };
        Color::Rgb(val(r), val(g), val(b))
    } else {
        let gray = (code - 232) * 10 + 8;
        Color::Rgb(gray, gray, gray)
    }
}

/// Blend two colors with an alpha factor
///
/// `base` is the background, `overlay` is the foreground.
/// `alpha` should be between 0.0 (fully base) and 1.0 (fully overlay).
pub fn blend(base: Color, overlay: Color, alpha: f32) -> Color {
    let (br, bg, bb) = to_rgb(base);
    let (or, og, ob) = to_rgb(overlay);

    let r = (br as f32 + (or as f32 - br as f32) * alpha).round() as u8;
    let g = (bg as f32 + (og as f32 - bg as f32) * alpha).round() as u8;
    let b = (bb as f32 + (ob as f32 - bb as f32) * alpha).round() as u8;

    Color::Rgb(r, g, b)
}

/// Convert ratatui::Color to RGB components
pub fn to_rgb(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::White => (241, 241, 241),
        Color::DarkGray => (118, 118, 118),
        Color::LightRed => (255, 123, 123),
        Color::LightGreen => (128, 255, 128),
        Color::LightYellow => (255, 255, 113),
        Color::LightBlue => (128, 182, 255),
        Color::LightMagenta => (255, 128, 255),
        Color::LightCyan => (128, 255, 255),
        Color::Gray => (190, 190, 190),
        _ => (128, 128, 128),
    }
}

/// Calculate luminance of a color (0.0 - 1.0)
pub fn luminance(color: Color) -> f32 {
    let (r, g, b) = to_rgb(color);
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
}

/// Determine if a color is "dark" (luminance < 0.5)
pub fn is_dark(color: Color) -> bool {
    luminance(color) < 0.5
}

/// Get a contrasting foreground color (black or white) for a given background
pub fn contrasting_fg(bg: Color) -> Color {
    if is_dark(bg) {
        Color::White
    } else {
        Color::Black
    }
}

/// A wrapper around Color that provides Display and debug formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThemeColor(pub Color);

impl fmt::Display for ThemeColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (r, g, b) = to_rgb(self.0);
        write!(f, "#{:02x}{:02x}{:02x}", r, g, b)
    }
}

impl From<Color> for ThemeColor {
    fn from(color: Color) -> Self {
        ThemeColor(color)
    }
}

impl From<ThemeColor> for Color {
    fn from(tc: ThemeColor) -> Self {
        tc.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_3_digit() {
        let color = parse_hex("#abc").unwrap();
        assert_eq!(color, Color::Rgb(0xaa, 0xbb, 0xcc));
    }

    #[test]
    fn test_parse_hex_6_digit() {
        let color = parse_hex("#ff0000").unwrap();
        assert_eq!(color, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn test_parse_hex_invalid() {
        assert!(parse_hex("invalid").is_none());
        assert!(parse_hex("#gggggg").is_none());
    }

    #[test]
    fn test_blend() {
        let base = Color::Rgb(0, 0, 0);
        let overlay = Color::Rgb(255, 255, 255);
        let result = blend(base, overlay, 0.5);
        assert_eq!(result, Color::Rgb(128, 128, 128));
    }

    #[test]
    fn test_luminance() {
        assert!(luminance(Color::Black) < 0.1);
        assert!(luminance(Color::White) > 0.9);
    }

    #[test]
    fn test_contrasting_fg() {
        assert_eq!(contrasting_fg(Color::Black), Color::White);
        assert_eq!(contrasting_fg(Color::White), Color::Black);
    }
}
