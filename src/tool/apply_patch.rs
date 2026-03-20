//! ApplyPatch tool - apply custom patch format to modify files
//!
//! This tool implements a custom patch format designed for LLM output.
//! The format uses Begin/End markers with file operations:
//! - Add File: Create new file
//! - Update File: Modify existing file (with optional move)
//! - Delete File: Remove file

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::error::{Result, ToolError};
use crate::tool::traits::{Tool, ToolResult};

/// Patch operation types
#[derive(Debug, Clone, PartialEq)]
enum PatchOperation {
    /// Add a new file with the given content
    Add {
        path: String,
        contents: String,
    },
    /// Delete an existing file
    Delete {
        path: String,
    },
    /// Update an existing file (optionally move/rename)
    Update {
        path: String,
        move_path: Option<String>,
        chunks: Vec<UpdateChunk>,
    },
}

/// A chunk of changes in an update operation
#[derive(Debug, Clone, PartialEq)]
struct UpdateChunk {
    /// Context line (after @@ marker)
    context: Option<String>,
    /// Lines to be removed (prefixed with -)
    old_lines: Vec<String>,
    /// Lines to be added (prefixed with +)
    new_lines: Vec<String>,
    /// Whether this chunk is at end of file
    is_end_of_file: bool,
}

/// ApplyPatch tool - apply custom patch format to modify files
pub struct ApplyPatch;

impl ApplyPatch {
    /// Create a new ApplyPatch instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApplyPatch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ApplyPatch {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Use the `apply_patch` tool to edit files. Custom patch format:
*** Begin Patch
*** Add File: <path>
+content line
*** Update File: <path>
*** Move to: <newpath>  (optional)
@@ <context>
-removed line
+added line
 context line
*** Delete File: <path>
*** End Patch"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patchText": {
                    "type": "string",
                    "description": "The full patch text that describes all changes to be made"
                }
            },
            "required": ["patchText"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let patch_text = args["patchText"]
            .as_str()
            .ok_or_else(|| ToolError::Parse("Missing 'patchText' argument".to_string()))?;

        if patch_text.trim().is_empty() {
            return Err(ToolError::Parse("patchText is required".to_string()));
        }

        // Parse the patch
        let operations = parse_patch(patch_text)?;

        if operations.is_empty() {
            return Err(ToolError::Parse("patch rejected: empty patch".to_string()));
        }

        let mut modified_files = Vec::new();
        let operations_count = operations.len();

        // Apply each operation
        for op in operations {
            match op {
                PatchOperation::Add { path, contents } => {
                    apply_add_file(&path, &contents).await?;
                    modified_files.push(path.clone());
                }
                PatchOperation::Delete { path } => {
                    apply_delete_file(&path).await?;
                    modified_files.push(path.clone());
                }
                PatchOperation::Update {
                    path,
                    move_path,
                    chunks,
                } => {
                    apply_update_file(&path, move_path.as_deref(), &chunks).await?;
                    let final_path = move_path.unwrap_or(path);
                    modified_files.push(final_path);
                }
            }
        }

        Ok(ToolResult::new(format!(
            "Success. Applied {} operations to {} files.",
            operations_count,
            modified_files.len()
        ))
        .with_metadata(serde_json::json!({
            "operations": operations_count,
            "filesModified": modified_files
        })))
    }
}

// ============================================================================
// Parser Implementation
// ============================================================================

/// Parse the patch text into operations
fn parse_patch(patch_text: &str) -> Result<Vec<PatchOperation>> {
    let cleaned = strip_heredoc(patch_text.trim());
    let lines: Vec<&str> = cleaned.lines().collect();

    // Find Begin/End markers
    let begin_idx = lines.iter().position(|l| l.trim() == "*** Begin Patch");
    let end_idx = lines.iter().position(|l| l.trim() == "*** End Patch");

    let (start, end) = match (begin_idx, end_idx) {
        (Some(b), Some(e)) if b < e => (b + 1, e),
        _ => return Err(ToolError::Parse("Invalid patch format: missing Begin/End markers".to_string())),
    };

    let mut operations = Vec::new();
    let mut i = start;

    while i < end {
        let line = lines[i];

        if line.starts_with("*** Add File:") {
            let path = line["*** Add File:".len()..].trim().to_string();
            if path.is_empty() {
                return Err(ToolError::Parse("Add File: empty path".to_string()));
            }
            let (contents, next_idx) = parse_add_content(&lines, i + 1, end);
            operations.push(PatchOperation::Add { path, contents });
            i = next_idx;
        } else if line.starts_with("*** Delete File:") {
            let path = line["*** Delete File:".len()..].trim().to_string();
            if path.is_empty() {
                return Err(ToolError::Parse("Delete File: empty path".to_string()));
            }
            operations.push(PatchOperation::Delete { path });
            i += 1;
        } else if line.starts_with("*** Update File:") {
            let path = line["*** Update File:".len()..].trim().to_string();
            if path.is_empty() {
                return Err(ToolError::Parse("Update File: empty path".to_string()));
            }

            // Check for move directive
            let mut move_path = None;
            let mut next_idx = i + 1;

            if next_idx < end && lines[next_idx].starts_with("*** Move to:") {
                move_path = Some(lines[next_idx]["*** Move to:".len()..].trim().to_string());
                next_idx += 1;
            }

            let (chunks, next_idx) = parse_update_chunks(&lines, next_idx, end);
            operations.push(PatchOperation::Update {
                path,
                move_path,
                chunks,
            });
            i = next_idx;
        } else {
            i += 1;
        }
    }

    Ok(operations)
}

/// Strip heredoc wrapper if present
fn strip_heredoc(input: &str) -> String {
    // Match heredoc patterns like: cat <<'EOF'\n...\nEOF or <<EOF\n...\nEOF
    // Simple implementation without regex
    let trimmed = input.trim();

    // Check for heredoc pattern: <<WORD or <<'WORD' or <<"WORD"
    if !trimmed.starts_with("<<") {
        return input.to_string();
    }

    // Extract the delimiter
    let rest = &trimmed[2..];
    let (delimiter, content_start) = if rest.starts_with('\'') {
        // <<'WORD' format
        if let Some(end) = rest.find('\'') {
            if end > 1 {
                (&rest[1..end], end + 1)
            } else {
                return input.to_string();
            }
        } else {
            return input.to_string();
        }
    } else if rest.starts_with('"') {
        // <<"WORD" format
        if let Some(end) = rest.find('"') {
            if end > 1 {
                (&rest[1..end], end + 1)
            } else {
                return input.to_string();
            }
        } else {
            return input.to_string();
        }
    } else {
        // <<WORD format - find end of word (newline or space)
        let end = rest.find(|c: char| c == '\n' || c == ' ').unwrap_or(rest.len());
        if end == 0 {
            return input.to_string();
        }
        (&rest[..end], end)
    };

    // Find the content between the start and the closing delimiter
    let after_header = &rest[content_start..];
    let newline_pos = after_header.find('\n').unwrap_or(0);
    let content = &after_header[newline_pos + 1..];

    // Find the closing delimiter on its own line
    let closing = format!("\n{}", delimiter);
    if let Some(end_pos) = content.find(&closing) {
        content[..end_pos].to_string()
    } else {
        // Try without leading newline
        if content.starts_with(delimiter) {
            String::new()
        } else {
            input.to_string()
        }
    }
}

/// Parse content lines for Add File operation
fn parse_add_content(lines: &[&str], start_idx: usize, end_idx: usize) -> (String, usize) {
    let mut content = String::new();
    let mut i = start_idx;

    while i < end_idx && !lines[i].starts_with("***") {
        if lines[i].starts_with('+') {
            content.push_str(&lines[i][1..]);
            content.push('\n');
        }
        i += 1;
    }

    // Remove trailing newline if present
    if content.ends_with('\n') {
        content.pop();
    }

    (content, i)
}

/// Parse update chunks for Update File operation
fn parse_update_chunks(lines: &[&str], start_idx: usize, end_idx: usize) -> (Vec<UpdateChunk>, usize) {
    let mut chunks = Vec::new();
    let mut i = start_idx;

    while i < end_idx && !lines[i].starts_with("***") {
        if lines[i].starts_with("@@") {
            let context = if lines[i].len() > 2 {
                Some(lines[i][2..].trim().to_string())
            } else {
                None
            };
            i += 1;

            let mut old_lines = Vec::new();
            let mut new_lines = Vec::new();
            let mut is_end_of_file = false;

            while i < end_idx && !lines[i].starts_with("@@") && !lines[i].starts_with("***") {
                if lines[i] == "*** End of File" {
                    is_end_of_file = true;
                    i += 1;
                    break;
                }

                let line = lines[i];
                if line.starts_with(' ') {
                    // Context line (unchanged)
                    let content = &line[1..];
                    old_lines.push(content.to_string());
                    new_lines.push(content.to_string());
                } else if line.starts_with('-') {
                    // Removed line
                    old_lines.push(line[1..].to_string());
                } else if line.starts_with('+') {
                    // Added line
                    new_lines.push(line[1..].to_string());
                }
                i += 1;
            }

            chunks.push(UpdateChunk {
                context,
                old_lines,
                new_lines,
                is_end_of_file,
            });
        } else {
            i += 1;
        }
    }

    (chunks, i)
}

// ============================================================================
// Fuzzy Matching
// ============================================================================

/// Normalize Unicode punctuation to ASCII equivalents
fn normalize_unicode(s: &str) -> String {
    s.replace('\u{2018}', "'") // left single quotation mark
        .replace('\u{2019}', "'") // right single quotation mark
        .replace('\u{201A}', "'") // single low-9 quotation mark
        .replace('\u{201B}', "'") // single high-reversed-9 quotation mark
        .replace('\u{201C}', "\"") // left double quotation mark
        .replace('\u{201D}', "\"") // right double quotation mark
        .replace('\u{201E}', "\"") // double low-9 quotation mark
        .replace('\u{201F}', "\"") // double high-reversed-9 quotation mark
        .replace('\u{2010}', "-") // hyphen
        .replace('\u{2011}', "-") // non-breaking hyphen
        .replace('\u{2012}', "-") // figure dash
        .replace('\u{2013}', "-") // en dash
        .replace('\u{2014}', "-") // em dash
        .replace('\u{2015}', "-") // horizontal bar
        .replace('\u{2026}', "...") // horizontal ellipsis
        .replace('\u{00A0}', " ") // non-breaking space
}

/// Seek a pattern in lines using fuzzy matching
fn seek_sequence(lines: &[String], pattern: &[String], start_index: usize, is_eof: bool) -> Option<usize> {
    if pattern.is_empty() {
        return None;
    }

    // Pass 1: exact match
    if let Some(pos) = try_match(lines, pattern, start_index, |a, b| a == b, is_eof) {
        return Some(pos);
    }

    // Pass 2: trim end (rstrip)
    if let Some(pos) = try_match(lines, pattern, start_index, |a, b| a.trim_end() == b.trim_end(), is_eof) {
        return Some(pos);
    }

    // Pass 3: trim both ends
    if let Some(pos) = try_match(lines, pattern, start_index, |a, b| a.trim() == b.trim(), is_eof) {
        return Some(pos);
    }

    // Pass 4: normalized Unicode
    if let Some(pos) = try_match(
        lines,
        pattern,
        start_index,
        |a, b| normalize_unicode(a.trim()) == normalize_unicode(b.trim()),
        is_eof,
    ) {
        return Some(pos);
    }

    None
}

/// Try to match pattern in lines with a given comparator
fn try_match<F>(lines: &[String], pattern: &[String], start_index: usize, compare: F, is_eof: bool) -> Option<usize>
where
    F: Fn(&str, &str) -> bool,
{
    // If EOF anchor, try matching from end of file first
    if is_eof {
        let from_end = lines.len().saturating_sub(pattern.len());
        if from_end >= start_index {
            let matches = pattern.iter().enumerate().all(|(j, p)| {
                compare(&lines[from_end + j], p)
            });
            if matches {
                return Some(from_end);
            }
        }
    }

    // Forward search from start_index
    for i in start_index..=(lines.len().saturating_sub(pattern.len())) {
        let matches = pattern.iter().enumerate().all(|(j, p)| {
            compare(&lines[i + j], p)
        });
        if matches {
            return Some(i);
        }
    }

    None
}

// ============================================================================
// File Operations
// ============================================================================

/// Apply Add File operation
async fn apply_add_file(path: &str, contents: &str) -> Result<()> {
    // Create parent directories
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    // Write file with trailing newline
    let content = if contents.is_empty() || contents.ends_with('\n') {
        contents.to_string()
    } else {
        format!("{}\n", contents)
    };

    tokio::fs::write(path, content).await?;
    Ok(())
}

/// Apply Delete File operation
async fn apply_delete_file(path: &str) -> Result<()> {
    tokio::fs::remove_file(path).await?;
    Ok(())
}

/// Apply Update File operation
async fn apply_update_file(path: &str, move_path: Option<&str>, chunks: &[UpdateChunk]) -> Result<()> {
    // Read original file
    let original_content = tokio::fs::read_to_string(path).await?;

    // Compute new content
    let new_content = apply_chunks_to_content(&original_content, path, chunks)?;

    // Handle move or update
    if let Some(new_path) = move_path {
        // Create parent directories for new path
        if let Some(parent) = std::path::Path::new(new_path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::write(new_path, new_content).await?;
        tokio::fs::remove_file(path).await?;
    } else {
        tokio::fs::write(path, new_content).await?;
    }

    Ok(())
}

/// Apply chunks to file content
fn apply_chunks_to_content(original: &str, path: &str, chunks: &[UpdateChunk]) -> Result<String> {
    let mut lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();

    // Compute all replacements
    let mut replacements: Vec<(usize, usize, Vec<String>)> = Vec::new();
    let mut line_index = 0;

    for chunk in chunks {
        // Handle context-based seeking (only if context is non-empty)
        if let Some(ref context) = chunk.context {
            if !context.is_empty() {
                let context_pattern = vec![context.clone()];
                if let Some(context_idx) = seek_sequence(&lines, &context_pattern, line_index, false) {
                    line_index = context_idx + 1;
                } else {
                    return Err(ToolError::Execution(format!(
                        "Failed to find context '{}' in {}",
                        context, path
                    )));
                }
            }
        }

        // Handle pure addition (no old lines)
        if chunk.old_lines.is_empty() {
            let insertion_idx = if lines.is_empty() {
                0
            } else {
                lines.len()
            };
            replacements.push((insertion_idx, 0, chunk.new_lines.clone()));
            continue;
        }

        // Try to match old lines in the file
        let mut pattern = chunk.old_lines.clone();
        let mut new_slice = chunk.new_lines.clone();
        let mut found = seek_sequence(&lines, &pattern, line_index, chunk.is_end_of_file);

        // Retry without trailing empty line if not found
        if found.is_none() && !pattern.is_empty() && pattern.last().map(|l| l.is_empty()) == Some(true) {
            pattern.pop();
            if !new_slice.is_empty() && new_slice.last().map(|l| l.is_empty()) == Some(true) {
                new_slice.pop();
            }
            found = seek_sequence(&lines, &pattern, line_index, chunk.is_end_of_file);
        }

        if let Some(pos) = found {
            replacements.push((pos, pattern.len(), new_slice));
            line_index = pos + pattern.len();
        } else {
            return Err(ToolError::Execution(format!(
                "Failed to find expected lines in {}:\n{}",
                path,
                chunk.old_lines.join("\n")
            )));
        }
    }

    // Sort replacements by index
    replacements.sort_by_key(|r| r.0);

    // Apply replacements in reverse order to avoid index shifting
    for (start_idx, old_len, new_segment) in replacements.into_iter().rev() {
        lines.splice(start_idx..start_idx + old_len, new_segment);
    }

    // Ensure trailing newline
    let mut result = lines.join("\n");
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    // ========================================================================
    // Parser Tests
    // ========================================================================

    #[test]
    fn test_parse_add_file() {
        let patch = r#"*** Begin Patch
*** Add File: test.txt
+Hello World
+Second Line
*** End Patch"#;

        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);

        match &ops[0] {
            PatchOperation::Add { path, contents } => {
                assert_eq!(path, "test.txt");
                assert_eq!(contents, "Hello World\nSecond Line");
            }
            _ => panic!("Expected Add operation"),
        }
    }

    #[test]
    fn test_parse_delete_file() {
        let patch = r#"*** Begin Patch
*** Delete File: old_file.txt
*** End Patch"#;

        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);

        match &ops[0] {
            PatchOperation::Delete { path } => {
                assert_eq!(path, "old_file.txt");
            }
            _ => panic!("Expected Delete operation"),
        }
    }

    #[test]
    fn test_parse_update_file() {
        let patch = r#"*** Begin Patch
*** Update File: src/app.py
@@ def greet():
-    print("Hi")
+    print("Hello, world!")
*** End Patch"#;

        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);

        match &ops[0] {
            PatchOperation::Update {
                path,
                move_path,
                chunks,
            } => {
                assert_eq!(path, "src/app.py");
                assert!(move_path.is_none());
                assert_eq!(chunks.len(), 1);
                assert_eq!(chunks[0].context, Some("def greet():".to_string()));
                assert_eq!(chunks[0].old_lines, vec!["    print(\"Hi\")"]);
                assert_eq!(chunks[0].new_lines, vec!["    print(\"Hello, world!\")"]);
            }
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_parse_move_file() {
        let patch = r#"*** Begin Patch
*** Update File: old/path.txt
*** Move to: new/path.txt
@@ context
-old content
+new content
*** End Patch"#;

        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 1);

        match &ops[0] {
            PatchOperation::Update {
                path,
                move_path,
                chunks,
            } => {
                assert_eq!(path, "old/path.txt");
                assert_eq!(move_path, &Some("new/path.txt".to_string()));
                assert_eq!(chunks.len(), 1);
            }
            _ => panic!("Expected Update operation"),
        }
    }

    #[test]
    fn test_parse_invalid_patch() {
        // Missing Begin marker
        let result = parse_patch("*** End Patch");
        assert!(result.is_err());

        // Missing End marker
        let result = parse_patch("*** Begin Patch\n*** Add File: test.txt\n+content");
        assert!(result.is_err());

        // Empty patch
        let result = parse_patch("*** Begin Patch\n*** End Patch");
        assert!(result.is_ok());
        let ops = result.unwrap();
        assert!(ops.is_empty());
    }

    #[test]
    fn test_parse_multiple_operations() {
        let patch = r#"*** Begin Patch
*** Add File: new.txt
+new content
*** Update File: existing.txt
@@ ctx
-old
+new
*** Delete File: obsolete.txt
*** End Patch"#;

        let ops = parse_patch(patch).unwrap();
        assert_eq!(ops.len(), 3);

        assert!(matches!(ops[0], PatchOperation::Add { .. }));
        assert!(matches!(ops[1], PatchOperation::Update { .. }));
        assert!(matches!(ops[2], PatchOperation::Delete { .. }));
    }

    // ========================================================================
    // Fuzzy Matching Tests
    // ========================================================================

    #[test]
    fn test_seek_sequence_exact_match() {
        let lines = vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "target".to_string(),
            "line 4".to_string(),
        ];
        let pattern = vec!["target".to_string()];

        let pos = seek_sequence(&lines, &pattern, 0, false);
        assert_eq!(pos, Some(2));
    }

    #[test]
    fn test_seek_sequence_trim_match() {
        let lines = vec![
            "line 1".to_string(),
            "target   ".to_string(), // trailing whitespace
            "line 3".to_string(),
        ];
        let pattern = vec!["target".to_string()];

        let pos = seek_sequence(&lines, &pattern, 0, false);
        assert_eq!(pos, Some(1));
    }

    #[test]
    fn test_seek_sequence_unicode_match() {
        let lines = vec![
            "line 1".to_string(),
            "it's a test".to_string(), // ASCII apostrophe
            "line 3".to_string(),
        ];
        let pattern = vec!["it's a test".to_string()]; // curly apostrophe

        let pos = seek_sequence(&lines, &pattern, 0, false);
        // This should match via Unicode normalization
        assert_eq!(pos, Some(1));
    }

    #[test]
    fn test_seek_sequence_multiline() {
        let lines = vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
            "line 4".to_string(),
        ];
        let pattern = vec!["line 2".to_string(), "line 3".to_string()];

        let pos = seek_sequence(&lines, &pattern, 0, false);
        assert_eq!(pos, Some(1));
    }

    #[test]
    fn test_seek_sequence_eof() {
        let lines = vec![
            "line 1".to_string(),
            "line 2".to_string(),
            "last line".to_string(),
        ];
        let pattern = vec!["last line".to_string()];

        let pos = seek_sequence(&lines, &pattern, 0, true);
        // Should find at EOF position
        assert_eq!(pos, Some(2));
    }

    #[test]
    fn test_normalize_unicode() {
        // Test curly quotes
        assert_eq!(normalize_unicode("'quoted'"), "'quoted'");
        assert_eq!(normalize_unicode("\"quoted\""), "\"quoted\"");

        // Test dashes
        assert_eq!(normalize_unicode("a - b"), "a - b");

        // Test ellipsis
        assert_eq!(normalize_unicode("wait..."), "wait...");
    }

    // ========================================================================
    // Apply Operation Tests
    // ========================================================================

    #[tokio::test]
    async fn test_apply_add_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("new_file.txt");
        let path_str = path.to_str().unwrap();

        apply_add_file(path_str, "Hello\nWorld").await.unwrap();

        let content = tokio::fs::read_to_string(path_str).await.unwrap();
        assert_eq!(content, "Hello\nWorld\n");
    }

    #[tokio::test]
    async fn test_apply_add_file_with_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("sub/nested/file.txt");
        let path_str = path.to_str().unwrap();

        apply_add_file(path_str, "content").await.unwrap();

        assert!(path.exists());
        let content = tokio::fs::read_to_string(path_str).await.unwrap();
        assert_eq!(content, "content\n");
    }

    #[tokio::test]
    async fn test_apply_delete_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        apply_delete_file(path).await.unwrap();

        assert!(!std::path::Path::new(path).exists());
    }

    #[tokio::test]
    async fn test_apply_update_file_simple() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line1\nline2\nline3").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let chunks = vec![UpdateChunk {
            context: None,
            old_lines: vec!["line2".to_string()],
            new_lines: vec!["replaced".to_string()],
            is_end_of_file: false,
        }];

        apply_update_file(path, None, &chunks).await.unwrap();

        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "line1\nreplaced\nline3\n");
    }

    #[tokio::test]
    async fn test_apply_update_file_with_context() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "def greet():\n    print(\"Hi\")\n    return").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let chunks = vec![UpdateChunk {
            context: Some("def greet():".to_string()),
            old_lines: vec!["    print(\"Hi\")".to_string()],
            new_lines: vec!["    print(\"Hello\")".to_string()],
            is_end_of_file: false,
        }];

        apply_update_file(path, None, &chunks).await.unwrap();

        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert!(content.contains("print(\"Hello\")"));
    }

    #[tokio::test]
    async fn test_apply_update_file_with_move() {
        let temp_dir = TempDir::new().unwrap();
        let old_path = temp_dir.path().join("old.txt");
        let new_path = temp_dir.path().join("new.txt");

        tokio::fs::write(&old_path, "content\n").await.unwrap();

        let chunks = vec![];

        apply_update_file(old_path.to_str().unwrap(), Some(new_path.to_str().unwrap()), &chunks)
            .await
            .unwrap();

        assert!(!old_path.exists());
        assert!(new_path.exists());
    }

    // ========================================================================
    // Tool Integration Tests
    // ========================================================================

    #[tokio::test]
    async fn test_tool_execute_add_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");
        let path_str = path.to_str().unwrap();

        let tool = ApplyPatch::new();
        let patch = format!(
            r#"*** Begin Patch
*** Add File: {}
+Hello World
*** End Patch"#,
            path_str
        );

        let args = serde_json::json!({ "patchText": patch });
        let result = tool.execute(args).await.unwrap();

        assert!(result.output.contains("Success"));
        assert!(path.exists());

        let content = tokio::fs::read_to_string(path_str).await.unwrap();
        assert_eq!(content, "Hello World\n");
    }

    #[tokio::test]
    async fn test_tool_execute_delete_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "content").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = ApplyPatch::new();
        let patch = format!(
            r#"*** Begin Patch
*** Delete File: {}
*** End Patch"#,
            path
        );

        let args = serde_json::json!({ "patchText": patch });
        let result = tool.execute(args).await.unwrap();

        assert!(result.output.contains("Success"));
        assert!(!std::path::Path::new(path).exists());
    }

    #[tokio::test]
    async fn test_tool_execute_update_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line1\nline2\nline3").unwrap();
        let path = temp_file.path().to_str().unwrap();

        let tool = ApplyPatch::new();
        let patch = format!(
            r#"*** Begin Patch
*** Update File: {}
@@ 
-line2
+replaced
*** End Patch"#,
            path
        );

        let args = serde_json::json!({ "patchText": patch });
        let result = tool.execute(args).await.unwrap();

        assert!(result.output.contains("Success"));
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert_eq!(content, "line1\nreplaced\nline3\n");
    }

    #[tokio::test]
    async fn test_tool_execute_invalid_patch() {
        let tool = ApplyPatch::new();
        let args = serde_json::json!({ "patchText": "invalid patch" });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tool_execute_missing_patch_text() {
        let tool = ApplyPatch::new();
        let args = serde_json::json!({});

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'patchText'"));
    }

    #[tokio::test]
    async fn test_tool_execute_empty_patch() {
        let tool = ApplyPatch::new();
        let args = serde_json::json!({ "patchText": "*** Begin Patch\n*** End Patch" });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_metadata() {
        let tool = ApplyPatch::new();
        assert_eq!(tool.name(), "apply_patch");
        assert!(tool.description().contains("Begin Patch"));

        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["required"].as_array().unwrap().contains(&"patchText".into()));
    }

    #[tokio::test]
    async fn test_tool_result_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.txt");
        let path_str = path.to_str().unwrap();

        let tool = ApplyPatch::new();
        let patch = format!(
            r#"*** Begin Patch
*** Add File: {}
+content
*** End Patch"#,
            path_str
        );

        let args = serde_json::json!({ "patchText": patch });
        let result = tool.execute(args).await.unwrap();

        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["operations"], 1);
        let files = metadata["filesModified"].as_array().unwrap();
        assert_eq!(files.len(), 1);
    }
}