//! Two-phase markdown renderer for TUI transcript lines.
//!
//! ## Architecture
//!
//! Split into width-independent *parse* and width-dependent *render*:
//!
//! - [`parse`] classifies each source line into a [`Block`] AST. No width math.
//! - [`render_parsed`] takes the AST + terminal width and produces `Vec<Line>`.
//!   Only word-wrap and span styling happen here.
//! - [`render_markdown`] is a thin one-shot wrapper for callers that don't cache.
//!
//! The [`MessageList`] caches `ParsedMarkdown` per cell and only re-runs
//! `render_parsed` on width changes — skipping the parse entirely on resize.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

// ── Palette constants (warm-orange theme) ────────────────────────────────────

pub const C_TEXT: Color = Color::Rgb(238, 238, 238);
pub const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
pub const C_TEXT_DIM: Color = Color::Rgb(80, 80, 80);
pub const C_PRIMARY: Color = Color::Rgb(250, 178, 131); // #fab283 warm orange
pub const C_ACCENT: Color = Color::Rgb(157, 124, 216);  // #9d7cd8 purple
pub const C_INFO: Color = Color::Rgb(86, 182, 194);     // #56b6c2 cyan (inline code)
pub const C_SUCCESS: Color = Color::Rgb(127, 216, 143); // #7fd88f green
pub const C_ERROR: Color = Color::Rgb(224, 108, 117);   // #e06c75 red
pub const C_WARNING: Color = Color::Rgb(245, 167, 66);  // #f5a742 amber
pub const C_BG_ELEM: Color = Color::Rgb(30, 30, 30);    // #1e1e1e code bg
pub const C_BORDER: Color = Color::Rgb(72, 72, 72);     // #484848

// ── Block AST (width-independent) ────────────────────────────────────────────

/// One classified line of markdown — all decisions are width-independent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// ATX heading `# …` levels 1-6.
    Heading { level: usize, text: String },
    /// Horizontal rule drawn under H1.
    HeadingRule,
    /// Standalone `---` / `***` / `___` rule.
    HorizontalRule,
    /// List item with its bullet prefix and body text.
    ListItem { bullet: String, text: String },
    /// A line inside a fenced code block (fences stripped).
    Code { line: String },
    /// Markdown table row; separator rows are dropped.
    TableRow(Vec<String>),
    /// Non-empty paragraph line — may contain inline bold/italic/code/links.
    Paragraph { text: String },
    /// Empty source line — preserved for paragraph spacing.
    Blank,
}

/// Width-independent parsed AST for one message cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown {
    pub blocks: Vec<Block>,
}

// ── Parse phase ───────────────────────────────────────────────────────────────

/// Parse markdown source into a width-independent block AST.
///
/// Line-oriented, minimal — handles the patterns that matter in chat:
/// fenced code, ATX headings, dash/star/number lists, pipe tables, paragraphs.
#[must_use]
pub fn parse(content: &str) -> ParsedMarkdown {
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut code_fence = String::new();

    for raw in content.lines() {
        let trimmed = raw.trim_start();

        // Detect fence open/close
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let fence_char = &trimmed[..3];
            if !in_code {
                in_code = true;
                code_fence = fence_char.to_string();
            } else if trimmed.starts_with(&code_fence) {
                in_code = false;
                code_fence.clear();
            } else {
                blocks.push(Block::Code { line: raw.to_string() });
            }
            continue;
        }

        if in_code {
            blocks.push(Block::Code { line: raw.to_string() });
            continue;
        }

        if let Some((level, text)) = parse_heading(trimmed) {
            blocks.push(Block::Heading { level, text: text.to_string() });
            if level == 1 {
                blocks.push(Block::HeadingRule);
            }
            continue;
        }

        if let Some((bullet, text)) = parse_list_item(trimmed) {
            blocks.push(Block::ListItem { bullet, text: text.to_string() });
            continue;
        }

        if is_hr(trimmed) {
            blocks.push(Block::HorizontalRule);
            continue;
        }

        if let Some(cells) = parse_table_row(trimmed) {
            blocks.push(Block::TableRow(cells));
            continue;
        }

        // Drop table separator rows like `|---|---|`
        if trimmed.starts_with('|') && trimmed.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
            continue;
        }

        if raw.trim().is_empty() {
            blocks.push(Block::Blank);
        } else {
            blocks.push(Block::Paragraph { text: trimmed.to_string() });
        }
    }

    ParsedMarkdown { blocks }
}

fn parse_heading(s: &str) -> Option<(usize, &str)> {
    let level = s.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &s[level..];
    if !rest.starts_with(' ') && !rest.is_empty() {
        return None;
    }
    Some((level, rest.trim()))
}

fn parse_list_item(s: &str) -> Option<(String, &str)> {
    // Unordered: `- ` / `* ` / `+ `
    if let Some(rest) = s.strip_prefix("- ").or_else(|| s.strip_prefix("* ")).or_else(|| s.strip_prefix("+ ")) {
        return Some(("•".to_string(), rest));
    }
    // Ordered: `1. ` / `2. ` etc.
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &s[digits.len()..];
        if let Some(body) = rest.strip_prefix(". ") {
            return Some((format!("{}.", digits), body));
        }
    }
    None
}

fn is_hr(s: &str) -> bool {
    let clean: String = s.chars().filter(|&c| !c.is_whitespace()).collect();
    (clean.len() >= 3) && (clean.chars().all(|c| c == '-') || clean.chars().all(|c| c == '*') || clean.chars().all(|c| c == '_'))
}

fn parse_table_row(s: &str) -> Option<Vec<String>> {
    if !s.starts_with('|') {
        return None;
    }
    let cells: Vec<String> = s
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect();
    if cells.is_empty() {
        None
    } else {
        Some(cells)
    }
}

// ── Render phase ──────────────────────────────────────────────────────────────

/// Render a parsed AST at the given terminal width.
///
/// This is the only phase that knows about width — word-wrap, indentation, and
/// span styling all happen here.
#[must_use]
pub fn render_parsed(parsed: &ParsedMarkdown, width: u16) -> Vec<Line<'static>> {
    let w = width.max(10) as usize;
    let mut out: Vec<Line<'static>> = Vec::with_capacity(parsed.blocks.len() + 4);

    for block in &parsed.blocks {
        match block {
            Block::Heading { level, text } => {
                let style = Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD);
                let prefix = "#".repeat(*level) + " ";
                for line in word_wrap(&(prefix + text), w, style) {
                    out.push(line);
                }
            }
            Block::HeadingRule => {
                out.push(Line::from(Span::styled(
                    "─".repeat(w.min(50)),
                    Style::default().fg(C_TEXT_DIM),
                )));
            }
            Block::HorizontalRule => {
                out.push(Line::from(Span::styled(
                    "─".repeat(w.min(60)),
                    Style::default().fg(C_TEXT_DIM),
                )));
            }
            Block::ListItem { bullet, text } => {
                let prefix = format!("  {} ", bullet);
                let indent = prefix.width();
                let full = format!("{}{}", prefix, text);
                let wrapped = wrap_text(&full, w);
                for (i, chunk) in wrapped.into_iter().enumerate() {
                    if i == 0 {
                        out.push(Line::from(render_inline(&chunk, Style::default().fg(C_TEXT))));
                    } else {
                        let indented = format!("{}{}", " ".repeat(indent), chunk);
                        out.push(Line::from(render_inline(&indented, Style::default().fg(C_TEXT))));
                    }
                }
            }
            Block::Code { line } => {
                let display = if line.len() > w.saturating_sub(2) {
                    line[..w.saturating_sub(2)].to_string()
                } else {
                    line.clone()
                };
                // Pad to width with BG_ELEM background for the code block feel
                let padded = format!(" {:<width$}", display, width = w.saturating_sub(2));
                out.push(Line::from(Span::styled(
                    padded,
                    Style::default().fg(C_TEXT_MUTED).bg(C_BG_ELEM),
                )));
            }
            Block::TableRow(cells) => {
                let col_w = if cells.is_empty() {
                    w
                } else {
                    (w.saturating_sub(cells.len() + 1)) / cells.len()
                };
                let mut spans: Vec<Span<'static>> = vec![Span::styled("│", Style::default().fg(C_TEXT_DIM))];
                for cell in cells {
                    let content = format!(" {:<width$} ", cell, width = col_w.saturating_sub(2));
                    spans.push(Span::styled(content, Style::default().fg(C_TEXT)));
                    spans.push(Span::styled("│", Style::default().fg(C_TEXT_DIM)));
                }
                out.push(Line::from(spans));
            }
            Block::Paragraph { text } => {
                for chunk in wrap_text(text, w) {
                    out.push(Line::from(render_inline(&chunk, Style::default().fg(C_TEXT))));
                }
            }
            Block::Blank => {
                out.push(Line::from(""));
            }
        }
    }

    out
}

/// Convenience wrapper: parse + render in one call.
#[must_use]
pub fn render_markdown(content: &str, width: u16) -> Vec<Line<'static>> {
    render_parsed(&parse(content), width)
}

// ── Inline rendering ──────────────────────────────────────────────────────────

/// Render inline markdown spans: `**bold**`, `*italic*`, `` `code` ``, `_italic_`.
fn render_inline(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut buf = String::new();

    while i < chars.len() {
        // Bold: **text**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
            if !buf.is_empty() {
                spans.push(Span::styled(buf.clone(), base));
                buf.clear();
            }
            i += 2;
            let mut inner = String::new();
            while i < chars.len() {
                if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                    i += 2;
                    break;
                }
                inner.push(chars[i]);
                i += 1;
            }
            spans.push(Span::styled(inner, base.add_modifier(Modifier::BOLD)));
            continue;
        }

        // Inline code: `code`
        if chars[i] == '`' {
            if !buf.is_empty() {
                spans.push(Span::styled(buf.clone(), base));
                buf.clear();
            }
            i += 1;
            let mut inner = String::new();
            while i < chars.len() && chars[i] != '`' {
                inner.push(chars[i]);
                i += 1;
            }
            if i < chars.len() { i += 1; } // closing `
            spans.push(Span::styled(inner, Style::default().fg(C_INFO)));
            continue;
        }

        // Italic: *text* or _text_
        let italic_char = chars[i];
        if (italic_char == '*' || italic_char == '_')
            && (i == 0 || chars[i - 1] != italic_char)
            && i + 1 < chars.len()
            && chars[i + 1] != italic_char
            && chars[i + 1] != ' '
        {
            if !buf.is_empty() {
                spans.push(Span::styled(buf.clone(), base));
                buf.clear();
            }
            i += 1;
            let mut inner = String::new();
            while i < chars.len() && chars[i] != italic_char {
                inner.push(chars[i]);
                i += 1;
            }
            if i < chars.len() { i += 1; }
            spans.push(Span::styled(inner, base.add_modifier(Modifier::ITALIC)));
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    if !buf.is_empty() {
        spans.push(Span::styled(buf, base));
    }

    if spans.is_empty() {
        vec![Span::styled(String::new(), base)]
    } else {
        spans
    }
}

// ── Word-wrap helpers ─────────────────────────────────────────────────────────

/// Wrap `text` to `width` columns, returning a Vec of chunk strings.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_w = 0usize;

    for word in text.split_whitespace() {
        let ww = word.width();
        if cur_w == 0 {
            cur.push_str(word);
            cur_w = ww;
        } else if cur_w + 1 + ww <= width {
            cur.push(' ');
            cur.push_str(word);
            cur_w += 1 + ww;
        } else {
            lines.push(cur.clone());
            cur = word.to_string();
            cur_w = ww;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Wrap a pre-formatted string (with prefix) into styled Lines.
fn word_wrap(text: &str, width: usize, style: Style) -> Vec<Line<'static>> {
    wrap_text(text, width)
        .into_iter()
        .map(|s| Line::from(Span::styled(s, style)))
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_heading_levels() {
        let md = parse("# H1\n## H2\n### H3");
        assert!(matches!(&md.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(&md.blocks[1], Block::HeadingRule));
        assert!(matches!(&md.blocks[2], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn parse_code_block() {
        let md = parse("```\nfn main() {}\n```");
        assert_eq!(md.blocks.len(), 1);
        assert!(matches!(&md.blocks[0], Block::Code { .. }));
    }

    #[test]
    fn parse_list_items() {
        let md = parse("- foo\n* bar\n1. baz");
        assert_eq!(md.blocks.len(), 3);
        for b in &md.blocks {
            assert!(matches!(b, Block::ListItem { .. }));
        }
    }

    #[test]
    fn parse_horizontal_rule() {
        let md = parse("---");
        assert!(matches!(md.blocks[0], Block::HorizontalRule));
    }

    #[test]
    fn parse_blank_lines() {
        let md = parse("hello\n\nworld");
        assert_eq!(md.blocks.len(), 3);
        assert!(matches!(md.blocks[1], Block::Blank));
    }

    #[test]
    fn render_produces_lines() {
        let lines = render_markdown("# Hello\n\nWorld paragraph", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_code_block_has_bg() {
        let lines = render_markdown("```\nlet x = 1;\n```", 80);
        assert!(!lines.is_empty());
        // Code line should have BG_ELEM background
        let has_bg = lines[0].spans.iter().any(|s| s.style.bg == Some(C_BG_ELEM));
        assert!(has_bg);
    }

    #[test]
    fn render_respects_width() {
        // Long paragraph should wrap
        let long = "word ".repeat(30);
        let lines = render_markdown(&long, 40);
        assert!(lines.len() > 1);
    }

    #[test]
    fn wrap_text_basic() {
        let chunks = wrap_text("hello world foo bar", 10);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn inline_bold() {
        let spans = render_inline("**bold** text", Style::default());
        assert!(spans.iter().any(|s| s.style.add_modifier.contains(Modifier::BOLD)));
    }

    #[test]
    fn inline_code_color() {
        let spans = render_inline("`code`", Style::default());
        assert!(spans.iter().any(|s| s.style.fg == Some(C_INFO)));
    }

    #[test]
    fn parse_tilde_fence() {
        let md = parse("~~~\ncode line\n~~~");
        assert_eq!(md.blocks.len(), 1);
        assert!(matches!(&md.blocks[0], Block::Code { .. }));
    }
}
