//! Command 模块错误类型
//!
//! 定义命令解析、渲染、执行过程中可能发生的错误。

use std::path::PathBuf;

/// 命令错误
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// 命令不存在
    #[error("命令不存在: {0}")]
    NotFound(String),

    /// 命令已禁用
    #[error("命令已禁用: {0}")]
    Disabled(String),

    /// 命令解析失败
    #[error("命令解析失败: {0}")]
    ParseError(#[from] ParseError),

    /// 模板渲染失败
    #[error("模板渲染失败: {0}")]
    RenderError(#[from] RenderError),

    /// 命令文件读取失败
    #[error("命令文件读取失败: {0}: {1}")]
    FileReadError(PathBuf, std::io::Error),

    /// 命令 frontmatter 解析失败
    #[error("命令 frontmatter 解析失败: {0}")]
    FrontmatterError(String),
}

/// 解析错误
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    /// 无效的命令格式
    #[error("无效的命令格式: {0}")]
    InvalidFormat(String),

    /// 命令名称不能为空
    #[error("命令名称不能为空")]
    EmptyName,

    /// 命令名称包含非法字符
    #[error("命令名称包含非法字符: {0}")]
    InvalidCharacters(String),
}

/// 渲染错误
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// 缺少必需变量
    #[error("缺少必需变量: {0}")]
    MissingVariable(String),

    /// 变量替换失败
    #[error("变量替换失败: {0}")]
    SubstitutionFailed(String),
}

impl CommandError {
    /// Create a not found error
    pub fn not_found(name: impl Into<String>) -> Self {
        Self::NotFound(name.into())
    }

    /// Create a disabled error
    pub fn disabled(name: impl Into<String>) -> Self {
        Self::Disabled(name.into())
    }

    /// Create a file read error
    pub fn file_read(path: impl Into<PathBuf>, err: std::io::Error) -> Self {
        Self::FileReadError(path.into(), err)
    }

    /// Create a frontmatter parse error
    pub fn frontmatter(msg: impl Into<String>) -> Self {
        Self::FrontmatterError(msg.into())
    }
}

impl ParseError {
    /// Create an invalid format error
    pub fn invalid_format(msg: impl Into<String>) -> Self {
        Self::InvalidFormat(msg.into())
    }

    /// Create an invalid characters error
    pub fn invalid_chars(chars: impl Into<String>) -> Self {
        Self::InvalidCharacters(chars.into())
    }
}

impl RenderError {
    /// Create a missing variable error
    pub fn missing_variable(name: impl Into<String>) -> Self {
        Self::MissingVariable(name.into())
    }

    /// Create a substitution failed error
    pub fn substitution_failed(msg: impl Into<String>) -> Self {
        Self::SubstitutionFailed(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_messages() {
        let err = CommandError::not_found("test-cmd");
        assert_eq!(err.to_string(), "命令不存在: test-cmd");

        let err = CommandError::disabled("disabled-cmd");
        assert_eq!(err.to_string(), "命令已禁用: disabled-cmd");

        let err = CommandError::frontmatter("invalid yaml");
        assert!(err.to_string().contains("frontmatter"));
    }

    #[test]
    fn test_parse_error_messages() {
        let err = ParseError::invalid_format("expected 'name: desc'");
        assert!(err.to_string().contains("无效的命令格式"));

        let err = ParseError::EmptyName;
        assert_eq!(err.to_string(), "命令名称不能为空");

        let err = ParseError::invalid_chars("@#$");
        assert!(err.to_string().contains("非法字符"));
    }

    #[test]
    fn test_render_error_messages() {
        let err = RenderError::missing_variable("file_path");
        assert!(err.to_string().contains("缺少必需变量"));

        let err = RenderError::substitution_failed("unmatched brace");
        assert!(err.to_string().contains("变量替换失败"));
    }

    #[test]
    fn test_error_from_conversions() {
        let parse_err = ParseError::EmptyName;
        let cmd_err: CommandError = parse_err.into();
        assert!(cmd_err.to_string().contains("解析失败"));

        let render_err = RenderError::missing_variable("x");
        let cmd_err: CommandError = render_err.into();
        assert!(cmd_err.to_string().contains("渲染失败"));
    }
}
