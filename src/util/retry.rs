//! Retry mechanism with exponential backoff.
//!
//! This module provides utilities for retrying asynchronous operations
//! with configurable exponential backoff and optional jitter.
//!
//! # Examples
//!
//! ## Basic retry with default config
//!
//! ```rust
//! use crate::util::retry::{RetryConfig, retry_with_backoff};
//!
//! async fn fetch_data() -> Result<String, String> {
//!     // Some operation that might fail
//!     Ok("data".to_string())
//! }
//!
//! let config = RetryConfig::default();
//! let result = retry_with_backoff(&config, || async {
//!     fetch_data().await
//! }).await;
//! ```
//!
//! ## Custom retry configuration
//!
//! ```rust
//! use crate::util::retry::RetryConfig;
//!
//! let config = RetryConfig::builder()
//!     .max_retries(5)
//!     .initial_delay_ms(500)
//!     .max_delay_ms(10000)
//!     .backoff_multiplier(1.5)
//!     .use_jitter(true)
//!     .build();
//! ```

use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

/// Configuration for retry operations.
///
/// Controls the number of retries, delay behavior, and backoff strategy.
/// Use `RetryConfig::default()` for sensible defaults or the builder
/// pattern for customization.
///
/// # Default Values
///
/// - `max_retries`: 3
/// - `initial_delay_ms`: 1000
/// - `max_delay_ms`: 30000
/// - `backoff_multiplier`: 2.0
/// - `use_jitter`: true
///
/// # Examples
///
/// ```rust
/// use crate::util::retry::RetryConfig;
///
/// // Default configuration
/// let config = RetryConfig::default();
/// assert_eq!(config.max_retries, 3);
/// assert_eq!(config.initial_delay_ms, 1000);
///
/// // Custom configuration using builder
/// let config = RetryConfig::builder()
///     .max_retries(5)
///     .initial_delay_ms(500)
///     .build();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryConfig {
    /// Maximum number of retry attempts after the initial failure.
    pub max_retries: u32,

    /// Initial delay in milliseconds before the first retry.
    pub initial_delay_ms: u64,

    /// Maximum delay in milliseconds that backoff can reach.
    pub max_delay_ms: u64,

    /// Multiplier applied to delay between retries.
    pub backoff_multiplier: f64,

    /// Whether to add random jitter (±10%) to delays.
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Creates a new retry configuration with default values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::util::retry::RetryConfig;
    ///
    /// let config = RetryConfig::new();
    /// assert_eq!(config.max_retries, 3);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a builder for constructing a custom `RetryConfig`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::util::retry::RetryConfig;
    ///
    /// let config = RetryConfig::builder()
    ///     .max_retries(5)
    ///     .initial_delay_ms(500)
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder() -> RetryConfigBuilder {
        RetryConfigBuilder::default()
    }

    /// Converts the delay in milliseconds to a `Duration`.
    #[must_use]
    fn to_duration(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }
}

/// Builder for constructing a `RetryConfig`.
///
/// Provides a fluent API for customizing retry behavior.
#[derive(Debug, Default)]
pub struct RetryConfigBuilder {
    max_retries: Option<u32>,
    initial_delay_ms: Option<u64>,
    max_delay_ms: Option<u64>,
    backoff_multiplier: Option<f64>,
    use_jitter: Option<bool>,
}

impl RetryConfigBuilder {
    /// Sets the maximum number of retry attempts.
    #[must_use]
    pub fn max_retries(mut self, value: u32) -> Self {
        self.max_retries = Some(value);
        self
    }

    /// Sets the initial delay in milliseconds.
    #[must_use]
    pub fn initial_delay_ms(mut self, value: u64) -> Self {
        self.initial_delay_ms = Some(value);
        self
    }

    /// Sets the maximum delay in milliseconds.
    #[must_use]
    pub fn max_delay_ms(mut self, value: u64) -> Self {
        self.max_delay_ms = Some(value);
        self
    }

    /// Sets the backoff multiplier.
    #[must_use]
    pub fn backoff_multiplier(mut self, value: f64) -> Self {
        self.backoff_multiplier = Some(value);
        self
    }

    /// Sets whether to use jitter.
    #[must_use]
    pub fn use_jitter(mut self, value: bool) -> Self {
        self.use_jitter = Some(value);
        self
    }

    /// Builds the `RetryConfig` with the specified values.
    ///
    /// Uses default values for any fields that weren't explicitly set.
    #[must_use]
    pub fn build(self) -> RetryConfig {
        let default = RetryConfig::default();
        RetryConfig {
            max_retries: self.max_retries.unwrap_or(default.max_retries),
            initial_delay_ms: self.initial_delay_ms.unwrap_or(default.initial_delay_ms),
            max_delay_ms: self.max_delay_ms.unwrap_or(default.max_delay_ms),
            backoff_multiplier: self
                .backoff_multiplier
                .unwrap_or(default.backoff_multiplier),
            use_jitter: self.use_jitter.unwrap_or(default.use_jitter),
        }
    }
}

/// Iterator that generates exponential backoff delays.
///
/// Generates a sequence of delays: initial, initial*multiplier, initial*multiplier^2, ...
/// Respects the maximum delay and stops after max_retries attempts.
/// Optionally adds jitter (random ±10%) to each delay.
///
/// # Examples
///
/// ```rust
/// use crate::util::retry::{RetryConfig, ExponentialBackoff};
///
/// let config = RetryConfig::default();
/// let mut backoff = ExponentialBackoff::new(&config);
///
/// // Get delays
/// let d1 = backoff.next(); // ~1000ms (with optional jitter)
/// let d2 = backoff.next(); // ~2000ms (with optional jitter)
/// let d3 = backoff.next(); // ~4000ms (with optional jitter)
/// ```
#[derive(Debug)]
pub struct ExponentialBackoff {
    config: RetryConfig,
    attempt: u32,
    current_delay_ms: f64,
}

impl ExponentialBackoff {
    /// Creates a new exponential backoff iterator from a configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use crate::util::retry::{RetryConfig, ExponentialBackoff};
    ///
    /// let config = RetryConfig::default();
    /// let backoff = ExponentialBackoff::new(&config);
    /// ```
    #[must_use]
    pub fn new(config: &RetryConfig) -> Self {
        Self {
            config: *config,
            attempt: 0,
            current_delay_ms: config.initial_delay_ms as f64,
        }
    }

    /// Calculates the next delay value with optional jitter.
    fn calculate_delay(&self) -> u64 {
        let delay = if self.config.use_jitter {
            add_jitter(self.current_delay_ms)
        } else {
            self.current_delay_ms as u64
        };

        delay.min(self.config.max_delay_ms)
    }
}

impl Iterator for ExponentialBackoff {
    type Item = u64;

    /// Returns the next delay in milliseconds.
    ///
    /// Returns `None` when max_retries has been reached.
    fn next(&mut self) -> Option<Self::Item> {
        if self.attempt >= self.config.max_retries {
            return None;
        }

        let delay = self.calculate_delay();
        self.attempt += 1;
        self.current_delay_ms *= self.config.backoff_multiplier;

        // Cap the delay at max_delay_ms for future calculations
        if self.current_delay_ms > self.config.max_delay_ms as f64 {
            self.current_delay_ms = self.config.max_delay_ms as f64;
        }

        Some(delay)
    }
}

/// Adds random jitter (±10%) to a delay value.
///
/// This helps prevent thundering herd problems when multiple
/// clients retry simultaneously.
fn add_jitter(delay_ms: f64) -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Simple pseudo-random number generation based on timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();

    // Generate a value between 0.9 and 1.1 (±10%)
    let jitter_factor = 0.9 + ((now % 21) as f64 / 100.0);

    (delay_ms * jitter_factor) as u64
}

/// Retries an asynchronous operation with exponential backoff.
///
/// Executes the provided operation and retries on error using
/// exponential backoff delays. Returns the first successful result
/// or the last error after exhausting all retries.
///
/// # Type Parameters
///
/// - `T`: The success type returned by the operation
/// - `E`: The error type returned by the operation
/// - `F`: The closure type that produces the future
/// - `Fut`: The future type returned by the closure
///
/// # Arguments
///
/// * `config` - Configuration controlling retry behavior
/// * `operation` - A closure that returns a Future
///
/// # Returns
///
/// Returns `Ok(T)` on success or `Err(E)` if all retries are exhausted.
///
/// # Examples
///
/// ```rust
/// use crate::util::retry::{RetryConfig, retry_with_backoff};
///
/// async fn example() -> Result<String, String> {
///     let config = RetryConfig::default();
///     
///     let result = retry_with_backoff(&config, || async {
///         // Simulate an operation that might fail
///         Ok::<_, String>("success".to_string())
///     }).await;
///     
///     result
/// }
/// ```
///
/// With a fallible operation:
///
/// ```rust
/// use crate::util::retry::{RetryConfig, retry_with_backoff};
/// use std::sync::atomic::{AtomicU32, Ordering};
///
/// async fn example() -> Result<String, String> {
///     let config = RetryConfig::default();
///     let attempt = AtomicU32::new(0);
///     
///     let result = retry_with_backoff(&config, || async {
///         let count = attempt.fetch_add(1, Ordering::SeqCst);
///         if count < 2 {
///             Err("not ready yet".to_string())
///         } else {
///             Ok::<_, String>("finally succeeded".to_string())
///         }
///     }).await;
///     
///     result
/// }
/// ```
pub async fn retry_with_backoff<T, E, F, Fut>(
    config: &RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    // Try the operation first
    match operation().await {
        Ok(value) => Ok(value),
        Err(err) => {
            // Create backoff iterator for retries
            let backoff = ExponentialBackoff::new(config);
            let mut last_error = err;

            for delay_ms in backoff {
                // Sleep before retrying
                sleep(RetryConfig::to_duration(delay_ms)).await;

                // Try the operation again
                match operation().await {
                    Ok(value) => return Ok(value),
                    Err(err) => last_error = err,
                }
            }

            // All retries exhausted, return the last error
            Err(last_error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.use_jitter);
    }

    #[test]
    fn test_retry_config_new() {
        let config = RetryConfig::new();
        assert_eq!(config, RetryConfig::default());
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::builder()
            .max_retries(5)
            .initial_delay_ms(500)
            .max_delay_ms(10000)
            .backoff_multiplier(1.5)
            .use_jitter(false)
            .build();

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 10000);
        assert_eq!(config.backoff_multiplier, 1.5);
        assert!(!config.use_jitter);
    }

    #[test]
    fn test_retry_config_builder_partial() {
        let config = RetryConfig::builder().max_retries(10).build();

        assert_eq!(config.max_retries, 10);
        // Other fields should have default values
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.use_jitter);
    }

    #[test]
    fn test_exponential_backoff_without_jitter() {
        let config = RetryConfig::builder()
            .max_retries(3)
            .initial_delay_ms(1000)
            .max_delay_ms(30000)
            .backoff_multiplier(2.0)
            .use_jitter(false)
            .build();

        let mut backoff = ExponentialBackoff::new(&config);

        assert_eq!(backoff.next(), Some(1000));
        assert_eq!(backoff.next(), Some(2000));
        assert_eq!(backoff.next(), Some(4000));
        assert_eq!(backoff.next(), None);
    }

    #[test]
    fn test_exponential_backoff_respects_max_delay() {
        let config = RetryConfig::builder()
            .max_retries(5)
            .initial_delay_ms(1000)
            .max_delay_ms(3500)
            .backoff_multiplier(2.0)
            .use_jitter(false)
            .build();

        let mut backoff = ExponentialBackoff::new(&config);

        assert_eq!(backoff.next(), Some(1000));
        assert_eq!(backoff.next(), Some(2000));
        assert_eq!(backoff.next(), Some(3500)); // Capped at max_delay_ms
        assert_eq!(backoff.next(), Some(3500)); // Stays at max
        assert_eq!(backoff.next(), Some(3500));
        assert_eq!(backoff.next(), None);
    }

    #[test]
    fn test_exponential_backoff_with_jitter() {
        let config = RetryConfig::builder()
            .max_retries(3)
            .initial_delay_ms(1000)
            .use_jitter(true)
            .build();

        let mut backoff = ExponentialBackoff::new(&config);

        // With jitter, values should be approximately in the expected range
        let d1 = backoff.next().unwrap();
        let d2 = backoff.next().unwrap();
        let d3 = backoff.next().unwrap();

        // Jitter is ±10%, so check values are within reasonable bounds
        assert!(
            d1 >= 900 && d1 <= 1100,
            "First delay {} should be within ±10% of 1000",
            d1
        );
        assert!(
            d2 >= 1800 && d2 <= 2200,
            "Second delay {} should be within ±10% of 2000",
            d2
        );
        assert!(
            d3 >= 3600 && d3 <= 4400,
            "Third delay {} should be within ±10% of 4000",
            d3
        );

        assert_eq!(backoff.next(), None);
    }

    #[test]
    fn test_exponential_backoff_zero_retries() {
        let config = RetryConfig::builder().max_retries(0).build();

        let mut backoff = ExponentialBackoff::new(&config);
        assert_eq!(backoff.next(), None);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success_first_try() {
        let config = RetryConfig::default();

        let result: Result<String, String> =
            retry_with_backoff(&config, || async { Ok::<_, String>("success".to_string()) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_retry_with_backoff_eventual_success() {
        let config = RetryConfig::builder()
            .max_retries(3)
            .initial_delay_ms(10) // Short delays for fast tests
            .build();

        let attempt = Arc::new(AtomicU32::new(0));

        let result: Result<String, String> = retry_with_backoff(&config, || {
            let attempt = Arc::clone(&attempt);
            async move {
                let count = attempt.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(format!("attempt {} failed", count))
                } else {
                    Ok("success".to_string())
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(
            attempt.load(Ordering::SeqCst),
            3,
            "Should have attempted 3 times"
        );
    }

    #[tokio::test]
    async fn test_retry_with_backoff_exhausted() {
        let config = RetryConfig::builder()
            .max_retries(2)
            .initial_delay_ms(10)
            .build();

        let attempt = Arc::new(AtomicU32::new(0));

        let result: Result<String, String> = retry_with_backoff(&config, || {
            let attempt = Arc::clone(&attempt);
            async move {
                attempt.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>("always fails".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "always fails");
        // Initial attempt + 2 retries = 3 attempts total
        assert_eq!(attempt.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_zero_retries() {
        let config = RetryConfig::builder().max_retries(0).build();

        let attempt = Arc::new(AtomicU32::new(0));

        let result: Result<String, String> = retry_with_backoff(&config, || {
            let attempt = Arc::clone(&attempt);
            async move {
                attempt.fetch_add(1, Ordering::SeqCst);
                Err::<String, _>("fails".to_string())
            }
        })
        .await;

        assert!(result.is_err());
        // With 0 retries, only 1 attempt should be made
        assert_eq!(attempt.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_with_different_error_types() {
        let config = RetryConfig::builder()
            .max_retries(1)
            .initial_delay_ms(10)
            .build();

        // Test with custom error type
        #[derive(Debug, PartialEq)]
        struct CustomError {
            code: u32,
        }

        let result: Result<String, CustomError> =
            retry_with_backoff(&config, || async { Err(CustomError { code: 42 }) }).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), CustomError { code: 42 });
    }

    #[test]
    fn test_add_jitter_bounds() {
        // Test that jitter always produces values within ±10%
        for _ in 0..100 {
            let base = 1000.0;
            let jittered = add_jitter(base);

            // Jitter should be between 90% and 110% of base
            assert!(
                jittered >= 900,
                "Jittered value {} should be >= 900",
                jittered
            );
            assert!(
                jittered <= 1100,
                "Jittered value {} should be <= 1100",
                jittered
            );
        }
    }

    #[test]
    fn test_exponential_backoff_iterator_trait() {
        let config = RetryConfig::builder()
            .max_retries(2)
            .initial_delay_ms(100)
            .use_jitter(false)
            .build();

        let backoff = ExponentialBackoff::new(&config);
        let delays: Vec<u64> = backoff.collect();

        assert_eq!(delays.len(), 2);
        assert_eq!(delays[0], 100);
        assert_eq!(delays[1], 200);
    }
}
