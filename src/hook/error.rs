//! Hook module error types
//!
//! Provides user-friendly error messages for hook registration, execution, and configuration failures.

/// Hook identifier type used in error messages
pub type HookId = String;

/// Main hook error type
#[derive(Debug, Clone, thiserror::Error)]
pub enum HookError {
    /// Hook is already registered with the given ID
    #[error("钩子已注册: {0}")]
    AlreadyRegistered(HookId),

    /// Hook not found in registry
    #[error("钩子未找到: {0}")]
    NotFound(HookId),

    /// Hook execution timed out
    #[error("钩子执行超时: {0}")]
    Timeout(HookId),

    /// Hook execution failed with error message
    #[error("钩子执行失败: {0}")]
    ExecutionFailed(String),

    /// Invalid hook configuration
    #[error("无效的钩子配置: {0}")]
    InvalidConfig(String),

    /// Hook is disabled and cannot be executed
    #[error("钩子被禁用: {0}")]
    Disabled(HookId),

    /// Invalid event type for hook operation
    #[error("无效的事件类型")]
    InvalidEventType,
}

/// Execution-related errors during hook chain processing
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    /// Execution was interrupted by external signal or cancellation
    #[error("执行被中断: {0}")]
    Interrupted(String),

    /// Result type mismatch between expected and actual
    #[error("执行结果类型不匹配")]
    TypeMismatch,

    /// Required context is missing
    #[error("上下文缺失: {0}")]
    MissingContext(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_error_messages() {
        let err = HookError::AlreadyRegistered("test-hook".to_string());
        assert!(err.to_string().contains("钩子已注册"));
        assert!(err.to_string().contains("test-hook"));

        let err = HookError::NotFound("missing-hook".to_string());
        assert!(err.to_string().contains("钩子未找到"));

        let err = HookError::Timeout("slow-hook".to_string());
        assert!(err.to_string().contains("钩子执行超时"));

        let err = HookError::ExecutionFailed("something went wrong".to_string());
        assert!(err.to_string().contains("钩子执行失败"));

        let err = HookError::InvalidConfig("missing required field".to_string());
        assert!(err.to_string().contains("无效的钩子配置"));

        let err = HookError::Disabled("disabled-hook".to_string());
        assert!(err.to_string().contains("钩子被禁用"));

        let err = HookError::InvalidEventType;
        assert!(err.to_string().contains("无效的事件类型"));
    }

    #[test]
    fn test_execution_error_messages() {
        let err = ExecutionError::Interrupted("user cancelled".to_string());
        assert!(err.to_string().contains("执行被中断"));

        let err = ExecutionError::TypeMismatch;
        assert!(err.to_string().contains("类型不匹配"));

        let err = ExecutionError::MissingContext("session_id".to_string());
        assert!(err.to_string().contains("上下文缺失"));
    }

    #[test]
    fn test_error_debug_impl() {
        let err = HookError::AlreadyRegistered("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("AlreadyRegistered"));
    }
}