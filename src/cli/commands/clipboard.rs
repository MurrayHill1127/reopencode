//! Clipboard utilities for TUI applications
//!
//! OSC 52 escape sequences for SSH support, plus native Linux tools.

use std::env;
use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};

use base64::Engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardMethod {
    WlCopy,
    Xclip,
    Xsel,
}

pub fn is_wayland() -> bool {
    env::var("WAYLAND_DISPLAY").is_ok()
}

pub fn is_tmux() -> bool {
    env::var("TMUX").is_ok()
}

pub fn is_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Write OSC 52 escape sequence to /dev/tty.
/// Format: ESC ] 52 ; c ; <base64> BEL
/// TMUX passthrough: ESC Ptmux;ESC ... ESC \
fn write_osc52(text: &str) -> io::Result<()> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(text);
    let osc52 = format!("\x1b]52;c;{}\x07", encoded);
    let sequence = if is_tmux() {
        format!("\x1bPtmux;\x1b{}\x1b\\", osc52)
    } else {
        osc52
    };
    let mut tty = File::create("/dev/tty")?;
    tty.write_all(sequence.as_bytes())?;
    tty.flush()
}

pub fn detect_clipboard_tool() -> Option<ClipboardMethod> {
    if is_wayland() && Command::new("wl-copy").arg("--version").output().is_ok() {
        return Some(ClipboardMethod::WlCopy);
    }
    if Command::new("xclip").arg("-version").output().is_ok() {
        return Some(ClipboardMethod::Xclip);
    }
    if Command::new("xsel").arg("--version").output().is_ok() {
        return Some(ClipboardMethod::Xsel);
    }
    if Command::new("wl-copy").arg("--version").output().is_ok() {
        return Some(ClipboardMethod::WlCopy);
    }
    None
}

fn copy_with_wl_copy(text: &str) -> io::Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let _ = child.wait();
    Ok(())
}

fn copy_with_xclip(text: &str) -> io::Result<()> {
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let _ = child.wait();
    Ok(())
}

fn copy_with_xsel(text: &str) -> io::Result<()> {
    let mut child = Command::new("xsel")
        .args(["--clipboard", "--input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let _ = child.wait();
    Ok(())
}

fn copy_native(text: &str) -> io::Result<()> {
    match detect_clipboard_tool() {
        Some(ClipboardMethod::WlCopy) => copy_with_wl_copy(text),
        Some(ClipboardMethod::Xclip) => copy_with_xclip(text),
        Some(ClipboardMethod::Xsel) => copy_with_xsel(text),
        None => Err(io::Error::new(io::ErrorKind::NotFound, "No clipboard tool")),
    }
}

pub struct Clipboard;

impl Clipboard {
    /// Copy text to clipboard. Tries OSC 52 first, then native tools.
    /// Silently ignores errors to avoid disrupting TUI.
    pub fn copy(text: &str) -> io::Result<()> {
        let _ = write_osc52(text);
        let _ = copy_native(text);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc52_encoding() {
        let text = "Hello, World!";
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_osc52_sequence_format() {
        let text = "test";
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        let osc52 = format!("\x1b]52;c;{}\x07", encoded);
        assert!(osc52.starts_with("\x1b]52;c;"));
        assert!(osc52.ends_with('\x07'));
    }

    #[test]
    fn test_tmux_passthrough_format() {
        let text = "test";
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        let osc52 = format!("\x1b]52;c;{}\x07", encoded);
        let passthrough = format!("\x1bPtmux;\x1b{}\x1b\\", osc52);
        assert!(passthrough.starts_with("\x1bPtmux;\x1b\x1b]52;c;"));
        assert!(passthrough.ends_with("\x07\x1b\\"));
    }

    #[test]
    fn test_is_wayland() {
        let _ = is_wayland();
    }

    #[test]
    fn test_is_tmux() {
        let _ = is_tmux();
    }

    #[test]
    fn test_is_tty() {
        let _ = is_tty();
    }

    #[test]
    fn test_detect_clipboard_tool() {
        let _ = detect_clipboard_tool();
    }

    #[test]
    fn test_clipboard_copy_doesnt_panic() {
        let result = Clipboard::copy("test content");
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_string() {
        let result = Clipboard::copy("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_unicode_text() {
        let text = "Hello 世界 🌍";
        let encoded = base64::engine::general_purpose::STANDARD.encode(text);
        assert!(!encoded.is_empty());
    }

    #[test]
    fn test_multiline_text() {
        let text = "line1\nline2\nline3";
        let result = Clipboard::copy(text);
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_text() {
        let text = "x".repeat(10000);
        let result = Clipboard::copy(&text);
        assert!(result.is_ok());
    }
}
