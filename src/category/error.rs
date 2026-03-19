//! Error types for category module
//!
//! Defines all error variants for category resolution and model selection.

/// Category-related errors
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum CategoryError {
    /// Unknown category name
    #[error("Unknown category: {0}")]
    UnknownCategory(String),

    /// Category is disabled
    #[error("Category disabled: {0}")]
    CategoryDisabled(String),

    /// Required model not available
    #[error("Model unavailable: category '{category}' requires model '{model}'")]
    ModelUnavailable { category: String, model: String },

    /// Required provider not available
    #[error("Provider unavailable: category '{category}' requires one of {providers:?}")]
    ProviderUnavailable {
        category: String,
        providers: Vec<String>,
    },

    /// Invalid model format
    #[error("Invalid model format: {0}. Expected 'provider/model'")]
    InvalidModelFormat(String),

    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Resolution-related errors
#[derive(Debug, Clone, thiserror::Error)]
#[allow(dead_code)]
pub enum ResolutionError {
    /// No available model found
    #[error("No model available for category")]
    NoModelAvailable,

    /// No connected provider
    #[error("No connected provider available")]
    NoConnectedProvider,

    /// Missing category configuration
    #[error("Missing category configuration")]
    MissingConfig,

    /// Category is disabled
    #[error("Category is disabled")]
    CategoryDisabled,

    /// Fallback chain exhausted
    #[error("Fallback chain exhausted: no models available")]
    FallbackExhausted,
}

impl CategoryError {
    /// Create an unknown category error
    pub fn unknown(name: impl Into<String>) -> Self {
        Self::UnknownCategory(name.into())
    }

    /// Create a disabled category error
    pub fn disabled(name: impl Into<String>) -> Self {
        Self::CategoryDisabled(name.into())
    }

    /// Create a model unavailable error
    pub fn model_unavailable(category: impl Into<String>, model: impl Into<String>) -> Self {
        Self::ModelUnavailable {
            category: category.into(),
            model: model.into(),
        }
    }

    /// Create a provider unavailable error
    pub fn provider_unavailable(category: impl Into<String>, providers: Vec<String>) -> Self {
        Self::ProviderUnavailable {
            category: category.into(),
            providers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_error_messages() {
        let err = CategoryError::unknown("test-category");
        assert_eq!(err.to_string(), "Unknown category: test-category");

        let err = CategoryError::disabled("disabled-cat");
        assert_eq!(err.to_string(), "Category disabled: disabled-cat");

        let err = CategoryError::model_unavailable("deep", "gpt-5.3-codex");
        assert!(err.to_string().contains("deep"));
        assert!(err.to_string().contains("gpt-5.3-codex"));
    }

    #[test]
    fn test_resolution_error_messages() {
        let err = ResolutionError::NoModelAvailable;
        assert!(!err.to_string().is_empty());

        let err = ResolutionError::FallbackExhausted;
        assert!(err.to_string().contains("Fallback chain"));
    }
}
