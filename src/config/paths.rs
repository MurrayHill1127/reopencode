//! Configuration path resolution
//!
//! Cross-platform path resolution for global and project configs.

use crate::config::error::ConfigError;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration path resolver
pub struct ConfigPaths;

impl ConfigPaths {
    pub fn global_config_dir() -> PathBuf {
        // On macOS, dirs::config_dir() returns ~/Library/Application Support,
        // but we prefer the XDG-style ~/.config/roc if it exists.
        let xdg = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config").join("roc");
        if xdg.exists() {
            return xdg;
        }
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("roc")
    }

    pub fn global_config_file() -> PathBuf {
        Self::global_config_dir().join("roc.toml")
    }

    pub fn project_files(start_dir: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if let Some(path) = Self::find_up(start_dir, "roc.toml") {
            files.push(path);
        }

        if let Some(path) = Self::find_up(start_dir, "opencode.toml") {
            files.push(path);
        }

        files
    }

    fn find_up(start_dir: &Path, filename: &str) -> Option<PathBuf> {
        let mut current = start_dir.to_path_buf();

        loop {
            let file_path = current.join(filename);
            if file_path.exists() {
                return Some(file_path);
            }

            if !current.pop() {
                return None;
            }
        }
    }

    pub fn read_file(path: &Path) -> Result<Option<String>, ConfigError> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ConfigError::IoError(path.to_path_buf(), e)),
        }
    }

    pub fn parse_text(text: &str) -> Result<toml::Value, ConfigError> {
        toml::from_str(text).map_err(ConfigError::ParseToml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_global_config_dir() {
        let dir = ConfigPaths::global_config_dir();
        assert!(dir.ends_with("roc") || dir.to_string_lossy().contains("roc"));
    }

    #[test]
    fn test_global_config_file() {
        let file = ConfigPaths::global_config_file();
        assert!(file.ends_with("roc.toml") || file.to_string_lossy().ends_with("roc/roc.toml"));
    }

    #[test]
    fn test_parse_text_valid() {
        let text = r#"
[server]
host = "localhost"
port = 8080
"#;
        let result = ConfigPaths::parse_text(text);
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value.get("server").is_some());
    }

    #[test]
    fn test_parse_text_invalid() {
        let text = "invalid toml [[[";
        let result = ConfigPaths::parse_text(text);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_files_not_found() {
        let temp_dir = env::temp_dir();
        let files = ConfigPaths::project_files(&temp_dir);
        assert!(files.is_empty() || files.iter().all(|p| !p.exists()));
    }
}
