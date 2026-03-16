//! Error types for the util module.
//!
//! This module provides comprehensive error types for handling various
//! utility operations including I/O, parsing, validation, and timeouts.

use thiserror::Error;

/// Main error enum for the util module.
///
/// Aggregates all possible errors that can occur in utility operations.
#[derive(Debug, Error)]
pub enum UtilError {
    /// I/O related errors (file system, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] IoError),

    /// Path-related errors.
    #[error("Path error: {0}")]
    Path(String),

    /// Encoding-related errors.
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Parsing errors.
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    /// Timeout-related errors.
    #[error("Timeout error: {0}")]
    Timeout(#[from] TimeoutError),

    /// Validation errors.
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Process-related errors.
    #[error("Process error: {0}")]
    Process(String),

    /// Glob pattern errors.
    #[error("Glob error: {0}")]
    Glob(String),

    /// JSON serialization/deserialization errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Base64 encoding/decoding errors.
    #[error("Base64 error: {0}")]
    Base64(#[from] base64::DecodeError),

    /// UTF-8 encoding errors.
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Unknown or unexpected errors.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// I/O specific errors.
///
/// Represents errors that occur during file system or stream operations.
#[derive(Debug, Error)]
pub enum IoError {
    /// File or resource not found.
    #[error("File not found: {0}")]
    NotFound(String),

    /// Permission denied when accessing a resource.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Failed to write data.
    #[error("Write failed: {0}")]
    WriteFailed(String),

    /// Failed to read data.
    #[error("Read failed: {0}")]
    ReadFailed(String),
}

/// Parsing errors.
///
/// Represents errors that occur during parsing or conversion operations.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Invalid UTF-8 path encoding.
    #[error("Invalid UTF-8 path: {0}")]
    InvalidUtf8Path(String),

    /// Invalid path format.
    #[error("Invalid path format: {0}")]
    InvalidPathFormat(String),

    /// Invalid JSON data.
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),
}

/// Timeout-related errors.
///
/// Represents errors related to timeouts and waiting operations.
#[derive(Debug, Error)]
pub enum TimeoutError {
    /// Operation elapsed beyond the timeout duration.
    #[error("Operation timed out: {0}")]
    Elapsed(String),

    /// Wait operation timed out.
    #[error("Wait timed out: {0}")]
    WaitElapsed(String),
}

/// Validation errors.
///
/// Represents errors that occur during data validation.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Invalid UUID format.
    #[error("Invalid UUID: {0}")]
    InvalidUuid(String),

    /// Invalid glob pattern.
    #[error("Invalid glob pattern: {0}")]
    InvalidGlob(String),

    /// Value out of allowed range.
    #[error("Value out of range: {0}")]
    OutOfRange(String),
}