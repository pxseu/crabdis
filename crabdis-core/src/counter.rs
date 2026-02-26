/// Atomic counter that notifies a subscriber when it reaches zero.
/// This is used to track the number of connections to the server, and to notify
/// the shutdown handler when all connections have been closed.
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

    pub fn increment(&self) {
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn decrement(&self) {
        if self.count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst) == 1 {
            self.notify.notify_one();
        }
    }

    pub async fn wait_for_zero(&self) {
        while self.count.load(std::sync::atomic::Ordering::SeqCst) != 0 {
            self.notify.notified().await;
        }
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}
