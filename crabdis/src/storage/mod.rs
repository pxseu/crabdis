use std::path::PathBuf;
use std::sync::atomic::AtomicU64;

use tokio::sync::RwLock;

use crate::prelude::*;

pub mod rdb;

pub type Store = Arc<RwLock<HashMap<Arc<str>, Value>>>;
pub type ExpireKey = Arc<RwLock<HashSet<Arc<str>>>>;

/// A save point configuration (seconds, changes).
#[derive(Debug, Clone)]
pub struct SavePoint {
    pub seconds: u64,
    pub changes: u64,
}

impl SavePoint {
    /// Parses a save point from a string like "900 1".
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() == 2 {
            let seconds = parts[0].parse().ok()?;
            let changes = parts[1].parse().ok()?;
            Some(Self { seconds, changes })
        } else {
            None
        }
    }
}

/// RDB configuration and state.
#[derive(Debug)]
pub struct RdbConfig {
    /// Whether RDB persistence is enabled.
    pub enabled: AtomicBool,
    /// Directory for RDB file.
    pub dir: RwLock<PathBuf>,
    /// RDB filename.
    pub dbfilename: RwLock<String>,
    /// Save points for auto-save.
    pub save_points: RwLock<Vec<SavePoint>>,
    /// Number of changes since last save.
    pub changes_since_save: AtomicU64,
    /// Last save timestamp.
    pub last_save_time: AtomicU64,
    /// Whether a background save is in progress.
    pub bgsave_in_progress: AtomicU64, // 0 = not in progress, timestamp = in progress since
}

impl RdbConfig {
    /// Creates a new RDB configuration.
    #[must_use]
    pub fn new(
        dir: PathBuf,
        dbfilename: String,
        save_points: Vec<SavePoint>,
        enabled: bool,
    ) -> Self {
        Self {
            enabled: AtomicBool::new(enabled),
            dir: RwLock::new(dir),
            dbfilename: RwLock::new(dbfilename),
            save_points: RwLock::new(save_points),
            changes_since_save: AtomicU64::new(0),
            last_save_time: AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
            bgsave_in_progress: AtomicU64::new(0),
        }
    }

    #[inline]
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    #[inline]
    #[must_use]
    pub fn is_bgsave_in_progress(&self) -> bool {
        self.bgsave_in_progress.load(Ordering::Relaxed) != 0
    }

    #[must_use]
    pub async fn rdb_path(&self) -> PathBuf {
        let dir = self.dir.read().await.clone();
        let dbfilename = self.dbfilename.read().await.clone();
        dir.join(dbfilename)
    }

    #[inline]
    pub fn increment_changes(&self) {
        self.changes_since_save.fetch_add(1, Ordering::Relaxed);
    }

    pub fn mark_saved(&self) {
        self.changes_since_save.store(0, Ordering::Relaxed);
        self.last_save_time.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            Ordering::Relaxed,
        );
    }

    /// Checks if any save point conditions are met.
    #[must_use]
    pub async fn should_save(&self) -> bool {
        // Never auto-save if disabled
        if !self.is_enabled() {
            return false;
        }

        let changes = self.changes_since_save.load(Ordering::Relaxed);
        if changes == 0 {
            return false;
        }

        let last_save = self.last_save_time.load(Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let elapsed = now.saturating_sub(last_save);

        let save_points = self.save_points.read().await;
        for sp in save_points.iter() {
            if elapsed >= sp.seconds && changes >= sp.changes {
                return true;
            }
        }

        false
    }
}
