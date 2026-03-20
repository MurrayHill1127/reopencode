//! Output truncation module.
//!
//! Provides utilities for truncating large outputs while preserving full content
//! in files for later reference. Used by tools that may produce large outputs.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::fs;

/// Maximum number of lines before truncation
pub const MAX_LINES: usize = 2000;

/// Maximum bytes before truncation (50KB)
pub const MAX_BYTES: usize = 50 * 1024;

/// Retention period in milliseconds (7 days)
const RETENTION_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Counter for generating ascending IDs
static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Truncation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Truncate from the head (keep tail)
    Head,
    /// Truncate from the tail (keep head)
    Tail,
}

impl Default for Direction {
    fn default() -> Self {
        Self::Head
    }
}

/// Options for truncation
#[derive(Debug, Clone)]
pub struct TruncateOptions {
    /// Maximum number of lines (default: MAX_LINES)
    pub max_lines: usize,
    /// Maximum bytes (default: MAX_BYTES)
    pub max_bytes: usize,
    /// Truncation direction (default: Head)
    pub direction: Direction,
}

impl Default for TruncateOptions {
    fn default() -> Self {
        Self {
            max_lines: MAX_LINES,
            max_bytes: MAX_BYTES,
            direction: Direction::default(),
        }
    }
}

/// Result of truncation
#[derive(Debug, Clone)]
pub enum TruncateResult {
    /// Content was not truncated
    NotTruncated { content: String },
    /// Content was truncated and saved to file
    Truncated {
        content: String,
        output_path: PathBuf,
    },
}

impl TruncateResult {
    /// Get the content (truncated or not)
    pub fn content(&self) -> &str {
        match self {
            Self::NotTruncated { content } => content,
            Self::Truncated { content, .. } => content,
        }
    }

    /// Check if content was truncated
    pub fn is_truncated(&self) -> bool {
        matches!(self, Self::Truncated { .. })
    }

    /// Get output path if truncated
    pub fn output_path(&self) -> Option<&PathBuf> {
        match self {
            Self::NotTruncated { .. } => None,
            Self::Truncated { output_path, .. } => Some(output_path),
        }
    }
}

/// Get the directory for storing truncated output files.
pub fn get_output_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("roc")
        .join("tool-output")
}

/// Generate an ascending ID for file naming.
fn generate_tool_id() -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("tool_{counter}")
}

/// Truncate output if it exceeds limits.
///
/// If the content exceeds the specified limits, it will be truncated
/// and the full content will be saved to a file.
///
/// # Arguments
///
/// * `text` - The text to potentially truncate
/// * `options` - Truncation options (uses defaults if None)
///
/// # Returns
///
/// A `TruncateResult` containing either the original content or truncated content
/// with the path to the full output file.
pub async fn truncate_output(text: &str, options: Option<TruncateOptions>) -> TruncateResult {
    let opts = options.unwrap_or_default();
    let lines: Vec<&str> = text.split('\n').collect();
    let total_bytes = text.len();

    // Check if truncation is needed
    if lines.len() <= opts.max_lines && total_bytes <= opts.max_bytes {
        return TruncateResult::NotTruncated {
            content: text.to_string(),
        };
    }

    // Perform truncation
    let mut out: Vec<String> = Vec::new();
    let mut bytes = 0;
    let mut hit_bytes = false;

    match opts.direction {
        Direction::Head => {
            // Keep head, truncate tail
            for line in lines.iter().take(opts.max_lines) {
                let size = line.len() + if !out.is_empty() { 1 } else { 0 };
                if bytes + size > opts.max_bytes {
                    hit_bytes = true;
                    break;
                }
                out.push((*line).to_string());
                bytes += size;
            }
        }
        Direction::Tail => {
            // Keep tail, truncate head
            for i in (0..lines.len()).rev().take(opts.max_lines) {
                let line = lines[i];
                let size = line.len() + if !out.is_empty() { 1 } else { 0 };
                if bytes + size > opts.max_bytes {
                    hit_bytes = true;
                    break;
                }
                out.insert(0, line.to_string());
                bytes += size;
            }
        }
    }

    let removed = if hit_bytes {
        total_bytes - bytes
    } else {
        lines.len() - out.len()
    };
    let unit = if hit_bytes { "bytes" } else { "lines" };
    let preview = out.join("\n");

    // Generate ID and save full output
    let id = generate_tool_id();
    let output_dir = get_output_dir();
    let output_path = output_dir.join(&id);

    // Ensure directory exists and write file
    if let Err(e) = fs::create_dir_all(&output_dir).await {
        tracing::warn!("Failed to create output directory: {}", e);
    }

    if let Err(e) = fs::write(&output_path, text).await {
        tracing::warn!("Failed to write output file: {}", e);
    }

    // Build hint message
    let hint = format!(
        "The tool call succeeded but the output was truncated. Full output saved to: {}\n\
         Use Grep to search the full content or Read with offset/limit to view specific sections.",
        output_path.display()
    );

    // Build final message
    let message = match opts.direction {
        Direction::Head => format!("{}\n\n...{} {} truncated...\n\n{}", preview, removed, unit, hint),
        Direction::Tail => format!("...{} {} truncated...\n\n{}\n\n{}", removed, unit, hint, preview),
    };

    TruncateResult::Truncated {
        content: message,
        output_path,
    }
}

/// Clean up old output files.
///
/// Removes files older than the retention period (7 days).
pub async fn cleanup_old_files() -> std::io::Result<()> {
    let output_dir = get_output_dir();

    if !output_dir.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(&output_dir).await?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let cutoff = now - RETENTION_MS;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Check file modification time
        if let Ok(metadata) = entry.metadata().await {
            if let Ok(modified) = metadata.modified() {
                let modified_ms = modified
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                if modified_ms < cutoff {
                    if let Err(e) = fs::remove_file(&path).await {
                        tracing::debug!("Failed to remove old file {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direction_default() {
        assert_eq!(Direction::default(), Direction::Head);
    }

    #[test]
    fn test_truncate_options_default() {
        let opts = TruncateOptions::default();
        assert_eq!(opts.max_lines, MAX_LINES);
        assert_eq!(opts.max_bytes, MAX_BYTES);
        assert_eq!(opts.direction, Direction::Head);
    }

    #[tokio::test]
    async fn test_no_truncation_needed() {
        let text = "Hello, world!";
        let result = truncate_output(text, None).await;

        assert!(!result.is_truncated());
        assert_eq!(result.content(), text);
        assert!(result.output_path().is_none());
    }

    #[tokio::test]
    async fn test_truncation_by_lines() {
        let text = (0..3000).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        let result = truncate_output(&text, Some(TruncateOptions {
            max_lines: 100,
            max_bytes: MAX_BYTES,
            direction: Direction::Head,
        })).await;

        assert!(result.is_truncated());
        assert!(result.output_path().is_some());
        assert!(result.content().contains("truncated"));
    }

    #[tokio::test]
    async fn test_truncation_by_bytes() {
        let text = "x".repeat(100_000);
        let result = truncate_output(&text, Some(TruncateOptions {
            max_lines: MAX_LINES,
            max_bytes: 1000,
            direction: Direction::Head,
        })).await;

        assert!(result.is_truncated());
        assert!(result.output_path().is_some());
        assert!(result.content().contains("bytes truncated"));
    }

    #[tokio::test]
    async fn test_truncation_direction_tail() {
        let text = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        let result = truncate_output(&text, Some(TruncateOptions {
            max_lines: 10,
            max_bytes: MAX_BYTES,
            direction: Direction::Tail,
        })).await;

        assert!(result.is_truncated());
        let content = result.content();
        // Tail truncation should show the end of the content at the bottom
        assert!(content.contains("Line 99"));
    }

    #[tokio::test]
    async fn test_truncate_result_helpers() {
        let not_truncated = TruncateResult::NotTruncated {
            content: "test".to_string(),
        };
        assert_eq!(not_truncated.content(), "test");
        assert!(!not_truncated.is_truncated());
        assert!(not_truncated.output_path().is_none());

        let truncated = TruncateResult::Truncated {
            content: "truncated".to_string(),
            output_path: PathBuf::from("/tmp/test"),
        };
        assert_eq!(truncated.content(), "truncated");
        assert!(truncated.is_truncated());
        assert_eq!(truncated.output_path(), Some(&PathBuf::from("/tmp/test")));
    }

    #[test]
    fn test_generate_tool_id() {
        let id1 = generate_tool_id();
        let id2 = generate_tool_id();

        assert!(id1.starts_with("tool_"));
        assert!(id2.starts_with("tool_"));
        // IDs should be different
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_get_output_dir() {
        let dir = get_output_dir();
        assert!(dir.ends_with("tool-output"));
    }
}