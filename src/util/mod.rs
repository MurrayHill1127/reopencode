//! Utility module for ROC (reopencode).
//!
//! This module provides common utility functions and types used across the codebase,
//! including path handling, encoding, string manipulation, ID generation, async tools,
//! retry mechanisms, and formatting utilities.
//!
//! # Modules
//!
//! - [`error`] - Error types for utility operations
//! - [`path`] - Path manipulation utilities using camino
//! - [`encoding`] - Base64, hex encoding and FNV-1a hashing
//! - [`string`] - String manipulation utilities
//! - [`id`] - UUID and ID generation
//! - [`async_tool`] - Async utilities: timeout, cancellation, queue, lock
//! - [`retry`] - Retry mechanism with exponential backoff
//! - [`format`] - Formatting utilities for duration, bytes, numbers

pub mod async_tool;
pub mod encoding;
pub mod error;
pub mod format;
pub mod id;
pub mod path;
pub mod retry;
pub mod string;

// Re-export commonly used types and functions for convenience

// Error types
pub use error::{IoError, ParseError, TimeoutError, UtilError, ValidationError};

// Path utilities
pub use path::{get_directory, get_filename, join_paths, normalize_path, truncate_middle};

// Encoding utilities
pub use encoding::{base64_decode, base64_encode, fnv1a_hash, hex_decode, hex_encode};

// String utilities
pub use string::{camel_to_snake, capitalize, snake_to_camel, truncate, truncate_with_ellipsis};

// ID generation
pub use id::{generate_ascending_id, generate_descending_id, generate_uuid};

// Async utilities
pub use async_tool::{AbortController, AsyncLock, AsyncQueue, CancellationToken, with_timeout};

// Retry mechanism
pub use retry::{ExponentialBackoff, RetryConfig, retry_with_backoff};

// Format utilities
pub use format::{format_bytes, format_duration, format_number, format_relative_time};
