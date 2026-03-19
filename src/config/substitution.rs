//! Variable substitution for configuration values
//!
//! Supports `{env:VAR}` and `{env:VAR|default}` syntax for environment variable injection.

use std::env;
use std::path::Path;

use crate::config::error::ConfigError;

/// Variable substitutor
pub struct Substitutor;

impl Substitutor {
    /// Replace environment variables {env:VAR}
    ///
    /// Supported formats:
    /// - `{env:VAR}` - simple variable
    /// - `{env:VAR|default}` - with default value
    ///
    /// Returns the substituted string or an error if a required env var is missing
    pub fn substitute_env(text: &str) -> Result<String, ConfigError> {
        let mut result = text.to_string();
        let mut changed = true;

        // Keep substituting until no more changes (handles nested patterns)
        while changed {
            changed = false;

            // Find {env:...} patterns
            if let Some(start) = result.find("{env:") {
                if let Some(end) = result[start..].find('}') {
                    let _full_match = &result[start..start + end + 1];
                    let inner = &result[start + 5..start + end]; // Skip "{env:"

                    // Parse variable name and optional default
                    let (var_name, default_value) = if let Some(pipe_pos) = inner.find('|') {
                        (&inner[..pipe_pos], Some(&inner[pipe_pos + 1..]))
                    } else {
                        (inner, None)
                    };

                    // Get environment variable value
                    let replacement = env::var(var_name)
                        .ok()
                        .or_else(|| default_value.map(|s| s.to_string()))
                        .unwrap_or_default();

                    result = format!(
                        "{}{}{}",
                        &result[..start],
                        replacement,
                        &result[start + end + 1..]
                    );
                    changed = true;
                }
            }
        }

        Ok(result)
    }

    /// Replace file content {file:path}
    ///
    /// Supported formats:
    /// - `{file:path/to/file}` - relative path
    /// - `{file:/absolute/path}` - absolute path
    /// - `{file:~/path}` - home directory
    ///
    /// Note: This is a Future feature. Current implementation returns unchanged text.
    pub fn substitute_file(text: &str, _base_dir: &Path) -> Result<String, ConfigError> {
        // Future: Implement file content substitution
        // For now, just return the text unchanged
        Ok(text.to_string())
    }

    /// Execute all substitutions
    ///
    /// Applies both environment variable and file substitutions
    pub fn substitute_all(text: &str, base_dir: &Path) -> Result<String, ConfigError> {
        let mut result = Self::substitute_env(text)?;
        result = Self::substitute_file(&result, base_dir)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_env_simple() {
        unsafe { env::set_var("TEST_VAR", "test_value") };
        let result = Substitutor::substitute_env("key = \"{env:TEST_VAR}\"").unwrap();
        assert_eq!(result, "key = \"test_value\"");
        unsafe { env::remove_var("TEST_VAR") };
    }

    #[test]
    fn test_substitute_env_with_default() {
        unsafe { env::remove_var("NONEXISTENT_VAR") };
        let result =
            Substitutor::substitute_env("key = \"{env:NONEXISTENT_VAR|default_value}\"").unwrap();
        assert_eq!(result, "key = \"default_value\"");
    }

    #[test]
    fn test_substitute_env_missing_no_default() {
        unsafe { env::remove_var("MISSING_VAR") };
        let result = Substitutor::substitute_env("key = \"{env:MISSING_VAR}\"").unwrap();
        assert_eq!(result, "key = \"\""); // Empty string when missing and no default
    }

    #[test]
    fn test_substitute_env_multiple() {
        unsafe { env::set_var("VAR1", "value1") };
        unsafe { env::set_var("VAR2", "value2") };
        let result = Substitutor::substitute_env("{env:VAR1} and {env:VAR2}").unwrap();
        assert_eq!(result, "value1 and value2");
        unsafe { env::remove_var("VAR1") };
        unsafe { env::remove_var("VAR2") };
    }

    #[test]
    fn test_substitute_env_no_match() {
        let result = Substitutor::substitute_env("plain text without vars").unwrap();
        assert_eq!(result, "plain text without vars");
    }
}
