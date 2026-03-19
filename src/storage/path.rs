//! Global path management for ROC storage
//!
//! Provides XDG-compliant directory paths for data, config, cache, and state.
//! Corresponds to TypeScript Global.Path (global/index.ts:14-26)

use std::path::PathBuf;
use std::sync::OnceLock;

/// Cache version for cache invalidation
pub const CACHE_VERSION: &str = "21";

/// Global paths singleton
///
/// Manages all application directories following XDG Base Directory specification.
///
/// # Directory Layout
///
/// ```text
/// ~/.local/share/opencode/   # data - databases, sessions, projects
/// ~/.config/opencode/        # config - configuration files
/// ~/.cache/opencode/         # cache - temporary cache files
/// ~/.local/state/opencode/   # state - runtime state, locks
/// ```
#[derive(Debug, Clone)]
pub struct GlobalPath {
    /// User home directory
    pub home: PathBuf,

    /// Data directory (~/.local/share/opencode/)
    /// Stores: databases, sessions, projects, logs
    pub data: PathBuf,

    /// Binary directory (~/.local/share/opencode/bin/)
    pub bin: PathBuf,

    /// Log directory (~/.local/share/opencode/log/)
    pub log: PathBuf,

    /// Cache directory (~/.cache/opencode/)
    /// Stores: temporary caches, version markers
    pub cache: PathBuf,

    /// Config directory (~/.config/opencode/)
    /// Stores: configuration files, OAuth tokens, MCP configs
    pub config: PathBuf,

    /// State directory (~/.local/state/opencode/)
    /// Stores: runtime state, lock files
    pub state: PathBuf,
}

impl GlobalPath {
    /// Get the global singleton instance
    ///
    /// Uses `OnceLock` for thread-safe lazy initialization.
    /// Respects `OPENCODE_TEST_HOME` environment variable for testing.
    pub fn get() -> &'static Self {
        static INSTANCE: OnceLock<GlobalPath> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let home = std::env::var("OPENCODE_TEST_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_default());

            let data = dirs::data_local_dir()
                .unwrap_or_else(|| home.join(".local/share"))
                .join("opencode");

            let cache = dirs::cache_dir()
                .unwrap_or_else(|| home.join(".cache"))
                .join("opencode");

            let config = dirs::config_dir()
                .unwrap_or_else(|| home.join(".config"))
                .join("opencode");

            let state = dirs::state_dir()
                .unwrap_or_else(|| home.join(".local/state"))
                .join("opencode");

            Self {
                home,
                data: data.clone(),
                bin: data.join("bin"),
                log: data.join("log"),
                cache,
                config,
                state,
            }
        })
    }

    /// Get database path for a specific channel
    ///
    /// - `latest` or `beta` → `opencode.db`
    /// - Other channels → `opencode-{sanitized-channel}.db`
    pub fn database_path(&self, channel: &str) -> PathBuf {
        if ["latest", "beta"].contains(&channel) {
            self.data.join("opencode.db")
        } else {
            let safe = channel.replace(
                |c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '-',
                "-",
            );
            self.data.join(format!("opencode-{}.db", safe))
        }
    }

    /// Initialize all required directories
    ///
    /// Creates data, config, state, log, and bin directories if they don't exist.
    pub async fn init(&self) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.data).await?;
        tokio::fs::create_dir_all(&self.config).await?;
        tokio::fs::create_dir_all(&self.state).await?;
        tokio::fs::create_dir_all(&self.log).await?;
        tokio::fs::create_dir_all(&self.bin).await?;
        tokio::fs::create_dir_all(&self.cache).await?;
        Ok(())
    }

    /// Get session storage directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.data.join("session")
    }

    /// Get message storage directory
    pub fn messages_dir(&self) -> PathBuf {
        self.data.join("message")
    }

    /// Get project storage directory
    pub fn projects_dir(&self) -> PathBuf {
        self.data.join("project")
    }

    /// Get todo storage directory
    pub fn todos_dir(&self) -> PathBuf {
        self.data.join("todo")
    }

    /// Get the main configuration file path
    pub fn config_file(&self) -> PathBuf {
        self.config.join("opencode.json")
    }

    /// Get the MCP OAuth tokens file path
    pub fn mcp_oauth_file(&self) -> PathBuf {
        self.config.join("mcp-oauth.json")
    }

    /// Get cache version file path
    pub fn cache_version_file(&self) -> PathBuf {
        self.cache.join("version")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_global_path_singleton() {
        let path1 = GlobalPath::get();
        let path2 = GlobalPath::get();
        // Both should be the same instance
        assert!(std::ptr::eq(path1, path2));
    }

    #[test]
    fn test_database_path_latest() {
        let path = GlobalPath::get();
        let db_path = path.database_path("latest");
        assert!(db_path.ends_with("opencode.db"));
    }

    #[test]
    fn test_database_path_beta() {
        let path = GlobalPath::get();
        let db_path = path.database_path("beta");
        assert!(db_path.ends_with("opencode.db"));
    }

    #[test]
    fn test_database_path_custom_channel() {
        let path = GlobalPath::get();
        let db_path = path.database_path("my-channel");
        assert!(db_path.ends_with("opencode-my-channel.db"));
    }

    #[test]
    fn test_database_path_sanitization() {
        let path = GlobalPath::get();
        let db_path = path.database_path("my@channel#test!");
        assert!(db_path.ends_with("opencode-my-channel-test-.db"));
    }

    #[tokio::test]
    async fn test_init_directories() {
        let temp = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("OPENCODE_TEST_HOME", temp.path());
        }

        let home = temp.path().to_path_buf();
        let data = home.join(".local/share/opencode");
        let config = home.join(".config/opencode");

        tokio::fs::create_dir_all(&data).await.unwrap();
        tokio::fs::create_dir_all(&config).await.unwrap();

        assert!(data.exists());
        assert!(config.exists());

        unsafe {
            std::env::remove_var("OPENCODE_TEST_HOME");
        }
    }

    #[test]
    fn test_subdirectory_paths() {
        let path = GlobalPath::get();
        assert!(path.sessions_dir().ends_with("session"));
        assert!(path.messages_dir().ends_with("message"));
        assert!(path.projects_dir().ends_with("project"));
        assert!(path.todos_dir().ends_with("todo"));
    }

    #[test]
    fn test_config_file_paths() {
        let path = GlobalPath::get();
        assert!(path.config_file().ends_with("opencode.json"));
        assert!(path.mcp_oauth_file().ends_with("mcp-oauth.json"));
    }
}
