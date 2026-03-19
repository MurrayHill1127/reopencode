//! Configuration loader
//!
//! Multi-layer config loading with priority:
//! defaults -> global -> project -> env

use crate::config::error::ConfigError;
use crate::config::merge::MergeStrategy;
use crate::config::paths::ConfigPaths;
use crate::config::substitution::Substitutor;
use crate::config::types::Config;
use crate::config::validation::Validator;
use std::env;
use std::path::{Path, PathBuf};

/// Configuration layer source
#[derive(Debug, Clone)]
pub enum ConfigLayer {
    Default,
    Global,
    Project(PathBuf),
    File(PathBuf),
    EnvPrefix(String),
}

/// Configuration loader with builder pattern
pub struct ConfigLoader {
    layers: Vec<ConfigLayer>,
    substitutions_enabled: bool,
}

impl ConfigLoader {
    /// 创建新的加载器
    pub fn new() -> Self {
        Self {
            layers: vec![ConfigLayer::Default],
            substitutions_enabled: true,
        }
    }

    /// 添加全局配置层 (~/.config/roc/roc.toml)
    pub fn with_global_config(mut self) -> Self {
        self.layers.push(ConfigLayer::Global);
        self
    }

    /// 添加项目配置层 (查找 roc.toml / opencode.toml)
    pub fn with_project_config(mut self, dir: &Path) -> Self {
        self.layers.push(ConfigLayer::Project(dir.to_path_buf()));
        self
    }

    /// 添加自定义配置文件
    pub fn with_file(mut self, path: PathBuf) -> Self {
        self.layers.push(ConfigLayer::File(path));
        self
    }

    /// 启用环境变量覆盖 (ROC__SERVER__PORT=8080)
    pub fn with_env_prefix(mut self, prefix: &str) -> Self {
        self.layers.push(ConfigLayer::EnvPrefix(prefix.to_string()));
        self
    }

    /// 启用/禁用变量替换
    pub fn with_substitutions(mut self, enabled: bool) -> Self {
        self.substitutions_enabled = enabled;
        self
    }

    /// 执行加载
    pub fn load(self) -> Result<Config, ConfigError> {
        let mut config = Config::default();

        for layer in &self.layers {
            let layer_config = self.load_layer(layer)?;
            config = MergeStrategy::merge(&config, &layer_config);
        }

        // Validate final config
        Validator::validate(&config)?;

        Ok(config)
    }

    /// 从单个文件加载 (快捷方法)
    pub fn from_file(path: &Path) -> Result<Config, ConfigError> {
        Self::new().with_file(path.to_path_buf()).load()
    }

    /// 加载单个配置层
    fn load_layer(&self, layer: &ConfigLayer) -> Result<Config, ConfigError> {
        match layer {
            ConfigLayer::Default => Ok(Config::default()),

            ConfigLayer::Global => {
                let path = ConfigPaths::global_config_file();
                match ConfigPaths::read_file(&path)? {
                    Some(content) => {
                        let content = if self.substitutions_enabled {
                            Substitutor::substitute_env(&content)?
                        } else {
                            content
                        };
                        Self::parse_config(&content)
                    }
                    None => Ok(Config::default()),
                }
            }

            ConfigLayer::Project(dir) => {
                let files = ConfigPaths::project_files(dir);
                let mut config = Config::default();

                for path in files {
                    if let Some(content) = ConfigPaths::read_file(&path)? {
                        let content = if self.substitutions_enabled {
                            Substitutor::substitute_env(&content)?
                        } else {
                            content
                        };
                        if let Ok(layer_config) = Self::parse_config(&content) {
                            config = MergeStrategy::merge(&config, &layer_config);
                        }
                    }
                }

                Ok(config)
            }

            ConfigLayer::File(path) => match ConfigPaths::read_file(path)? {
                Some(content) => {
                    let content = if self.substitutions_enabled {
                        Substitutor::substitute_env(&content)?
                    } else {
                        content
                    };
                    Self::parse_config(&content)
                }
                None => Err(ConfigError::FileNotFound(path.clone())),
            },

            ConfigLayer::EnvPrefix(prefix) => self.load_from_env(prefix),
        }
    }

    /// Parse TOML content into Config
    fn parse_config(content: &str) -> Result<Config, ConfigError> {
        toml::from_str(content).map_err(ConfigError::ParseToml)
    }

    /// 从环境变量加载配置
    fn load_from_env(&self, prefix: &str) -> Result<Config, ConfigError> {
        // Basic env var loading: ROC__SERVER__PORT=8080
        // This is a simplified implementation
        let mut config = Config::default();

        // Check for server config env vars
        if let Some(port) = env::var(format!("{}__SERVER__PORT", prefix))
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
        {
            config.server.port = port;
        }

        if let Ok(host) = env::var(format!("{}__SERVER__HOST", prefix)) {
            config.server.host = host;
        }

        Ok(config)
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loader_default() {
        let loader = ConfigLoader::new();
        assert_eq!(loader.layers.len(), 1);
    }

    #[test]
    fn test_loader_chain() {
        let loader = ConfigLoader::new()
            .with_global_config()
            .with_project_config(Path::new("."))
            .with_env_prefix("ROC");

        assert_eq!(loader.layers.len(), 4);
    }

    #[test]
    fn test_load_default_config() {
        let config = ConfigLoader::new().load().unwrap();
        assert_eq!(config.server.port, 4096);
        assert_eq!(config.server.host, "127.0.0.1");
    }
}
