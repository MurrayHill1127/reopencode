use lazy_static::lazy_static;
use regex::Regex;
use super::types::CommandParseResult;

const SLASH_COMMAND_PATTERN: &str = r"^/([a-zA-Z][a-zA-Z0-9_-]*)(?:\s+(.*))?$";

lazy_static! {
    static ref SLASH_PATTERN: Regex = Regex::new(SLASH_COMMAND_PATTERN).unwrap();
}

fn looks_like_command(text: &str, start: usize) -> Option<(String, usize)> {
    if start >= text.len() || text.as_bytes()[start] != b'/' {
        return None;
    }
    let rest = &text[start + 1..];
    let name_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .unwrap_or(rest.len());
    if name_end == 0 {
        return None;
    }
    let name = &rest[..name_end];
    if !name.chars().next().unwrap().is_alphabetic() {
        return None;
    }
    Some((name.to_string(), start + 1 + name_end))
}

pub fn parse_slash_command(input: &str) -> Option<CommandParseResult> {
    let trimmed = input.trim();
    SLASH_PATTERN.captures(trimmed).map(|caps| {
        let match_str = caps.get(0).unwrap();
        let full_match = match_str.as_str().to_string();
        let command_name = caps.get(1).unwrap().as_str().to_string();
        let arguments = caps
            .get(2)
            .map(|m| m.as_str().trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let start = match_str.start();
        let end = match_str.end();

        CommandParseResult::new(full_match, command_name, start, end).with_arguments_optional(arguments)
    })
}

pub fn find_commands_in_text(text: &str) -> Vec<CommandParseResult> {
    let mut results = Vec::new();
    let mut i = 0;
    
    while i < text.len() {
        if text.as_bytes()[i] == b'/' {
            let at_line_start = i == 0 || text.as_bytes()[i - 1].is_ascii_whitespace();
            if at_line_start {
                if let Some((name, name_end)) = looks_like_command(text, i) {
                    let start = i;
                    i = name_end;
                    
                    let arg_start = i;
                    let mut arg_end = i;
                    while i < text.len() {
                        if text.as_bytes()[i] == b'\n' {
                            break;
                        }
                        if text.as_bytes()[i] == b'/' {
                            if looks_like_command(text, i).is_some() {
                                break;
                            }
                        }
                        i += 1;
                        arg_end = i;
                    }
                    
                    let args = text[arg_start..arg_end].trim();
                    let args = if args.is_empty() { None } else { Some(args.to_string()) };
                    let full_match = text[start..arg_end].to_string();
                    
                    results.push(
                        CommandParseResult::new(full_match, name, start, arg_end)
                            .with_arguments_optional(args)
                    );
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    
    results
}

trait CommandParseResultExt {
    fn with_arguments_optional(self, args: Option<String>) -> Self;
}

impl CommandParseResultExt for CommandParseResult {
    fn with_arguments_optional(self, args: Option<String>) -> Self {
        match args {
            Some(a) => self.with_arguments(a),
            None => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let result = parse_slash_command("/init-deep").unwrap();
        assert_eq!(result.command_name, "init-deep");
        assert!(result.arguments.is_none());
        assert_eq!(result.full_match, "/init-deep");
    }

    #[test]
    fn test_parse_command_with_arguments() {
        let result = parse_slash_command("/init-deep --create-new").unwrap();
        assert_eq!(result.command_name, "init-deep");
        assert_eq!(result.arguments, Some("--create-new".to_string()));
    }

    #[test]
    fn test_parse_command_with_quoted_arguments() {
        let result = parse_slash_command(r#"/ralph-loop "fix the bug""#).unwrap();
        assert_eq!(result.command_name, "ralph-loop");
        assert_eq!(result.arguments, Some(r#""fix the bug""#.to_string()));
    }

    #[test]
    fn test_parse_command_with_trailing_whitespace() {
        let result = parse_slash_command("/init-deep   ").unwrap();
        assert_eq!(result.command_name, "init-deep");
        assert!(result.arguments.is_none());
    }

    #[test]
    fn test_parse_invalid_command_not_a_command() {
        assert!(parse_slash_command("not a command").is_none());
    }

    #[test]
    fn test_parse_invalid_command_just_slash() {
        assert!(parse_slash_command("/").is_none());
    }

    #[test]
    fn test_parse_invalid_command_starts_with_number() {
        assert!(parse_slash_command("/123invalid").is_none());
    }

    #[test]
    fn test_parse_invalid_command_with_special_chars() {
        assert!(parse_slash_command("/test@cmd").is_none());
    }

    #[test]
    fn test_parse_command_with_hyphens_and_underscores() {
        let result = parse_slash_command("/my-test_cmd").unwrap();
        assert_eq!(result.command_name, "my-test_cmd");
    }

    #[test]
    fn test_find_commands_in_text() {
        let text = "Let me run /init-deep first, then /refactor src/";
        let commands = find_commands_in_text(text);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command_name, "init-deep");
        assert_eq!(commands[0].arguments, Some("first, then".to_string()));
        assert_eq!(commands[1].command_name, "refactor");
        assert_eq!(commands[1].arguments, Some("src/".to_string()));
    }

    #[test]
    fn test_find_commands_at_line_start() {
        let text = "/init-deep\n/refactor";
        let commands = find_commands_in_text(text);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command_name, "init-deep");
        assert_eq!(commands[1].command_name, "refactor");
    }

    #[test]
    fn test_find_commands_mixed_content() {
        let text = "Hello /world and /test --arg";
        let commands = find_commands_in_text(text);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command_name, "world");
        assert_eq!(commands[1].command_name, "test");
        assert_eq!(commands[1].arguments, Some("--arg".to_string()));
    }

    #[test]
    fn test_find_commands_empty_text() {
        let commands = find_commands_in_text("");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_find_commands_no_commands() {
        let commands = find_commands_in_text("Just some regular text");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_find_commands_multiline() {
        let text = "Start here\n/init-deep\nMore text\n/refactor file";
        let commands = find_commands_in_text(text);
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0].command_name, "init-deep");
        assert_eq!(commands[1].command_name, "refactor");
        assert_eq!(commands[1].arguments, Some("file".to_string()));
    }

    #[test]
    fn test_parse_preserves_position() {
        let text = "  /init-deep  ";
        let result = parse_slash_command(text).unwrap();
        assert_eq!(result.start, 0);
        assert_eq!(result.end, 10);
    }
}
