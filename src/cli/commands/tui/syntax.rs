use once_cell::sync::Lazy;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::text::Span;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style, ThemeSet};
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub struct SyntaxHighlighter {
    theme_name: String,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    pub fn with_theme(theme_name: &str) -> Self {
        Self {
            theme_name: theme_name.to_string(),
        }
    }

    pub fn highlight<'a>(&self, code: &'a str, language: &str) -> Vec<Span<'a>> {
        let syntax = self.detect_syntax(language);
        let theme = THEME_SET
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| THEME_SET.themes.values().next().unwrap());
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut spans = Vec::new();

        for line in code.lines() {
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        spans.push(style_to_span(style, text));
                    }
                }
                Err(_) => {
                    spans.push(Span::raw(line));
                }
            }
        }

        spans
    }

    fn detect_syntax(&self, language: &str) -> &'static syntect::parsing::SyntaxReference {
        let lang_lower = language.to_lowercase();

        let extension = match lang_lower.as_str() {
            "rust" => "rs",
            "typescript" | "ts" => "ts",
            "javascript" | "js" => "js",
            "python" | "py" => "py",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "markdown" | "md" => "md",
            "shell" | "bash" | "sh" => "sh",
            "c" => "c",
            "cpp" | "c++" => "cpp",
            "go" => "go",
            "java" => "java",
            "kotlin" => "kt",
            "swift" => "swift",
            "ruby" | "rb" => "rb",
            "php" => "php",
            "html" => "html",
            "css" => "css",
            "sql" => "sql",
            "xml" => "xml",
            _ => &lang_lower,
        };

        SYNTAX_SET
            .find_syntax_by_extension(extension)
            .or_else(|| SYNTAX_SET.find_syntax_by_token(language))
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text())
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn style_to_span<'a>(style: Style, text: &'a str) -> Span<'a> {
    let color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
    let mut modifiers = Modifier::empty();

    if style.font_style.contains(FontStyle::BOLD) {
        modifiers |= Modifier::BOLD;
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        modifiers |= Modifier::ITALIC;
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        modifiers |= Modifier::UNDERLINED;
    }

    Span::styled(
        text,
        ratatui::style::Style::default()
            .fg(color)
            .add_modifier(modifiers),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syntax_highlighter_new() {
        let _highlighter = SyntaxHighlighter::new();
    }

    #[test]
    fn test_highlight_rust_code() {
        let highlighter = SyntaxHighlighter::new();
        let code = "fn main() { let x = 42; }";
        let spans = highlighter.highlight(code, "rust");
        assert!(!spans.is_empty());

        let total_text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(total_text.contains("fn"));
        assert!(total_text.contains("main"));
        assert!(total_text.contains("42"));
    }

    #[test]
    fn test_detect_syntax_by_extension() {
        let highlighter = SyntaxHighlighter::new();
        let code = "let x = 1;";
        let spans = highlighter.highlight(code, "rs");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_fallback_on_unknown_language() {
        let highlighter = SyntaxHighlighter::new();
        let code = "some code";
        let spans = highlighter.highlight(code, "unknown_language_xyz");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_python() {
        let highlighter = SyntaxHighlighter::new();
        let code = "def hello():\n    print('world')";
        let spans = highlighter.highlight(code, "python");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_javascript() {
        let highlighter = SyntaxHighlighter::new();
        let code = "const x = () => { return 1; };";
        let spans = highlighter.highlight(code, "js");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_json() {
        let highlighter = SyntaxHighlighter::new();
        let code = r#"{"key": "value", "number": 123}"#;
        let spans = highlighter.highlight(code, "json");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_syntax_set_is_shared() {
        let set1 = &*SYNTAX_SET;
        let set2 = &*SYNTAX_SET;
        assert!(std::ptr::eq(set1, set2));
    }

    #[test]
    fn test_multiline_code() {
        let highlighter = SyntaxHighlighter::new();
        let code = r#"fn main() {
    let x = 1;
    let y = 2;
    println!("{}", x + y);
}"#;
        let spans = highlighter.highlight(code, "rust");
        assert!(!spans.is_empty());
    }
}
