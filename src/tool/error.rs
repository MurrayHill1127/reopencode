use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Execution error: {0}")]
    Execution(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl ToolError {
    pub fn is_execution(&self) -> bool {
        matches!(self, ToolError::Execution(_))
    }

    pub fn is_io(&self) -> bool {
        matches!(self, ToolError::Io(_))
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, ToolError::NotFound(_))
    }
}

pub type Result<T> = std::result::Result<T, ToolError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let exec_err = ToolError::Execution("command failed".to_string());
        assert!(exec_err.is_execution());
        assert!(!exec_err.is_io());
        assert!(!exec_err.is_not_found());

        let parse_err = ToolError::Parse("invalid json".to_string());
        assert!(!parse_err.is_execution());

        let not_found_err = ToolError::NotFound("file.txt".to_string());
        assert!(not_found_err.is_not_found());
    }

    #[test]
    fn test_error_helpers() {
        let exec = ToolError::Execution("test".to_string());
        assert!(exec.is_execution());
        assert!(!exec.is_io());
        assert!(!exec.is_not_found());

        let io_err = ToolError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(!io_err.is_execution());
        assert!(io_err.is_io());

        let not_found = ToolError::NotFound("resource".to_string());
        assert!(!not_found.is_execution());
        assert!(!not_found.is_io());
        assert!(not_found.is_not_found());
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let tool_err: ToolError = io_err.into();
        assert!(tool_err.is_io());
    }
}