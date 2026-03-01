/// Atomic counter that notifies a subscriber when it reaches zero.
///
/// This is used to track the number of connections to the server, and to notify
/// the shutdown handler when all connections have been closed.
///
/// # Example
///
/// Simple usage:
///
/// ```
/// use crabdis_core::counter::Counter;
/// let counter = Counter::new();
/// counter.increment();
/// assert_eq!(counter.get(), 1);
/// counter.decrement();
/// assert_eq!(counter.get(), 0);
/// ```
///
/// Using the RAII guard:
/// ```
/// use crabdis_core::counter::Counter;
/// let counter = Counter::new();
/// {
///     let _guard = counter.guard();
///     assert_eq!(counter.get(), 1);
/// }
/// assert_eq!(counter.get(), 0);
/// ```
pub struct Counter {
    count: std::sync::atomic::AtomicU64,
    notify: tokio::sync::Notify,
}

impl Counter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            count: std::sync::atomic::AtomicU64::new(0),
            notify: tokio::sync::Notify::new(),
        }
    }

    /// Gets the current value of the counter.
    #[inline]
    #[must_use]
    pub fn get(&self) -> u64 {
        self.count.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Creates an RAII guard that increments the counter now and decrements it
    /// when the guard is dropped. This is useful for tracking the lifetime of
    /// a resource (e.g. a connection) without manual increment/decrement calls.
    #[must_use]
    pub fn guard(&self) -> CounterGuard<'_> {
        self.increment();
        CounterGuard { counter: self }
    }

    /// Increments the counter by 1. Panics in debug mode if the counter would
    /// overflow.
    #[inline]
    pub fn increment(&self) {
        let prev = self
            .count
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        debug_assert!(
            prev < u64::MAX,
            "Counter cannot be incremented above u64::MAX"
        );
    }

    /// Decrements the counter by 1. Panics in debug mode if the counter would
    /// underflow. If the counter reaches zero, notifies any waiters.
    #[inline]
    pub fn decrement(&self) {
        let prev = self
            .count
            .fetch_sub(1, std::sync::atomic::Ordering::Release);

        debug_assert!(prev > 0, "Counter cannot be decremented below zero");

        if prev == 1 {
            self.notify.notify_waiters();
        }
    }

    /// Waits until the counter reaches zero. If the counter is already zero,
    /// returns immediately.
    pub async fn wait_for_zero(&self) {
        while self.get() != 0 {
            self.notify.notified().await;
        }
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for a [`Counter`]. Increments the counter on creation (via
/// [`Counter::guard`]) and decrements it on drop.
pub struct CounterGuard<'a> {
    counter: &'a Counter,
}

impl Drop for CounterGuard<'_> {
    fn drop(&mut self) {
        self.counter.decrement();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_counter_starts_at_zero() {
        let counter = Counter::new();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_increment() {
        let counter = Counter::new();
        counter.increment();
        assert_eq!(counter.get(), 1);
        counter.increment();
        assert_eq!(counter.get(), 2);
    }

    #[test]
    fn test_decrement() {
        let counter = Counter::new();
        counter.increment();
        counter.increment();
        counter.decrement();
        assert_eq!(counter.get(), 1);
    }

    #[test]
    fn test_increment_then_decrement_back_to_zero() {
        let counter = Counter::new();
        counter.increment();
        counter.decrement();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_guard_increments_on_creation() {
        let counter = Counter::new();
        let guard = counter.guard();
        assert_eq!(counter.get(), 1);
        drop(guard);
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_multiple_guards() {
        let counter = Counter::new();
        let g1 = counter.guard();
        let g2 = counter.guard();
        let g3 = counter.guard();
        assert_eq!(counter.get(), 3);
        drop(g2);
        assert_eq!(counter.get(), 2);
        drop(g1);
        assert_eq!(counter.get(), 1);
        drop(g3);
        assert_eq!(counter.get(), 0);
    }

    #[tokio::test]
    async fn test_guard_triggers_wait_for_zero() {
        let counter = std::sync::Arc::new(Counter::new());
        let guard = counter.guard();

        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            counter_clone.wait_for_zero().await;
        });

        tokio::task::yield_now().await;

        drop(guard);

        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("timed out waiting for wait_for_zero")
            .expect("task panicked");
    }

    #[tokio::test]
    async fn test_wait_for_zero_returns_immediately_when_already_zero() {
        let counter = Counter::new();
        // Should return immediately without blocking

        tokio::time::timeout(std::time::Duration::from_micros(1), counter.wait_for_zero())
            .await
            .expect("timed out waiting for wait_for_zero");
    }

    #[tokio::test]
    async fn test_wait_for_zero_resolves_after_decrement() {
        let counter = std::sync::Arc::new(Counter::new());
        counter.increment();

        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            counter_clone.wait_for_zero().await;
        });

        // Give the spawned task a moment to start waiting
        tokio::task::yield_now().await;

        counter.decrement();
        // The wait should resolve now that the counter is zero
        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("timed out waiting for wait_for_zero")
            .expect("task panicked");
    }

    #[tokio::test]
    async fn test_wait_for_zero_does_not_resolve_while_nonzero() {
        let counter = std::sync::Arc::new(Counter::new());
        counter.increment();
        counter.increment();

        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            counter_clone.wait_for_zero().await;
        });

        tokio::task::yield_now().await;

        // Decrement once — still at 1, should not resolve
        counter.decrement();

        tokio::pin!(handle);
        let result = tokio::time::timeout(std::time::Duration::from_millis(50), &mut handle).await;
        assert!(
            result.is_err(),
            "wait_for_zero should not resolve while count is nonzero"
        );
    }

    #[tokio::test]
    async fn test_multiple_increments_and_decrements() {
        let counter = std::sync::Arc::new(Counter::new());

        for _ in 0..10 {
            counter.increment();
        }

        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            counter_clone.wait_for_zero().await;
        });

        for _ in 0..10 {
            counter.decrement();
        }

        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("timed out waiting for wait_for_zero")
            .expect("task panicked");
    }

    #[tokio::test]
    async fn test_concurrent_increments_and_decrements() {
        let counter = std::sync::Arc::new(Counter::new());
        let num_tasks = 100;

        // Increment from many concurrent tasks
        let mut handles = Vec::new();
        for _ in 0..num_tasks {
            let c = counter.clone();
            handles.push(tokio::spawn(async move {
                c.increment();
            }));
        }
        for h in handles {
            h.await.expect("task panicked");
        }

        assert_eq!(counter.get(), num_tasks);

        let counter_clone = counter.clone();
        let waiter = tokio::spawn(async move {
            counter_clone.wait_for_zero().await;
        });

        // Decrement from many concurrent tasks
        let mut handles = Vec::new();
        for _ in 0..num_tasks {
            let c = counter.clone();
            handles.push(tokio::spawn(async move {
                c.decrement();
            }));
        }
        for h in handles {
            h.await.expect("task panicked");
        }

        tokio::time::timeout(std::time::Duration::from_secs(1), waiter)
            .await
            .expect("timed out waiting for wait_for_zero")
            .expect("task panicked");
    }
}
