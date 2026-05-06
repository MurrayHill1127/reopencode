//! Two-phase markdown renderer for TUI transcript lines.
//!
//! ## Architecture
//!
//! - [`parse`] classifies source lines into a [`Block`] AST (width-independent).
//! - [`render_parsed`] takes the AST + terminal width → `Vec<Line>`.
//! - [`render_markdown`] is a one-shot convenience wrapper.
//!
//! [`MessageList`] caches `ParsedMarkdown` per cell and only re-runs
//! `render_parsed` on width changes.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

// ── Palette ───────────────────────────────────────────────────────────────────

pub const C_TEXT: Color = Color::Rgb(238, 238, 238);
pub const C_TEXT_MUTED: Color = Color::Rgb(128, 128, 128);
pub const C_TEXT_DIM: Color = Color::Rgb(80, 80, 80);
pub const C_PRIMARY: Color = Color::Rgb(250, 178, 131);
pub const C_ACCENT: Color = Color::Rgb(157, 124, 216);
pub const C_INFO: Color = Color::Rgb(86, 182, 194);
pub const C_SUCCESS: Color = Color::Rgb(127, 216, 143);
pub const C_ERROR: Color = Color::Rgb(224, 108, 117);
pub const C_WARNING: Color = Color::Rgb(245, 167, 66);
pub const C_BG_ELEM: Color = Color::Rgb(22, 22, 30);
pub const C_BORDER: Color = Color::Rgb(72, 72, 72);
const C_H2: Color = Color::Rgb(215, 195, 120);
const C_H3: Color = Color::Rgb(160, 200, 240);
const C_QUOTE_BAR: Color = Color::Rgb(100, 90, 140);

// ── Block AST ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading { level: usize, text: String },
    HeadingRule,
    HorizontalRule,
    ListItem { bullet: String, text: String, indent: usize },
    CodeBlock { language: String, lines: Vec<String> },
    TableRow(Vec<String>),
    Paragraph { text: String },
    Blockquote { text: String },
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown {
    pub blocks: Vec<Block>,
}

// ── Parse ─────────────────────────────────────────────────────────────────────

pub fn parse(content: &str) -> ParsedMarkdown {
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut code_fence = String::new();
    let mut code_lang = String::new();
    let mut code_lines: Vec<String> = Vec::new();

    for raw in content.lines() {
        let trimmed = raw.trim_start();

        // Code fence open / close
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let fence = &trimmed[..3];
            if !in_code {
                in_code = true;
                code_fence = fence.to_string();
                code_lang = trimmed[3..].trim().to_string();
                code_lines = Vec::new();
            } else if trimmed.starts_with(&code_fence) {
                in_code = false;
                blocks.push(Block::CodeBlock {
                    language: code_lang.clone(),
                    lines: code_lines.clone(),
                });
                code_lines.clear();
                code_lang.clear();
                code_fence.clear();
            } else {
                code_lines.push(raw.to_string());
            }
            continue;
        }

        if in_code {
            code_lines.push(raw.to_string());
            continue;
        }

        // Heading
        if let Some((level, text)) = parse_heading(trimmed) {
            blocks.push(Block::Heading { level, text: text.to_string() });
            if level == 1 {
                blocks.push(Block::HeadingRule);
            }
            continue;
        }

        // Blockquote
        if let Some(rest) = trimmed.strip_prefix("> ").or_else(|| trimmed.strip_prefix(">")) {
            blocks.push(Block::Blockquote { text: rest.to_string() });
            continue;
        }

        // List item (supports 0, 2, 4-space indent)
        let leading = raw.len() - raw.trim_start().len();
        let indent_level = leading / 2;
        if let Some((bullet, text)) = parse_list_item(trimmed) {
            blocks.push(Block::ListItem { bullet, text: text.to_string(), indent: indent_level });
            continue;
        }

        // Horizontal rule
        if is_hr(trimmed) {
            blocks.push(Block::HorizontalRule);
            continue;
        }

        // Table separator — drop
        if trimmed.starts_with('|') && trimmed.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
            continue;
        }

        // Table row
        if let Some(cells) = parse_table_row(trimmed) {
            blocks.push(Block::TableRow(cells));
            continue;
        }

        if raw.trim().is_empty() {
            blocks.push(Block::Blank);
        } else {
            blocks.push(Block::Paragraph { text: trimmed.to_string() });
        }
    }

    // Unclosed code fence — flush what we have
    if in_code && !code_lines.is_empty() {
        blocks.push(Block::CodeBlock { language: code_lang, lines: code_lines });
    }

    ParsedMarkdown { blocks }
}

fn parse_heading(s: &str) -> Option<(usize, &str)> {
    let level = s.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 { return None; }
    let rest = &s[level..];
    if !rest.starts_with(' ') && !rest.is_empty() { return None; }
    Some((level, rest.trim()))
}

fn parse_list_item(s: &str) -> Option<(String, &str)> {
    // Task list checks first — must come before generic `- ` prefix
    if let Some(rest) = s.strip_prefix("- [ ] ") { return Some(("☐".to_string(), rest)); }
    if let Some(rest) = s.strip_prefix("- [x] ").or_else(|| s.strip_prefix("- [X] ")) {
        return Some(("☑".to_string(), rest));
    }
    if let Some(rest) = s.strip_prefix("- ").or_else(|| s.strip_prefix("* ")).or_else(|| s.strip_prefix("+ ")) {
        return Some(("•".to_string(), rest));
    }
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
    clean.len() >= 3
        && (clean.chars().all(|c| c == '-')
            || clean.chars().all(|c| c == '*')
            || clean.chars().all(|c| c == '_'))
}

fn parse_table_row(s: &str) -> Option<Vec<String>> {
    if !s.starts_with('|') { return None; }
    let cells: Vec<String> = s
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect();
    if cells.is_empty() { None } else { Some(cells) }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub fn render_parsed(parsed: &ParsedMarkdown, width: u16) -> Vec<Line<'static>> {
    let w = width.max(10) as usize;
    let mut out: Vec<Line<'static>> = Vec::with_capacity(parsed.blocks.len() + 4);

    for block in &parsed.blocks {
        match block {
            Block::Heading { level, text } => {
                render_heading(&mut out, *level, text, w);
            }
            Block::HeadingRule => {
                out.push(Line::from(Span::styled(
                    "─".repeat(w.min(60)),
                    Style::default().fg(C_TEXT_DIM),
                )));
            }
            Block::HorizontalRule => {
                out.push(Line::from(Span::styled(
                    "─".repeat(w.min(60)),
                    Style::default().fg(C_BORDER),
                )));
            }
            Block::ListItem { bullet, text, indent } => {
                let pad = "  ".repeat(*indent);
                let prefix = format!("{}  {} ", pad, bullet);
                let prefix_w = prefix.width();
                let avail = w.saturating_sub(prefix_w);
                let chunks = wrap_text(text, avail.max(10));
                for (i, chunk) in chunks.into_iter().enumerate() {
                    if i == 0 {
                        let mut spans = vec![Span::styled(prefix.clone(), Style::default().fg(C_TEXT_DIM))];
                        spans.extend(render_inline(&chunk, Style::default().fg(C_TEXT)));
                        out.push(Line::from(spans));
                    } else {
                        let indent_str = " ".repeat(prefix_w);
                        let mut spans = vec![Span::raw(indent_str)];
                        spans.extend(render_inline(&chunk, Style::default().fg(C_TEXT)));
                        out.push(Line::from(spans));
                    }
                }
            }
            Block::CodeBlock { language, lines } => {
                render_code_block(&mut out, language, lines, w);
            }
            Block::TableRow(cells) => {
                let col_w = if cells.is_empty() {
                    w
                } else {
                    (w.saturating_sub(cells.len() + 1)) / cells.len()
                };
                let mut spans: Vec<Span<'static>> = vec![
                    Span::styled("│", Style::default().fg(C_BORDER)),
                ];
                for cell in cells {
                    let content = format!(" {:<width$} ", cell, width = col_w.saturating_sub(2));
                    spans.push(Span::styled(content, Style::default().fg(C_TEXT)));
                    spans.push(Span::styled("│", Style::default().fg(C_BORDER)));
                }
                out.push(Line::from(spans));
            }
            Block::Paragraph { text } => {
                for chunk in wrap_text(text, w) {
                    out.push(Line::from(render_inline(&chunk, Style::default().fg(C_TEXT))));
                }
            }
            Block::Blockquote { text } => {
                let avail = w.saturating_sub(3);
                for (i, chunk) in wrap_text(text, avail.max(10)).into_iter().enumerate() {
                    let bar = if i == 0 { "┃ " } else { "  " };
                    let mut spans = vec![Span::styled(bar, Style::default().fg(C_QUOTE_BAR))];
                    spans.extend(render_inline(&chunk, Style::default().fg(C_TEXT_MUTED)));
                    out.push(Line::from(spans));
                }
            }
            Block::Blank => {
                out.push(Line::from(""));
            }
        }
    }

    out
}

pub fn render_markdown(content: &str, width: u16) -> Vec<Line<'static>> {
    render_parsed(&parse(content), width)
}

// ── Heading renderer ──────────────────────────────────────────────────────────

fn render_heading(out: &mut Vec<Line<'static>>, level: usize, text: &str, w: usize) {
    let (style, prefix) = match level {
        1 => (
            Style::default().fg(C_PRIMARY).add_modifier(Modifier::BOLD),
            "",
        ),
        2 => (
            Style::default().fg(C_H2).add_modifier(Modifier::BOLD),
            "",
        ),
        3 => (
            Style::default().fg(C_H3).add_modifier(Modifier::BOLD),
            "  ",
        ),
        4 => (
            Style::default().fg(C_TEXT_MUTED).add_modifier(Modifier::BOLD),
            "  ",
        ),
        _ => (
            Style::default().fg(C_TEXT_DIM).add_modifier(Modifier::BOLD),
            "    ",
        ),
    };

    let full = format!("{}{}", prefix, text);
    for chunk in wrap_text(&full, w) {
        out.push(Line::from(Span::styled(chunk, style)));
    }
}

// ── Code block renderer ───────────────────────────────────────────────────────

fn render_code_block(out: &mut Vec<Line<'static>>, language: &str, lines: &[String], w: usize) {
    // Box is w-2 wide (1 char margin on each side of the content area)
    let box_w = w.saturating_sub(0);

    // Top border: ╭─ lang ──────╮
    let top = if language.is_empty() {
        format!("╭{}╮", "─".repeat(box_w.saturating_sub(2)))
    } else {
        let label = format!(" {} ", language);
        let dashes = box_w.saturating_sub(2 + label.width() + 2);
        format!("╭─{}{}╮", label, "─".repeat(dashes))
    };
    let mut top_spans: Vec<Span<'static>> = Vec::new();
    if language.is_empty() {
        top_spans.push(Span::styled(top, Style::default().fg(C_BORDER)));
    } else {
        // ╭─ + lang label in INFO + ─...╮
        let lang_label = format!(" {} ", language);
        let dashes = box_w.saturating_sub(2 + lang_label.width() + 2);
        top_spans.push(Span::styled("╭─".to_string(), Style::default().fg(C_BORDER)));
        top_spans.push(Span::styled(lang_label, Style::default().fg(C_INFO)));
        top_spans.push(Span::styled(
            format!("{}╮", "─".repeat(dashes)),
            Style::default().fg(C_BORDER),
        ));
    }
    out.push(Line::from(top_spans));

    // Content lines
    let inner_w = box_w.saturating_sub(4); // │ <content> │
    for line in lines {
        // Expand tabs to 4 spaces
        let expanded = line.replace('\t', "    ");
        let display = if expanded.width() > inner_w {
            truncate_display(&expanded, inner_w)
        } else {
            expanded.clone()
        };
        let padding = inner_w.saturating_sub(display.width());
        out.push(Line::from(vec![
            Span::styled("│ ".to_string(), Style::default().fg(C_BORDER)),
            Span::styled(display, Style::default().fg(C_TEXT_MUTED).bg(C_BG_ELEM)),
            Span::styled(
                format!("{} │", " ".repeat(padding)),
                Style::default().fg(C_BORDER).bg(C_BG_ELEM),
            ),
        ]));
    }

    // Bottom border: ╰─────────╯
    out.push(Line::from(Span::styled(
        format!("╰{}╯", "─".repeat(box_w.saturating_sub(2))),
        Style::default().fg(C_BORDER),
    )));
}

fn truncate_display(s: &str, max: usize) -> String {
    if s.width() <= max { return s.to_string(); }
    let mut out = String::new();
    for c in s.chars() {
        if out.width() + c.len_utf8() > max.saturating_sub(1) {
            out.push('…');
            break;
        }
        out.push(c);
    }
    out
}

// ── Inline renderer ───────────────────────────────────────────────────────────

pub fn render_inline(text: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut buf = String::new();

    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                spans.push(Span::styled(buf.clone(), base));
                buf.clear();
            }
        };
    }

    while i < n {
        // Bold+italic: ***text***
        if i + 2 < n && chars[i] == '*' && chars[i+1] == '*' && chars[i+2] == '*' {
            flush!();
            i += 3;
            let mut inner = String::new();
            while i + 2 < n {
                if chars[i] == '*' && chars[i+1] == '*' && chars[i+2] == '*' { i += 3; break; }
                inner.push(chars[i]); i += 1;
            }
            spans.push(Span::styled(
                inner,
                base.add_modifier(Modifier::BOLD).add_modifier(Modifier::ITALIC),
            ));
            continue;
        }

        // Bold: **text**
        if i + 1 < n && chars[i] == '*' && chars[i+1] == '*' {
            flush!();
            i += 2;
            let mut inner = String::new();
            while i + 1 < n {
                if chars[i] == '*' && chars[i+1] == '*' { i += 2; break; }
                inner.push(chars[i]); i += 1;
            }
            spans.push(Span::styled(inner, base.add_modifier(Modifier::BOLD)));
            continue;
        }

        // Bold with __text__
        if i + 1 < n && chars[i] == '_' && chars[i+1] == '_' {
            flush!();
            i += 2;
            let mut inner = String::new();
            while i + 1 < n {
                if chars[i] == '_' && chars[i+1] == '_' { i += 2; break; }
                inner.push(chars[i]); i += 1;
            }
            spans.push(Span::styled(inner, base.add_modifier(Modifier::BOLD)));
            continue;
        }

        // Inline code: `code`
        if chars[i] == '`' {
            flush!();
            i += 1;
            let mut inner = String::new();
            while i < n && chars[i] != '`' { inner.push(chars[i]); i += 1; }
            if i < n { i += 1; }
            spans.push(Span::styled(
                inner,
                Style::default().fg(C_INFO).bg(C_BG_ELEM),
            ));
            continue;
        }

        // Link: [text](url)
        if chars[i] == '[' {
            if let Some((text_end, url_end)) = find_link(&chars, i) {
                flush!();
                let link_text: String = chars[i+1..text_end].iter().collect();
                spans.push(Span::styled(
                    link_text,
                    Style::default().fg(C_INFO).add_modifier(Modifier::UNDERLINED),
                ));
                i = url_end + 1;
                continue;
            }
        }

        // Italic: *text* (single star, not followed by another)
        if chars[i] == '*'
            && i + 1 < n
            && chars[i+1] != '*'
            && chars[i+1] != ' '
        {
            flush!();
            i += 1;
            let mut inner = String::new();
            while i < n && chars[i] != '*' { inner.push(chars[i]); i += 1; }
            if i < n { i += 1; }
            spans.push(Span::styled(inner, base.add_modifier(Modifier::ITALIC)));
            continue;
        }

        // Italic: _text_ (single underscore)
        if chars[i] == '_'
            && (i == 0 || chars[i-1] == ' ' || chars[i-1] == '(')
            && i + 1 < n
            && chars[i+1] != '_'
            && chars[i+1] != ' '
        {
            flush!();
            i += 1;
            let mut inner = String::new();
            while i < n && chars[i] != '_' { inner.push(chars[i]); i += 1; }
            if i < n { i += 1; }
            spans.push(Span::styled(inner, base.add_modifier(Modifier::ITALIC)));
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    flush!();

    if spans.is_empty() {
        vec![Span::styled(String::new(), base)]
    } else {
        spans
    }
}

fn find_link(chars: &[char], start: usize) -> Option<(usize, usize)> {
    // start = index of '['
    let n = chars.len();
    let mut text_end = None;
    let mut j = start + 1;
    while j < n {
        if chars[j] == ']' { text_end = Some(j); break; }
        j += 1;
    }
    let text_end = text_end?;
    if text_end + 1 >= n || chars[text_end + 1] != '(' { return None; }
    let mut url_end = None;
    let mut k = text_end + 2;
    while k < n {
        if chars[k] == ')' { url_end = Some(k); break; }
        k += 1;
    }
    let url_end = url_end?;
    Some((text_end, url_end))
}

// ── Wrap helpers ──────────────────────────────────────────────────────────────

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 { return vec![text.to_string()]; }
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
    if !cur.is_empty() { lines.push(cur); }
    if lines.is_empty() { lines.push(String::new()); }
    lines
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
    fn parse_code_block_grouped() {
        let md = parse("```rust\nfn main() {}\nlet x = 1;\n```");
        assert_eq!(md.blocks.len(), 1);
        match &md.blocks[0] {
            Block::CodeBlock { language, lines } => {
                assert_eq!(language, "rust");
                assert_eq!(lines.len(), 2);
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn parse_code_block_no_lang() {
        let md = parse("```\ncode here\n```");
        match &md.blocks[0] {
            Block::CodeBlock { language, lines } => {
                assert_eq!(language, "");
                assert_eq!(lines.len(), 1);
            }
            _ => panic!("expected CodeBlock"),
        }
    }

    #[test]
    fn parse_blockquote() {
        let md = parse("> hello world");
        assert!(matches!(&md.blocks[0], Block::Blockquote { .. }));
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
    fn parse_task_list() {
        let md = parse("- [ ] pending\n- [x] done");
        assert_eq!(md.blocks.len(), 2);
        match &md.blocks[0] {
            Block::ListItem { bullet, .. } => assert_eq!(bullet, "☐"),
            _ => panic!(),
        }
        match &md.blocks[1] {
            Block::ListItem { bullet, .. } => assert_eq!(bullet, "☑"),
            _ => panic!(),
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
    fn parse_unclosed_fence() {
        // Unclosed fence should still produce a CodeBlock
        let md = parse("```rust\nfn foo() {}");
        assert!(matches!(&md.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn render_produces_lines() {
        let lines = render_markdown("# Hello\n\nWorld paragraph", 80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn render_code_block_has_border() {
        let lines = render_markdown("```rust\nlet x = 1;\n```", 60);
        // Should have top border, content, bottom border = 3 lines
        assert_eq!(lines.len(), 3);
        // Top border contains ╭
        let top = lines[0].spans.iter().any(|s| s.content.contains('╭'));
        assert!(top, "code block should have top border");
    }

    #[test]
    fn render_code_block_lang_label() {
        let lines = render_markdown("```python\nprint(1)\n```", 60);
        // Top line should contain language label
        let top_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(top_text.contains("python"));
    }

    #[test]
    fn render_respects_width() {
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
    fn inline_bold_italic() {
        let spans = render_inline("***both*** text", Style::default());
        assert!(spans.iter().any(|s| {
            s.style.add_modifier.contains(Modifier::BOLD)
                && s.style.add_modifier.contains(Modifier::ITALIC)
        }));
    }

    #[test]
    fn inline_code_color_and_bg() {
        let spans = render_inline("`code`", Style::default());
        assert!(spans.iter().any(|s| s.style.fg == Some(C_INFO)));
        assert!(spans.iter().any(|s| s.style.bg == Some(C_BG_ELEM)));
    }

    #[test]
    fn inline_link() {
        let spans = render_inline("[click here](https://example.com)", Style::default());
        let link_span = spans.iter().find(|s| s.content == "click here");
        assert!(link_span.is_some());
        assert!(link_span.unwrap().style.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn parse_tilde_fence() {
        let md = parse("~~~\ncode line\n~~~");
        assert_eq!(md.blocks.len(), 1);
        assert!(matches!(&md.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn heading_h1_has_rule() {
        let md = parse("# Title");
        assert_eq!(md.blocks.len(), 2);
        assert!(matches!(&md.blocks[1], Block::HeadingRule));
    }

    #[test]
    fn heading_h2_no_rule() {
        let md = parse("## Section");
        assert_eq!(md.blocks.len(), 1);
        assert!(matches!(&md.blocks[0], Block::Heading { level: 2, .. }));
    }
}
