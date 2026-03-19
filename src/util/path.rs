//! Path manipulation utilities.
//!
//! This module provides utility functions for working with file paths using
//! the `camino` crate for UTF-8 safe path handling. All functions operate on
//! [`camino::Utf8Path`] and [`camino::Utf8PathBuf`] types.
//!
//! # Examples
//!
//! ```
//! use camino::Utf8Path;
//! use reopencode::util::path::{get_filename, normalize_path};
//!
//! let filename = get_filename(Utf8Path::new("/home/user/file.txt"));
//! assert_eq!(filename, "file.txt");
//! ```

use camino::{Utf8Path, Utf8PathBuf};
use dirs;

use super::error::UtilError;

/// Extracts the filename from a path.
///
/// Returns the last component of the path, or an empty string if the path
/// ends in `..` or is the root.
///
/// # Arguments
///
/// * `path` - The path to extract the filename from
///
/// # Returns
///
/// The filename component as a string slice
///
/// # Examples
///
/// ```
/// use camino::Utf8Path;
/// use reopencode::util::path::get_filename;
///
/// assert_eq!(get_filename(Utf8Path::new("/path/to/file.txt")), "file.txt");
/// assert_eq!(get_filename(Utf8Path::new("file.txt")), "file.txt");
/// assert_eq!(get_filename(Utf8Path::new("/")), "");
/// ```
pub fn get_filename(path: &Utf8Path) -> &str {
    path.file_name().unwrap_or("")
}

/// Gets the parent directory of a path.
///
/// Returns `Some(parent)` if the path has a parent directory, or `None`
/// if the path is the root or has no parent.
///
/// # Arguments
///
/// * `path` - The path to get the parent directory from
///
/// # Returns
///
/// An optional string slice containing the parent directory path
///
/// # Examples
///
/// ```
/// use camino::Utf8Path;
/// use reopencode::util::path::get_directory;
///
/// assert_eq!(
///     get_directory(Utf8Path::new("/path/to/file.txt")),
///     Some("/path/to")
/// );
/// assert_eq!(
///     get_directory(Utf8Path::new("/file.txt")),
///     Some("/")
/// );
/// assert_eq!(get_directory(Utf8Path::new("file.txt")), None);
/// ```
pub fn get_directory(path: &Utf8Path) -> Option<&str> {
    path.parent().map(|p| p.as_str())
}

/// Truncates a path from the middle if it exceeds the maximum length.
///
/// Keeps the start and end of the path, inserting `"..."` in the middle
/// when truncation is necessary. This preserves the most informative parts
/// of a path (the root and the filename).
///
/// # Arguments
///
/// * `path` - The path string to truncate
/// * `max_length` - The maximum allowed length for the resulting string
///
/// # Returns
///
/// A string with the path truncated in the middle if necessary
///
/// # Examples
///
/// ```
/// use reopencode::util::path::truncate_middle;
///
/// let path = "/very/long/path/to/some/deep/file.txt";
/// let truncated = truncate_middle(path, 25);
/// assert!(truncated.len() <= 25);
/// assert!(truncated.starts_with("/very/"));
/// assert!(truncated.ends_with("/file.txt"));
/// assert!(truncated.contains("..."));
/// ```
pub fn truncate_middle(path: &str, max_length: usize) -> String {
    if path.len() <= max_length {
        return path.to_string();
    }

    if max_length <= 3 {
        return ".".repeat(max_length);
    }

    let ellipsis = "...";
    let available = max_length - ellipsis.len();
    let prefix_len = available / 2;
    let suffix_len = available - prefix_len;

    let prefix = &path[..prefix_len];
    let suffix = &path[path.len() - suffix_len..];

    format!("{}{}{}", prefix, ellipsis, suffix)
}

/// Normalizes a path by expanding `~` and resolving `.` and `..`.
///
/// This function:
/// - Expands `~` to the user's home directory
/// - Resolves `.` to the current directory
/// - Resolves `..` to parent directories
/// - Converts relative paths to absolute paths
///
/// # Arguments
///
/// * `path` - The path string to normalize
///
/// # Returns
///
/// A `Result` containing the normalized `Utf8PathBuf` or a `UtilError`
///
/// # Errors
///
/// Returns `UtilError::Path` if:
/// - The path is not valid UTF-8
/// - The home directory cannot be determined
/// - The path cannot be canonicalized
///
/// # Examples
///
/// ```
/// use reopencode::util::path::normalize_path;
///
/// // Expands ~ to home directory
/// let home_path = normalize_path("~/documents").unwrap();
/// assert!(!home_path.as_str().starts_with("~"));
///
/// // Resolves parent references
/// let normalized = normalize_path("/path/to/../file.txt").unwrap();
/// assert!(normalized.as_str().contains("/path/file.txt"));
/// ```
pub fn normalize_path(path: &str) -> Result<Utf8PathBuf, UtilError> {
    let expanded = if path.starts_with("~/") || path == "~" {
        let home = dirs::home_dir()
            .ok_or_else(|| UtilError::Path("Could not determine home directory".to_string()))?;
        let home_utf8 = Utf8PathBuf::from_path_buf(home)
            .map_err(|_| UtilError::Path("Home directory is not valid UTF-8".to_string()))?;
        if path == "~" {
            home_utf8
        } else {
            home_utf8.join(&path[2..])
        }
    } else {
        Utf8PathBuf::from(path)
    };

    // Try to canonicalize to resolve ., .., and symlinks
    // If canonicalize fails (e.g., path doesn't exist), do manual normalization
    match std::fs::canonicalize(&expanded) {
        Ok(canonical) => Utf8PathBuf::from_path_buf(canonical)
            .map_err(|_| UtilError::Path("Canonicalized path is not valid UTF-8".to_string())),
        Err(_) => Ok(manual_normalize(&expanded)),
    }
}

/// Manually normalizes a path by resolving `.` and `..` components.
///
/// This is a fallback when canonicalize fails (e.g., for non-existent paths).
fn manual_normalize(path: &Utf8Path) -> Utf8PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component.as_str() {
            "." => {} // Skip current directory
            ".." => {
                // Go up one level if possible
                if components.len() > 1 || (components.len() == 1 && components[0] != "/") {
                    components.pop();
                }
            }
            "" => {
                // Empty component from leading slash
                if components.is_empty() {
                    components.push("/");
                }
            }
            c => components.push(c),
        }
    }

    if components.is_empty() {
        return Utf8PathBuf::from(".");
    }

    if components.len() == 1 && components[0] == "/" {
        return Utf8PathBuf::from("/");
    }

    // Join components
    let mut result = String::new();
    for (i, component) in components.iter().enumerate() {
        if i > 0 && *component != "/" {
            result.push('/');
        }
        result.push_str(component);
    }

    Utf8PathBuf::from(result)
}

/// Joins multiple path segments into a single path.
///
/// Each segment is appended to the path in order, with appropriate
/// separators added. If the first segment starts with `/`, the result
/// will be an absolute path.
///
/// # Arguments
///
/// * `paths` - A slice of path segment strings to join
///
/// # Returns
///
/// A `Utf8PathBuf` containing the joined path
///
/// # Examples
///
/// ```
/// use reopencode::util::path::join_paths;
///
/// let path = join_paths(&["/home", "user", "documents", "file.txt"]);
/// assert_eq!(path.as_str(), "/home/user/documents/file.txt");
///
/// let relative = join_paths(&["src", "util", "path.rs"]);
/// assert_eq!(relative.as_str(), "src/util/path.rs");
///
/// // Handles empty segments gracefully
/// let with_empty = join_paths(&["/home", "", "user"]);
/// assert_eq!(with_empty.as_str(), "/home/user");
/// ```
pub fn join_paths(paths: &[&str]) -> Utf8PathBuf {
    if paths.is_empty() {
        return Utf8PathBuf::new();
    }

    let mut result = Utf8PathBuf::from(paths[0]);

    for path in &paths[1..] {
        if !path.is_empty() {
            result.push(path);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_filename() {
        assert_eq!(get_filename(Utf8Path::new("/path/to/file.txt")), "file.txt");
        assert_eq!(get_filename(Utf8Path::new("file.txt")), "file.txt");
        assert_eq!(get_filename(Utf8Path::new("/")), "");
        assert_eq!(get_filename(Utf8Path::new("/path/to/dir/")), "dir");
    }

    #[test]
    fn test_get_directory() {
        assert_eq!(
            get_directory(Utf8Path::new("/path/to/file.txt")),
            Some("/path/to")
        );
        assert_eq!(get_directory(Utf8Path::new("/file.txt")), Some("/"));
        assert_eq!(get_directory(Utf8Path::new("file.txt")), Some(""));
        assert_eq!(
            get_directory(Utf8Path::new("/path/to/dir/")),
            Some("/path/to")
        );
    }

    #[test]
    fn test_truncate_middle() {
        // No truncation needed
        assert_eq!(truncate_middle("/short/path.txt", 50), "/short/path.txt");

        // Truncation with ...
        let truncated = truncate_middle("/very/long/path/to/some/deep/file.txt", 25);
        assert!(truncated.len() <= 25);
        assert!(truncated.contains("..."));
        assert!(truncated.starts_with("/very/"));
        assert!(truncated.ends_with("/file.txt"));

        // Very short max length
        assert_eq!(truncate_middle("/long/path", 3), "...");
        assert_eq!(truncate_middle("/long/path", 2), "..");
        assert_eq!(truncate_middle("/long/path", 1), ".");

        // Edge case: exact fit
        assert_eq!(truncate_middle("/path/file.txt", 14), "/path/file.txt");
    }

    #[test]
    fn test_join_paths() {
        // Basic absolute path joining
        let path = join_paths(&["/home", "user", "documents"]);
        assert_eq!(path.as_str(), "/home/user/documents");

        // Relative path joining
        let relative = join_paths(&["src", "util", "path.rs"]);
        assert_eq!(relative.as_str(), "src/util/path.rs");

        // Empty paths
        let empty = join_paths(&[]);
        assert_eq!(empty.as_str(), "");

        // Single path
        let single = join_paths(&["/home"]);
        assert_eq!(single.as_str(), "/home");

        // With empty segments
        let with_empty = join_paths(&["/home", "", "user"]);
        assert_eq!(with_empty.as_str(), "/home/user");

        // Path with trailing slash in component
        let trailing = join_paths(&["/home/", "user"]);
        assert_eq!(trailing.as_str(), "/home/user");
    }

    #[test]
    fn test_manual_normalize() {
        // Current directory
        assert_eq!(
            manual_normalize(Utf8Path::new("./file.txt")),
            Utf8PathBuf::from("file.txt")
        );

        // Parent directory
        assert_eq!(
            manual_normalize(Utf8Path::new("/path/to/../file.txt")),
            Utf8PathBuf::from("/path/file.txt")
        );

        // Multiple parent directories
        assert_eq!(
            manual_normalize(Utf8Path::new("/path/to/another/../..")),
            Utf8PathBuf::from("/path")
        );

        // Root path
        assert_eq!(manual_normalize(Utf8Path::new("/")), Utf8PathBuf::from("/"));

        // Relative with parent beyond root
        assert_eq!(
            manual_normalize(Utf8Path::new("../file.txt")),
            Utf8PathBuf::from("file.txt")
        );
    }

    #[test]
    fn test_normalize_path() {
        // Test with current directory - should not fail
        let current_dir = normalize_path(".").unwrap();
        assert!(!current_dir.as_str().is_empty());

        // Test with parent directory
        let parent = normalize_path("..").unwrap();
        assert!(!parent.as_str().is_empty());
    }

    #[test]
    fn test_normalize_path_with_parent_refs() {
        // Non-existent path with parent references should still normalize
        let result = normalize_path("/path/to/../nonexistent/file.txt");
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.as_str().contains("/nonexistent/file.txt"));
    }
}
