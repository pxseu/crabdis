use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::RwLock;

use crate::prelude::*;

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
    pub enabled: bool,
    /// Directory for RDB file.
    pub dir: PathBuf,
    /// RDB filename.
    pub dbfilename: String,
    /// Save points for auto-save.
    pub save_points: Vec<SavePoint>,
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
            enabled,
            dir,
            dbfilename,
            save_points,
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

    /// Returns the full path to the RDB file.
    #[must_use]
    pub fn rdb_path(&self) -> PathBuf {
        self.dir.join(&self.dbfilename)
    }

    /// Increments the change counter.
    pub fn increment_changes(&self) {
        self.changes_since_save.fetch_add(1, Ordering::Relaxed);
    }

    /// Resets the change counter and updates last save time.
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
    pub fn should_save(&self) -> bool {
        // Never auto-save if disabled
        if !self.enabled {
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

        for sp in &self.save_points {
            if elapsed >= sp.seconds && changes >= sp.changes {
                return true;
            }
        }

        false
    }
}
