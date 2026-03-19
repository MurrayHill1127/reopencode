//! Configuration module for ROC
//!
//! Provides multi-layer configuration loading with support for:
//! - TOML format parsing
//! - Environment variable substitution (`{env:VAR}`)
//! - Deep merge of configuration layers
//! - Schema validation
//!
//! # Example
//!
//! ```no_run
//! use reopencode::config::{Config, ConfigLoader};
//!
//! // Quick load with defaults
//! let config = Config::load().unwrap();
//!
//! // Or use builder pattern
//! let config = ConfigLoader::new()
//!     .with_global_config()
//!     .with_project_config(&std::env::current_dir().unwrap())
//!     .load()
//!     .unwrap();
//! ```

pub mod error;
pub mod loader;
pub mod merge;
pub mod paths;
pub mod substitution;
pub mod types;
pub mod validation;

// Re-export error types
pub use error::ConfigError;

// Re-export config types
pub use types::{
    Config, McpConfig,
    McpLocalConfig, McpRemoteConfig,
};

// Re-export CategoryConfig from category module

// Re-export loader
pub use loader::ConfigLoader;

// Re-export paths

// Re-export merge

// Re-export validation

impl Config {
    /// 快捷方法：加载完整配置
    ///
    /// 等价于 `ConfigLoader::new().load()`
    pub fn load() -> Result<Self, ConfigError> {
        ConfigLoader::new().load()
    }

    /// 快捷方法：从文件加载配置
    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        ConfigLoader::new().with_file(path.to_path_buf()).load()
    }
}
