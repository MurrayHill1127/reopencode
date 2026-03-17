//! Remote skill loader (stub for MVP)
//!
//! This module provides functionality for loading skills from remote sources.
//! Currently a stub implementation - remote loading will be implemented in a future version.

use std::path::PathBuf;

use super::{SkillError, SkillInfo};

/// Remote skill loader (stub for MVP)
///
/// This struct is designed to handle downloading and caching skills from remote URLs.
/// Currently returns errors as remote loading is not implemented.
pub struct SkillLoader {
    /// Directory for caching downloaded skills
    cache_dir: PathBuf,
}

impl SkillLoader {
    /// Create a new SkillLoader with default cache directory
    pub fn new() -> Self {
        Self::with_cache_dir(default_cache_dir())
    }

    /// Create a new SkillLoader with a custom cache directory
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Pull skills from remote URL (not implemented in MVP)
    ///
    /// # Errors
    ///
    /// Returns [`SkillError::RemoteLoadError`] indicating that remote loading
    /// is not yet implemented.
    pub async fn pull_from_url(
        &self,
        _url: &str,
    ) -> Result<Vec<SkillInfo>, SkillError> {
        // MVP stub: return error
        Err(SkillError::RemoteLoadError(
            "Remote skill loading not implemented in MVP".to_string(),
        ))
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Pull skills from URL (stub for MVP)
///
/// This is a convenience function that wraps [`SkillLoader::pull_from_url`].
///
/// # Errors
///
/// Currently returns an empty vector as remote loading is not implemented.
/// A warning is logged to indicate this is a stub implementation.
///
/// # Logging
///
/// Logs a warning using [`tracing::warn!`] indicating that remote skill loading
/// is not implemented in MVP.
pub async fn pull_skills_from_url(_url: &str) -> Result<Vec<SkillInfo>, SkillError> {
    // MVP stub: return empty vec
    tracing::warn!("Remote skill loading not implemented in MVP");
    Ok(Vec::new())
}

/// Get the default cache directory for skills
fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("reopencode")
        .join("skills")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_loader_new() {
        let loader = SkillLoader::new();
        assert!(loader.cache_dir().ends_with("reopencode/skills"));
    }

    #[test]
    fn test_skill_loader_with_cache_dir() {
        let custom_dir = PathBuf::from("/custom/cache");
        let loader = SkillLoader::with_cache_dir(custom_dir.clone());
        assert_eq!(loader.cache_dir(), &custom_dir);
    }

    #[test]
    fn test_skill_loader_default() {
        let loader = SkillLoader::default();
        let loader2 = SkillLoader::new();
        assert_eq!(loader.cache_dir(), loader2.cache_dir());
    }

    #[test]
    fn test_pull_skills_from_url_returns_empty() {
        // Use a blocking runtime for testing async function
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(pull_skills_from_url("https://example.com/skills"));
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_pull_from_url_returns_error() {
        let loader = SkillLoader::new();
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(loader.pull_from_url("https://example.com/skills"));
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkillError::RemoteLoadError(_)));
    }

    #[test]
    fn test_default_cache_dir() {
        let cache_dir = default_cache_dir();
        assert!(cache_dir.to_string_lossy().contains("reopencode"));
        assert!(cache_dir.to_string_lossy().contains("skills"));
    }
}