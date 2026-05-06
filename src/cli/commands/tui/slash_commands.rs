//! Slash-command parser for the TUI input box.
//!
//! When the user submits text that starts with `/`, it is parsed here rather
//! than sent to the LLM. Unknown commands produce an `Unknown` variant so
//! the caller can show a helpful error.

/// A parsed slash command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    /// `/exit` | `/quit` | `/q` — quit the TUI.
    Exit,
    /// `/new [title]` — create a new session.
    New { title: Option<String> },
    /// `/clear` — clear the current conversation view.
    Clear,
    /// `/help` | `/h` | `/?` — show help text.
    Help,
    /// `/sessions` — toggle the session sidebar.
    Sessions,
    /// Unrecognised command name (preserved for error messaging).
    Unknown(String),
}

/// Parse a raw user input string into a [`SlashCommand`].
///
/// Returns `None` when the input does not start with `/` (treat as a normal
/// message). Returns `Some(SlashCommand::Unknown(_))` for unrecognised commands.
pub fn parse_slash(input: &str) -> Option<SlashCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    // Strip the leading `/` and split into command + optional argument.
    let rest = &trimmed[1..];
    let (cmd, arg) = match rest.find(char::is_whitespace) {
        Some(pos) => {
            let arg = rest[pos + 1..].trim();
            (&rest[..pos], if arg.is_empty() { None } else { Some(arg.to_string()) })
        }
        None => (rest, None),
    };

    Some(match cmd.to_lowercase().as_str() {
        "exit" | "quit" | "q" => SlashCommand::Exit,
        "new" => SlashCommand::New { title: arg },
        "clear" => SlashCommand::Clear,
        "help" | "h" | "?" => SlashCommand::Help,
        "sessions" | "session" | "s" => SlashCommand::Sessions,
        other => SlashCommand::Unknown(other.to_string()),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_slash_returns_none() {
        assert!(parse_slash("hello").is_none());
        assert!(parse_slash("").is_none());
    }

    #[test]
    fn exit_variants() {
        assert_eq!(parse_slash("/exit"), Some(SlashCommand::Exit));
        assert_eq!(parse_slash("/quit"), Some(SlashCommand::Exit));
        assert_eq!(parse_slash("/q"),    Some(SlashCommand::Exit));
    }

    #[test]
    fn new_with_and_without_title() {
        assert_eq!(parse_slash("/new"), Some(SlashCommand::New { title: None }));
        assert_eq!(
            parse_slash("/new My Project"),
            Some(SlashCommand::New { title: Some("My Project".into()) })
        );
    }

    #[test]
    fn clear_and_help() {
        assert_eq!(parse_slash("/clear"), Some(SlashCommand::Clear));
        assert_eq!(parse_slash("/help"),  Some(SlashCommand::Help));
        assert_eq!(parse_slash("/?"),     Some(SlashCommand::Help));
    }

    #[test]
    fn sessions_variants() {
        assert_eq!(parse_slash("/sessions"), Some(SlashCommand::Sessions));
        assert_eq!(parse_slash("/s"),        Some(SlashCommand::Sessions));
    }

    #[test]
    fn unknown_command() {
        assert_eq!(
            parse_slash("/foobar"),
            Some(SlashCommand::Unknown("foobar".into()))
        );
    }
}
