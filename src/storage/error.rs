//! Storage module error types
//!
//! Provides error handling for storage operations including database,
//! migration, and general storage failures.

use thiserror::Error;

/// Main storage error type
#[derive(Debug, Error)]
pub enum StorageError {
    /// I/O operation failed
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization failed
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Database operation failed
    #[error("数据库错误: {0}")]
    Database(DatabaseError),

    /// Migration failed
    #[error("迁移错误: {0}")]
    Migration(#[from] MigrationError),

    /// Resource not found
    #[error("资源未找到: {0}")]
    NotFound(String),

    /// Invalid storage key
    #[error("无效的键: {0}")]
    InvalidKey(String),

    /// Transaction failed
    #[error("事务失败: {0}")]
    TransactionFailed(String),

    /// Operation timed out
    #[error("操作超时: {0}")]
    Timeout(String),

    /// Lock acquisition failed
    #[error("锁获取失败: {0}")]
    LockFailed(String),

    /// Invalid data format
    #[error("数据格式错误: {0}")]
    InvalidData(String),

    /// Backend not available
    #[error("后端不可用: {0}")]
    BackendUnavailable(String),
}

/// Database-specific errors
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Connection to database failed
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    /// Query execution failed
    #[error("查询失败: {0}")]
    QueryFailed(String),

    /// Transaction failed
    #[error("事务失败: {0}")]
    TransactionFailed(String),

    /// Migration failed
    #[error("迁移失败: {0}")]
    MigrationFailed(String),

    /// Database is locked
    #[error("数据库已锁定")]
    DatabaseLocked,

    /// Pool exhausted
    #[error("连接池耗尽")]
    PoolExhausted,

    /// Invalid configuration
    #[error("无效配置: {0}")]
    InvalidConfig(String),
}

impl From<sqlx::Error> for DatabaseError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(e) => DatabaseError::QueryFailed(e.to_string()),
            sqlx::Error::PoolTimedOut => DatabaseError::DatabaseLocked,
            sqlx::Error::PoolClosed => DatabaseError::PoolExhausted,
            sqlx::Error::Configuration(e) => DatabaseError::InvalidConfig(e.to_string()),
            _ => DatabaseError::ConnectionFailed(err.to_string()),
        }
    }
}

impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        StorageError::Database(DatabaseError::from(err))
    }
}

/// Migration-specific errors
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Migration file not found
    #[error("迁移文件未找到: {0}")]
    MigrationNotFound(String),

    /// Migration execution failed
    #[error("迁移执行失败: {0}")]
    ExecutionFailed(String),

    /// Version conflict
    #[error("版本冲突: 期望 {expected}, 实际 {actual}")]
    VersionConflict { expected: i32, actual: i32 },

    /// Invalid migration
    #[error("无效迁移: {0}")]
    InvalidMigration(String),

    /// Rollback failed
    #[error("回滚失败: {0}")]
    RollbackFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_error_messages() {
        let err = StorageError::NotFound("session_123".to_string());
        assert!(err.to_string().contains("资源未找到"));
        assert!(err.to_string().contains("session_123"));

        let err = StorageError::InvalidKey("empty key".to_string());
        assert!(err.to_string().contains("无效的键"));

        let err = StorageError::Timeout("write operation".to_string());
        assert!(err.to_string().contains("操作超时"));
    }

    #[test]
    fn test_database_error_messages() {
        let err = DatabaseError::ConnectionFailed("localhost:5432".to_string());
        assert!(err.to_string().contains("连接失败"));

        let err = DatabaseError::DatabaseLocked;
        assert!(err.to_string().contains("数据库已锁定"));

        let err = DatabaseError::PoolExhausted;
        assert!(err.to_string().contains("连接池耗尽"));
    }

    #[test]
    fn test_migration_error_messages() {
        let err = MigrationError::VersionConflict {
            expected: 5,
            actual: 3,
        };
        assert!(err.to_string().contains("版本冲突"));
        assert!(err.to_string().contains("5"));
        assert!(err.to_string().contains("3"));

        let err = MigrationError::ExecutionFailed("SQL syntax error".to_string());
        assert!(err.to_string().contains("迁移执行失败"));
    }

    #[test]
    fn test_error_from_sqlx() {
        let sqlx_err = sqlx::Error::PoolTimedOut;
        let db_err: DatabaseError = sqlx_err.into();
        assert!(matches!(db_err, DatabaseError::DatabaseLocked));
    }
}
