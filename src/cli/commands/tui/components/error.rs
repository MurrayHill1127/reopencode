//! Error types for TUI components.
//!
//! This module provides error handling for component operations including
//! rendering, input handling, state management, and focus control.

use thiserror::Error;

/// Errors that can occur in TUI component operations.
#[derive(Debug, Error)]
pub enum ComponentError {
    /// Error during component rendering.
    #[error("Render error: {0}")]
    Render(String),

    /// Error during input handling.
    #[error("Input error: {0}")]
    Input(String),

    /// Error in component state management.
    #[error("State error: {0}")]
    State(String),

    /// Error related to focus management.
    #[error("Focus error: {0}")]
    Focus(String),

    /// Component not found.
    #[error("Component not found: {0}")]
    NotFound(String),
}

/// Result type alias for component operations.
pub type Result<T> = std::result::Result<T, ComponentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_error() {
        let err = ComponentError::Render("failed to draw widget".to_string());
        assert!(err.to_string().contains("Render error"));
        assert!(err.to_string().contains("failed to draw widget"));
    }

    #[test]
    fn test_input_error() {
        let err = ComponentError::Input("invalid key event".to_string());
        assert!(err.to_string().contains("Input error"));
        assert!(err.to_string().contains("invalid key event"));
    }

    #[test]
    fn test_state_error() {
        let err = ComponentError::State("invalid state transition".to_string());
        assert!(err.to_string().contains("State error"));
        assert!(err.to_string().contains("invalid state transition"));
    }

    #[test]
    fn test_focus_error() {
        let err = ComponentError::Focus("cannot focus disabled component".to_string());
        assert!(err.to_string().contains("Focus error"));
        assert!(err.to_string().contains("cannot focus disabled component"));
    }

    #[test]
    fn test_not_found_error() {
        let err = ComponentError::NotFound("message_list".to_string());
        assert!(err.to_string().contains("Component not found"));
        assert!(err.to_string().contains("message_list"));
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> Result<()> {
            Err(ComponentError::NotFound("test".to_string()))
        }

        let result = returns_result();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ComponentError::NotFound(_)));
    }
}
