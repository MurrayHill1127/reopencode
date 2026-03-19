//! Async utility tools for concurrent operations.
//!
//! This module provides async utilities for timeout handling, cancellation,
//! queuing, and synchronization primitives built on top of tokio.
//!
//! # Examples
//!
//! ## Timeout Wrapper
//! ```
//! use std::time::Duration;
//! use crate::util::async_tool::with_timeout;
//!
//! async fn example() {
//!     let result = with_timeout(
//!         Duration::from_millis(100),
//!         async { "completed" }
//!     ).await;
//!     assert!(result.is_ok());
//! }
//! ```
//!
//! ## Cancellation Token
//! ```
//! use crate::util::async_tool::{AbortController, CancellationToken};
//!
//! async fn example() {
//!     let controller = AbortController::new();
//!     let token = controller.token();
//!
//!     // Check if cancelled
//!     if !token.is_cancelled() {
//!         // Do work
//!     }
//!
//!     // Cancel from elsewhere
//!     controller.abort();
//!     assert!(token.is_cancelled());
//! }
//! ```

use std::collections::VecDeque;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
use tokio::time::timeout;

use super::error::TimeoutError;

/// Wraps a future with a timeout.
///
/// If the future completes before the timeout duration elapses, returns the result.
/// Otherwise, returns a `TimeoutError::Elapsed`.
///
/// # Type Parameters
/// - `T`: The success type of the future
/// - `F`: The future type
///
/// # Arguments
/// - `duration`: The maximum time to wait
/// - `future`: The async operation to execute
///
/// # Returns
/// - `Ok(T)` if the future completes in time
/// - `Err(TimeoutError::Elapsed)` if the timeout is reached
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use crate::util::async_tool::with_timeout;
///
/// async fn example() {
///     let result = with_timeout(
///         Duration::from_millis(100),
///         async { 42 }
///     ).await;
///     assert_eq!(result.unwrap(), 42);
/// }
/// ```
pub async fn with_timeout<T, F>(duration: Duration, future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    match timeout(duration, future).await {
        Ok(result) => Ok(result),
        Err(_) => Err(TimeoutError::Elapsed(format!(
            "Operation timed out after {:?}",
            duration
        ))),
    }
}

/// A token that can be used to check if an operation should be cancelled.
///
/// `CancellationToken` is backed by an atomic boolean and can be efficiently
/// checked from multiple threads. When cancelled, it notifies all waiters.
///
/// # Examples
///
/// ```
/// use crate::util::async_tool::CancellationToken;
///
/// async fn do_work(token: CancellationToken) {
///     while !token.is_cancelled() {
///         // Continue working until cancelled
///         tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<CancellationTokenInner>,
}

#[derive(Debug)]
struct CancellationTokenInner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationToken {
    /// Creates a new cancellation token in the non-cancelled state.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::CancellationToken;
    ///
    /// let token = CancellationToken::new();
    /// assert!(!token.is_cancelled());
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CancellationTokenInner {
                cancelled: AtomicBool::new(false),
                notify: Notify::new(),
            }),
        }
    }

    /// Returns true if the token has been cancelled.
    ///
    /// This is a non-blocking operation that reads the atomic state.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::CancellationToken;
    ///
    /// let token = CancellationToken::new();
    /// assert!(!token.is_cancelled());
    /// ```
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Returns a future that resolves when the token is cancelled.
    ///
    /// This is useful for async code that needs to wait for cancellation.
    /// If the token is already cancelled, the future resolves immediately.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::CancellationToken;
    ///
    /// async fn example() {
    ///     let token = CancellationToken::new();
    ///     
    ///     // In another task, cancel after some time
    ///     let token_clone = token.clone();
    ///     tokio::spawn(async move {
    ///         tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    ///         token_clone.cancel();
    ///     });
    ///     
    ///     // Wait for cancellation
    ///     token.cancelled().await;
    /// }
    /// ```
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.inner.notify.notified().await;
    }

    /// Cancels the token, notifying all waiters.
    ///
    /// This is typically called by `AbortController`, but can also be
    /// called directly on cloned tokens.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::CancellationToken;
    ///
    /// async fn example() {
    ///     let token = CancellationToken::new();
    ///     token.cancel();
    ///     assert!(token.is_cancelled());
    /// }
    /// ```
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.notify.notify_waiters();
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A controller for creating and managing cancellation tokens.
///
/// `AbortController` is the primary interface for creating cancellation tokens
/// and signalling cancellation. Each controller owns a single token that
/// can be cloned and distributed to async tasks.
///
/// # Examples
///
/// ```
/// use crate::util::async_tool::AbortController;
///
/// async fn example() {
///     let controller = AbortController::new();
///     let token = controller.token();
///
///     // Start some work
///     let handle = tokio::spawn(async move {
///         while !token.is_cancelled() {
///             tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
///         }
///         "cancelled"
///     });
///
///     // Cancel the operation
///     controller.abort();
///
///     let result = handle.await.unwrap();
///     assert_eq!(result, "cancelled");
/// }
/// ```
#[derive(Debug)]
pub struct AbortController {
    token: CancellationToken,
}

impl AbortController {
    /// Creates a new abort controller with a fresh cancellation token.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AbortController;
    ///
    /// let controller = AbortController::new();
    /// let token = controller.token();
    /// assert!(!token.is_cancelled());
    /// ```
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }

    /// Returns a clone of the cancellation token.
    ///
    /// The token can be distributed to async tasks and will be
    /// cancelled when `abort()` is called on the controller.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AbortController;
    ///
    /// let controller = AbortController::new();
    /// let token = controller.token();
    /// // Pass token to async operations
    /// ```
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Signals cancellation to all token holders.
    ///
    /// After calling `abort()`, all tokens obtained from this controller
    /// will report `is_cancelled() == true` and any `cancelled()` futures
    /// will resolve.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AbortController;
    ///
    /// async fn example() {
    ///     let controller = AbortController::new();
    ///     let token = controller.token();
    ///     
    ///     assert!(!token.is_cancelled());
    ///     controller.abort();
    ///     assert!(token.is_cancelled());
    /// }
    /// ```
    pub fn abort(&self) {
        self.token.cancel();
    }
}

impl Default for AbortController {
    fn default() -> Self {
        Self::new()
    }
}

/// An async-aware queue for producer-consumer patterns.
///
/// `AsyncQueue` provides a multi-producer, multi-consumer queue where
/// consumers can await new items asynchronously. It's backed by a tokio
/// mutex and a notify primitive for efficient blocking.
///
/// # Type Parameters
/// - `T`: The type of items stored in the queue
///
/// # Examples
///
/// ```
/// use crate::util::async_tool::AsyncQueue;
///
/// async fn example() {
///     let queue = AsyncQueue::new();
///     
///     // Producer
///     queue.push(1).await;
///     queue.push(2).await;
///     
///     // Consumer
///     assert_eq!(queue.pop().await, 1);
///     assert_eq!(queue.pop().await, 2);
/// }
/// ```
#[derive(Debug)]
pub struct AsyncQueue<T> {
    inner: Arc<AsyncQueueInner<T>>,
}

#[derive(Debug)]
struct AsyncQueueInner<T> {
    items: Mutex<VecDeque<T>>,
    notify: Notify,
}

impl<T> AsyncQueue<T> {
    /// Creates a new empty async queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncQueue;
    ///
    /// let queue: AsyncQueue<i32> = AsyncQueue::new();
    /// assert!(queue.is_empty().await);
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AsyncQueueInner {
                items: Mutex::new(VecDeque::new()),
                notify: Notify::new(),
            }),
        }
    }

    /// Pushes an item onto the back of the queue.
    ///
    /// Notifies any waiting consumers that a new item is available.
    ///
    /// # Arguments
    /// - `item`: The value to add to the queue
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncQueue;
    ///
    /// async fn example() {
    ///     let queue = AsyncQueue::new();
    ///     queue.push(42).await;
    ///     assert_eq!(queue.len().await, 1);
    /// }
    /// ```
    pub async fn push(&self, item: T) {
        let mut items = self.inner.items.lock().await;
        items.push_back(item);
        self.inner.notify.notify_one();
    }

    /// Pops an item from the front of the queue, waiting if empty.
    ///
    /// This method blocks asynchronously until an item is available.
    /// Multiple consumers will receive items in FIFO order.
    ///
    /// # Returns
    /// The next item from the queue
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncQueue;
    ///
    /// async fn example() {
    ///     let queue = AsyncQueue::new();
    ///     queue.push(1).await;
    ///     queue.push(2).await;
    ///     
    ///     assert_eq!(queue.pop().await, 1);
    ///     assert_eq!(queue.pop().await, 2);
    /// }
    /// ```
    pub async fn pop(&self) -> T {
        loop {
            {
                let mut items = self.inner.items.lock().await;
                if let Some(item) = items.pop_front() {
                    return item;
                }
            }
            // Wait for notification outside the lock
            self.inner.notify.notified().await;
        }
    }

    /// Attempts to pop an item without waiting.
    ///
    /// Returns `Some(item)` if the queue is not empty, or `None` if empty.
    ///
    /// # Returns
    /// - `Some(T)`: The next item if available
    /// - `None`: If the queue is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncQueue;
    ///
    /// async fn example() {
    ///     let queue = AsyncQueue::new();
    ///     assert!(queue.try_pop().await.is_none());
    ///     
    ///     queue.push("hello").await;
    ///     assert_eq!(queue.try_pop().await, Some("hello"));
    /// }
    /// ```
    pub async fn try_pop(&self) -> Option<T> {
        let mut items = self.inner.items.lock().await;
        items.pop_front()
    }

    /// Returns true if the queue contains no items.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncQueue;
    ///
    /// async fn example() {
    ///     let queue = AsyncQueue::new();
    ///     assert!(queue.is_empty().await);
    ///     
    ///     queue.push(1).await;
    ///     assert!(!queue.is_empty().await);
    /// }
    /// ```
    pub async fn is_empty(&self) -> bool {
        let items = self.inner.items.lock().await;
        items.is_empty()
    }

    /// Returns the number of items in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncQueue;
    ///
    /// async fn example() {
    ///     let queue = AsyncQueue::new();
    ///     assert_eq!(queue.len().await, 0);
    ///     
    ///     queue.push('a').await;
    ///     queue.push('b').await;
    ///     assert_eq!(queue.len().await, 2);
    /// }
    /// ```
    pub async fn len(&self) -> usize {
        let items = self.inner.items.lock().await;
        items.len()
    }
}

impl<T> Default for AsyncQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// An async-aware mutual exclusion lock.
///
/// `AsyncLock` provides exclusive access to data across async tasks.
/// It's a wrapper around tokio's `Mutex` with a simplified interface.
///
/// # Examples
///
/// ```
/// use crate::util::async_tool::AsyncLock;
///
/// async fn example() {
///     let lock = AsyncLock::new();
///     
///     // Acquire lock
///     let _guard = lock.acquire().await;
///     // Critical section - exclusive access guaranteed
///     
///     // Lock released when guard is dropped
/// }
/// ```
#[derive(Debug)]
pub struct AsyncLock {
    inner: Arc<Mutex<()>>,
}

/// A guard that holds the async lock.
///
/// When the guard is dropped, the lock is released.
pub type AsyncLockGuard = tokio::sync::OwnedMutexGuard<()>;

impl AsyncLock {
    /// Creates a new async lock in the unlocked state.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncLock;
    ///
    /// let lock = AsyncLock::new();
    /// ```
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
        }
    }

    /// Acquires the lock, waiting if necessary.
    ///
    /// Returns a guard that releases the lock when dropped.
    ///
    /// # Returns
    /// A guard representing the held lock
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncLock;
    ///
    /// async fn example() {
    ///     let lock = AsyncLock::new();
    ///     let guard = lock.acquire().await;
    ///     // Critical section
    ///     drop(guard); // Explicitly release
    /// }
    /// ```
    pub async fn acquire(&self) -> AsyncLockGuard {
        self.inner.clone().lock_owned().await
    }

    /// Attempts to acquire the lock without waiting.
    ///
    /// Returns `Some(guard)` if the lock is available, or `None` if
    /// another task currently holds the lock.
    ///
    /// # Returns
    /// - `Some(AsyncLockGuard)`: If the lock was acquired
    /// - `None`: If the lock is already held
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::util::async_tool::AsyncLock;
    ///
    /// async fn example() {
    ///     let lock = AsyncLock::new();
    ///     
    ///     // First acquire succeeds
    ///     let guard1 = lock.try_acquire().await;
    ///     assert!(guard1.is_some());
    ///     
    ///     // Second acquire fails (already held)
    ///     let guard2 = lock.try_acquire().await;
    ///     assert!(guard2.is_none());
    /// }
    /// ```
    pub async fn try_acquire(&self) -> Option<AsyncLockGuard> {
        self.inner.clone().try_lock_owned().ok()
    }
}

impl Default for AsyncLock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(Duration::from_millis(100), async { 42 }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_failure() {
        let result = with_timeout(Duration::from_millis(10), async {
            sleep(Duration::from_millis(100)).await;
            42
        })
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, TimeoutError::Elapsed(_)));
    }

    #[tokio::test]
    async fn test_cancellation_token_new() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_token_cancel() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_token_clone() {
        let token1 = CancellationToken::new();
        let token2 = token1.clone();

        token1.cancel();
        assert!(token2.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_token_cancelled() {
        let token = CancellationToken::new();

        // Spawn a task that will cancel after a delay
        let token_clone = token.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(50)).await;
            token_clone.cancel();
        });

        // Wait should complete after cancellation
        let start = std::time::Instant::now();
        token.cancelled().await;
        let elapsed = start.elapsed();

        assert!(token.is_cancelled());
        assert!(elapsed >= Duration::from_millis(40)); // Should have waited
    }

    #[tokio::test]
    async fn test_cancellation_token_already_cancelled() {
        let token = CancellationToken::new();
        token.cancel();

        // Should resolve immediately
        token.cancelled().await;
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_abort_controller_new() {
        let controller = AbortController::new();
        let token = controller.token();
        assert!(!token.is_cancelled());
    }

    #[tokio::test]
    async fn test_abort_controller_abort() {
        let controller = AbortController::new();
        let token = controller.token();

        controller.abort();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_abort_controller_multiple_tokens() {
        let controller = AbortController::new();
        let token1 = controller.token();
        let token2 = controller.token();

        controller.abort();
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[tokio::test]
    async fn test_async_queue_new() {
        let queue: AsyncQueue<i32> = AsyncQueue::new();
        assert!(queue.is_empty().await);
        assert_eq!(queue.len().await, 0);
    }

    #[tokio::test]
    async fn test_async_queue_push_pop() {
        let queue = AsyncQueue::new();

        queue.push(1).await;
        queue.push(2).await;
        queue.push(3).await;

        assert_eq!(queue.len().await, 3);
        assert!(!queue.is_empty().await);

        assert_eq!(queue.pop().await, 1);
        assert_eq!(queue.pop().await, 2);
        assert_eq!(queue.pop().await, 3);

        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_async_queue_try_pop() {
        let queue = AsyncQueue::new();

        assert!(queue.try_pop().await.is_none());

        queue.push("hello").await;
        queue.push("world").await;

        assert_eq!(queue.try_pop().await, Some("hello"));
        assert_eq!(queue.try_pop().await, Some("world"));
        assert!(queue.try_pop().await.is_none());
    }

    #[tokio::test]
    async fn test_async_queue_pop_waits() {
        let queue = AsyncQueue::new();

        // Start a consumer that will wait
        let queue_clone = queue.clone();
        let consumer = tokio::spawn(async move { queue_clone.pop().await });

        // Give consumer time to start waiting
        sleep(Duration::from_millis(50)).await;

        // Now push an item
        queue.push(42).await;

        // Consumer should receive it
        let result = consumer.await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_async_queue_clone() {
        let queue1 = AsyncQueue::new();
        let queue2 = queue1.clone();

        queue1.push(1).await;
        assert_eq!(queue2.pop().await, 1);
    }

    #[tokio::test]
    async fn test_async_lock_new() {
        let lock = AsyncLock::new();
        let _guard = lock.acquire().await;
    }

    #[tokio::test]
    async fn test_async_lock_acquire_release() {
        let lock = AsyncLock::new();

        {
            let _guard = lock.acquire().await;
            // Lock held
        }
        // Lock released

        // Can acquire again
        let _guard = lock.acquire().await;
    }

    #[tokio::test]
    async fn test_async_lock_try_acquire() {
        let lock = AsyncLock::new();

        let guard1 = lock.try_acquire().await;
        assert!(guard1.is_some());

        let guard2 = lock.try_acquire().await;
        assert!(guard2.is_none());

        // Drop first guard
        drop(guard1);

        // Can acquire again
        let guard3 = lock.try_acquire().await;
        assert!(guard3.is_some());
    }

    #[tokio::test]
    async fn test_async_lock_multiple_tasks() {
        let lock = AsyncLock::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        for _ in 0..5 {
            let lock = lock.clone();
            let counter = counter.clone();
            let handle = tokio::spawn(async move {
                let _guard = lock.acquire().await;
                // Increment while holding lock
                let val = counter.fetch_add(1, Ordering::SeqCst);
                sleep(Duration::from_millis(10)).await;
                val
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    use std::sync::atomic::AtomicUsize;

    impl Clone for AsyncLock {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }

    impl<T> Clone for AsyncQueue<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
}
