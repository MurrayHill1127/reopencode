//! Configuration error types
//! 
//! Provides user-friendly error messages for config parsing, validation, and loading failures.

use std::path::PathBuf;

/// Main configuration error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置文件不存在：{0}")]
    FileNotFound(PathBuf),

    #[error("TOML 解析失败：{0}")]
    ParseToml(#[from] toml::de::Error),

    #[error("环境变量替换失败：{0}")]
    SubstitutionFailed(String),

    #[error("文件读取失败：{0}: {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("验证失败：{0}")]
    Validation(#[from] ValidationError),

    #[error("合并失败：{0}")]
    Merge(#[from] MergeError),

    #[error("远程配置加载失败：{0}")]
    RemoteLoadFailed(String),

    #[error("未知配置错误：{0}")]
    Unknown(String),
}

/// Validation error with field path and line number
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("字段 '{field}' 是必填的")]
    RequiredField { field: String },

    #[error("字段 '{field}' 值无效：{message}")]
    InvalidValue { field: String, message: String },

    #[error("字段 '{field}' 超出范围：{actual} (期望：{expected})")]
    OutOfRange { field: String, actual: String, expected: String },

    #[error("字段 '{field}' 格式错误：{message}")]
    InvalidFormat { field: String, message: String },

    #[error("{0}")]
    Custom(String),
}

/// Merge conflict error
#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    #[error("类型不匹配：字段 '{0}'")]
    TypeMismatch(String),

    #[error("无法合并的数组：字段 '{0}'")]
    UnmergableArray(String),

    #[error("合并冲突：字段 '{0}'")]
    Conflict(String),
}