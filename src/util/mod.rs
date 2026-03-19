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

// Path utilities

// Encoding utilities

// String utilities

// ID generation
pub use id::generate_uuid;

// Async utilities

// Retry mechanism

// Format utilities
