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
pub use error::{ConfigError, MergeError, ValidationError};

// Re-export config types
pub use types::{
    AgentConfig, AgentConfigs, AgentMode, CommandConfig, Config, HookConfig, McpConfig,
    McpLocalConfig, McpRemoteConfig, PermissionConfig, PermissionPolicy, PermissionRule,
    ProviderConfig, ServerConfig, SkillsConfig, StorageConfig, StorageType,
};

// Re-export CategoryConfig from category module
pub use crate::category::CategoryConfig;

// Re-export loader
pub use loader::{ConfigLayer, ConfigLoader};

// Re-export paths
pub use paths::ConfigPaths;

// Re-export merge
pub use merge::MergeStrategy;

// Re-export validation
pub use validation::Validator;

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
