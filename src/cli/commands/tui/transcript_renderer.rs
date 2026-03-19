//! Transcript Renderer - Converts Markdown strings to ratatui Text with syntax highlighting
//!
//! This module provides a renderer that parses Markdown using `pulldown-cmark`
//! and converts it to `ratatui::text::Text` for display in the TUI.
//! Code blocks are syntax-highlighted using `syntect`.

use pulldown_cmark::{self, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

/// Renderer that converts Markdown to ratatui Text with syntax highlighting
///
/// Holds syntect's `SyntaxSet` which is loaded once during initialization.
/// This is relatively expensive to load, so it should be reused across multiple renders.
pub struct TranscriptRenderer {
    /// Syntax set for detecting code block languages
    syntax_set: SyntaxSet,
    /// Theme for syntax highlighting colors
    theme: syntect::highlighting::Theme,
}

impl Default for TranscriptRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscriptRenderer {
    /// Create a new TranscriptRenderer with default syntax sets and themes
    ///
    /// # Example
    /// ```
    /// use crate::cli::commands::tui::transcript_renderer::TranscriptRenderer;
    /// let renderer = TranscriptRenderer::new();
    /// let text = renderer.render("# Hello\n\nWorld");
    /// ```
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        // Use a dark theme by default (suitable for terminal)
        let theme = theme_set
            .themes
            .get("base16-ocean.dark")
            .cloned()
            .unwrap_or_else(|| {
                theme_set
                    .themes
                    .get("base16-ocean.light")
                    .cloned()
                    .unwrap_or_else(|| {
                        theme_set
                            .themes
                            .values()
                            .next()
                            .cloned()
                            .unwrap_or_else(|| {
                                ThemeSet::load_defaults().themes["base16-ocean.dark"].clone()
                            })
                    })
            });

        Self { syntax_set, theme }
    }

    /// Render Markdown string to ratatui Text
    ///
    /// # Arguments
    /// * `markdown` - The Markdown string to render
    ///
    /// # Returns
    /// A `ratatui::text::Text<'static>` that can be rendered in a Paragraph widget
    ///
    /// # Example
    /// ```
    /// let renderer = TranscriptRenderer::new();
    /// let text = renderer.render("# Header\n\nSome **bold** text");
    /// ```
    pub fn render(&self, markdown: &str) -> Text<'static> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let mut current_line_spans: Vec<Span<'static>> = Vec::new();
        let mut in_code_block = false;
        let mut code_block_language: String = String::new();
        let mut code_block_content: Vec<String> = Vec::new();

        // Track inline formatting state
        let mut is_bold = false;
        let mut is_italic = false;
        let mut in_heading = false;
        let mut heading_level: u32 = 0;
        let mut in_list_item = false;

        let parser_options = Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_TASKLISTS;

        let parser = Parser::new_ext(markdown, parser_options);

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading { level, .. } => {
                            in_heading = true;
                            heading_level = level as u32;
                            is_bold = true;
                        }
                        Tag::Paragraph => {
                            // Start of paragraph - nothing special needed
                        }
                        Tag::CodeBlock(kind) => {
                            in_code_block = true;
                            // Extract language from code block info
                            match kind {
                                pulldown_cmark::CodeBlockKind::Fenced(info) => {
                                    code_block_language = info.to_string();
                                }
                                pulldown_cmark::CodeBlockKind::Indented => {
                                    code_block_language = String::new();
                                }
                            }
                        }
                        Tag::Strong => {
                            is_bold = true;
                        }
                        Tag::Emphasis => {
                            is_italic = true;
                        }
                        Tag::List(_) => {
                            // List container - nothing special
                        }
                        Tag::Item => {
                            in_list_item = true;
                            current_line_spans.push(Span::raw("  • "));
                        }
                        _ => {}
                    }
                }
                Event::End(tag) => {
                    match tag {
                        TagEnd::Heading(_) => {
                            in_heading = false;
                            is_bold = false;
                            // Flush current line and add spacing
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.clone()));
                                current_line_spans.clear();
                            }
                            lines.push(Line::from(""));
                        }
                        TagEnd::Paragraph => {
                            // End of paragraph - flush line and add blank line
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.clone()));
                                current_line_spans.clear();
                                lines.push(Line::from(""));
                            }
                        }
                        TagEnd::CodeBlock => {
                            in_code_block = false;
                            // Render the code block with syntax highlighting
                            let highlighted_lines = self
                                .highlight_code_block(&code_block_content, &code_block_language);
                            lines.extend(highlighted_lines);
                            lines.push(Line::from(""));
                            code_block_content.clear();
                            code_block_language.clear();
                        }
                        TagEnd::Strong => {
                            is_bold = false;
                        }
                        TagEnd::Emphasis => {
                            is_italic = false;
                        }
                        TagEnd::Item => {
                            in_list_item = false;
                            // Flush list item
                            if !current_line_spans.is_empty() {
                                lines.push(Line::from(current_line_spans.clone()));
                                current_line_spans.clear();
                            }
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if in_code_block {
                        // Collect code block lines
                        code_block_content.push(text.to_string());
                    } else {
                        // Apply current formatting
                        let mut style = Style::default();

                        if in_heading {
                            match heading_level {
                                1 => style = style.fg(Color::Cyan),
                                2 => style = style.fg(Color::Blue),
                                3 => style = style.fg(Color::Green),
                                _ => style = style.fg(Color::White),
                            }
                        }

                        if is_bold {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        if is_italic {
                            style = style.add_modifier(Modifier::ITALIC);
                        }
                        if in_list_item {
                            style = style.fg(Color::Yellow);
                        }

                        current_line_spans.push(Span::styled(text.to_string(), style));
                    }
                }
                Event::Code(text) => {
                    // Inline code - use different style
                    let style = Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::ITALIC);
                    current_line_spans.push(Span::styled(text.to_string(), style));
                }
                Event::Rule => {
                    // Horizontal rule
                    if !current_line_spans.is_empty() {
                        lines.push(Line::from(current_line_spans.clone()));
                        current_line_spans.clear();
                    }
                    lines.push(Line::from(Span::styled(
                        "──────────────────────────────────────",
                        Style::default().fg(Color::DarkGray),
                    )));
                    lines.push(Line::from(""));
                }
                Event::SoftBreak => {
                    // Soft break - just add space
                    current_line_spans.push(Span::raw(" "));
                }
                Event::HardBreak => {
                    // Hard break - flush current line
                    if !current_line_spans.is_empty() {
                        lines.push(Line::from(current_line_spans.clone()));
                        current_line_spans.clear();
                    }
                }
                _ => {
                    // Ignore other events (HTML, footnotes, etc.)
                }
            }
        }

        // Flush any remaining content
        if !current_line_spans.is_empty() {
            lines.push(Line::from(current_line_spans));
        }

        Text::from(lines)
    }

    /// Highlight a code block using syntect
    ///
    /// # Arguments
    /// * `lines` - The lines of code to highlight
    /// * `language` - The programming language hint from the code fence
    ///
    /// # Returns
    /// A vector of ratatui `Line`s with syntax highlighting applied
    fn highlight_code_block(&self, lines: &[String], language: &str) -> Vec<Line<'static>> {
        // Try to find syntax for the language
        let syntax = if language.is_empty() {
            self.syntax_set.find_syntax_plain_text()
        } else {
            // Try different methods to find the syntax
            self.syntax_set
                .find_syntax_by_token(language)
                .or_else(|| self.syntax_set.find_syntax_by_extension(language))
                .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
        };

        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        let mut highlighted_lines: Vec<Line<'static>> = Vec::new();

        for line in lines {
            // Skip empty lines
            if line.trim().is_empty() {
                highlighted_lines.push(Line::from(Span::raw("")));
                continue;
            }

            // Get highlighted regions for this line
            let regions = highlighter
                .highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            // Convert syntect regions to ratatui spans
            let spans: Vec<Span<'static>> = regions
                .iter()
                .map(|(style, text)| {
                    let color = Self::syntect_style_to_color(style);
                    Span::styled(text.to_string(), Style::default().fg(color))
                })
                .collect();

            highlighted_lines.push(Line::from(spans));
        }

        highlighted_lines
    }

    /// Convert syntect style to ratatui Color
    ///
    /// This extracts RGB values from syntect's color for precise matching.
    fn syntect_style_to_color(style: &syntect::highlighting::Style) -> Color {
        // Extract RGB values from syntect color
        let r = style.foreground.r;
        let g = style.foreground.g;
        let b = style.foreground.b;

        // Use RGB color for precise matching
        Color::Rgb(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plain_text() {
        let renderer = TranscriptRenderer::new();
        let markdown = "Hello, World!";
        let result = renderer.render(markdown);

        assert!(!result.lines.is_empty());
        // The text should contain our message
        let text_content: String = result
            .lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text_content.contains("Hello, World!"));
    }

    #[test]
    fn test_render_headers() {
        let renderer = TranscriptRenderer::new();
        let markdown = "# Main Header\n## Sub Header\n### Small Header";
        let result = renderer.render(markdown);

        assert!(result.lines.len() >= 3);
        // Check that we have content
        let text_content: String = result
            .lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text_content.contains("Main Header"));
        assert!(text_content.contains("Sub Header"));
        assert!(text_content.contains("Small Header"));
    }

    #[test]
    fn test_render_code_blocks() {
        let renderer = TranscriptRenderer::new();
        let markdown = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let result = renderer.render(markdown);

        assert!(!result.lines.is_empty());
        // Code block should have at least 1 line (the code content)
        assert!(result.lines.len() >= 1);

        let text_content: String = result
            .lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text_content.contains("fn main()"));
    }

    #[test]
    fn test_render_bold_and_emphasis() {
        let renderer = TranscriptRenderer::new();
        let markdown = "This is **bold** and this is *italic*";
        let result = renderer.render(markdown);

        assert!(!result.lines.is_empty());

        let text_content: String = result
            .lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text_content.contains("bold"));
        assert!(text_content.contains("italic"));
    }

    #[test]
    fn test_render_list() {
        let renderer = TranscriptRenderer::new();
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let result = renderer.render(markdown);

        assert!(result.lines.len() >= 3);

        let text_content: String = result
            .lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|s| s.content.as_ref()))
            .collect();
        assert!(text_content.contains("Item 1"));
        assert!(text_content.contains("Item 2"));
        assert!(text_content.contains("Item 3"));
    }

    #[test]
    fn test_renderer_default() {
        let renderer = TranscriptRenderer::default();
        let markdown = "# Test";
        let result = renderer.render(markdown);

        assert!(!result.lines.is_empty());
    }
}
