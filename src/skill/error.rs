//! Skill module error types
//!
//! Provides error types for skill loading, parsing, and execution.

use std::path::PathBuf;

/// Main skill error type
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("IO 错误 '{0}': {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    #[error("解析错误: {0}")]
    ParseError(String),

    #[error("缺失字段 '{1}' in {0}")]
    MissingField(PathBuf, String),

    #[error("技能未找到: {0}")]
    NotFound(String),

    #[error("技能已存在: {0}")]
    AlreadyExists(String),

    #[error("无效的技能名称: {0}")]
    InvalidName(String),

    #[error("远程加载失败: {0}")]
    RemoteLoadError(String),

    #[error("权限被拒绝: {0}")]
    PermissionDenied(String),
}

/// Parse error for skill definition files
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("无效的 YAML: {0}")]
    InvalidYaml(String),

    #[error("未闭合的 frontmatter")]
    UnclosedFrontmatter,

    #[error("缺失必须字段: {0}")]
    MissingRequiredField(String),

    #[error("无效的字段值: {0}")]
    InvalidFieldValue(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_error_messages() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = SkillError::IoError(PathBuf::from("/path/to/skill.yaml"), io_err);
        assert!(err.to_string().contains("IO 错误"));
        assert!(err.to_string().contains("/path/to/skill.yaml"));

        let err = SkillError::ParseError("invalid syntax".to_string());
        assert!(err.to_string().contains("解析错误"));
        assert!(err.to_string().contains("invalid syntax"));

        let err = SkillError::MissingField(PathBuf::from("config.yaml"), "name".to_string());
        assert!(err.to_string().contains("缺失字段"));
        assert!(err.to_string().contains("name"));

        let err = SkillError::NotFound("my-skill".to_string());
        assert!(err.to_string().contains("技能未找到"));
        assert!(err.to_string().contains("my-skill"));

        let err = SkillError::AlreadyExists("existing-skill".to_string());
        assert!(err.to_string().contains("技能已存在"));
        assert!(err.to_string().contains("existing-skill"));

        let err = SkillError::InvalidName("@invalid!".to_string());
        assert!(err.to_string().contains("无效的技能名称"));
        assert!(err.to_string().contains("@invalid!"));

        let err = SkillError::RemoteLoadError("connection refused".to_string());
        assert!(err.to_string().contains("远程加载失败"));
        assert!(err.to_string().contains("connection refused"));

        let err = SkillError::PermissionDenied("read file".to_string());
        assert!(err.to_string().contains("权限被拒绝"));
        assert!(err.to_string().contains("read file"));
    }

    #[test]
    fn test_parse_error_messages() {
        let err = ParseError::InvalidYaml("mapping values are not allowed here".to_string());
        assert!(err.to_string().contains("无效的 YAML"));
        assert!(err.to_string().contains("mapping values"));

        let err = ParseError::UnclosedFrontmatter;
        assert!(err.to_string().contains("未闭合的 frontmatter"));

        let err = ParseError::MissingRequiredField("version".to_string());
        assert!(err.to_string().contains("缺失必须字段"));
        assert!(err.to_string().contains("version"));

        let err = ParseError::InvalidFieldValue("invalid type".to_string());
        assert!(err.to_string().contains("无效的字段值"));
        assert!(err.to_string().contains("invalid type"));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = SkillError::NotFound("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("NotFound"));

        let err = ParseError::InvalidYaml("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidYaml"));
    }

    #[test]
    fn test_error_clone() {
        let err = ParseError::MissingRequiredField("field".to_string());
        let cloned = err.clone();
        assert_eq!(err.to_string(), cloned.to_string());
    }
}
