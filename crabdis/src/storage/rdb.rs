//! RDB persistence operations.
//!
//! This module contains all RDB-related functions for loading and saving
//! the database to disk.

use std::collections::HashMap;
use std::sync::Arc;

use crabdis_core::parsers::rdb::Rdb;
use tokio::io::AsyncWriteExt;

use crate::prelude::*;
use crate::session::state::State;

/// Loads data from RDB file if it exists.
pub async fn load_rdb(state: &State) -> Result<usize> {
    let path = state.rdb_config.rdb_path();

    if !path.exists() {
        log::info!(
            "No RDB file found at {}, starting with empty database",
            path.display()
        );
        return Ok(0);
    }

    log::info!("Loading RDB from {}", path.display());

    let file = tokio::fs::File::open(&path).await?;
    let reader = tokio::io::BufReader::new(file);

    let db = Rdb::from(reader).await?;

    // Convert HashMap<Value, Value> to HashMap<Arc<str>, Value>
    let mut store = state.store.write().await;
    let mut expire_keys = state.expire_keys.write().await;

    store.clear();
    expire_keys.clear();

    let mut count = 0;

    for (key, value) in db {
        let key_str: Arc<str> = match key {
            Value::String(s) => s,
            _ => continue,
        };

        // Track expiring keys
        if matches!(value, Value::Expire(_)) {
            expire_keys.insert(key_str.clone());
        }

        store.insert(key_str, value);

        count += 1;
    }

    log::info!("Loaded {count} keys from RDB");

    Ok(count)
}

/// Saves data to RDB file (blocking, for SAVE command).
pub async fn save_rdb(state: &State) -> Result<()> {
    // Try to acquire the save lock (compare-and-swap from 0 to timestamp)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1); // Use 1 as minimum to distinguish from "not in progress"

    if state
        .rdb_config
        .bgsave_in_progress
        .compare_exchange(0, now, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Another save operation is already in progress",
        )
        .into());
    }

    // Ensure we release the lock even if save fails
    let result = save_rdb_inner(state).await;

    state
        .rdb_config
        .bgsave_in_progress
        .store(0, Ordering::Release);

    result
}

/// Inner implementation of `save_rdb` (actual save logic).
async fn save_rdb_inner(state: &State) -> Result<()> {
    let path = state.rdb_config.rdb_path();

    // Create temp file
    let temp_path = path.with_extension("rdb.tmp");

    log::info!("Saving RDB to {}", path.display());

    // Convert store to HashMap<Value, Value>
    let store = state.store.read().await;
    let db: HashMap<Value, Value> = store
        .iter()
        .map(|(k, v)| (Value::String(k.clone()), v.clone()))
        .collect();
    drop(store);

    // Write to temp file
    let file = tokio::fs::File::create(&temp_path).await?;
    let mut writer = tokio::io::BufWriter::new(file);
    Rdb::to(&mut writer, &db).await?;
    writer.shutdown().await?;

    // Rename temp file to actual file (atomic on most systems)
    tokio::fs::rename(&temp_path, &path).await?;

    state.rdb_config.mark_saved();

    log::info!("RDB saved successfully ({} keys)", db.len());

    Ok(())
}

/// Starts a background save (for BGSAVE command).
pub fn bgsave_rdb(state: Arc<State>) -> Result<()> {
    // Try to acquire the save lock (compare-and-swap from 0 to timestamp)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(1); // Use 1 as minimum to distinguish from "not in progress"

    if state
        .rdb_config
        .bgsave_in_progress
        .compare_exchange(0, now, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "A save operation is already in progress",
        )
        .into());
    }

    tokio::spawn(async move {
        if let Err(e) = save_rdb_inner(&state).await {
            log::error!("Background save failed: {e}");
        }
        state
            .rdb_config
            .bgsave_in_progress
            .store(0, Ordering::Release);
    });

    Ok(())
}

/// Load task that attempts to load RDB on startup.
pub fn spawn_load_task(state: Arc<State>) {
    tokio::spawn(async move {
        match load_rdb(&state).await {
            Ok(_) => {
                state.loaded.store(true, Ordering::Relaxed);
            }
            Err(e) => {
                log::error!("Failed to load RDB: {e}");
            }
        }
    });
}

/// Auto-save task that checks save points periodically.
pub fn spawn_auto_save_task(state: Arc<State>) {
    tokio::spawn(async move {
        let mut interval = interval(1).await;

        loop {
            interval.tick().await;

            // Skip if bgsave already in progress
            if state.rdb_config.bgsave_in_progress.load(Ordering::Relaxed) != 0 {
                continue;
            }

            if state.rdb_config.should_save() {
                log::debug!("Auto-save triggered");
                if let Err(e) = bgsave_rdb(state.clone()) {
                    log::error!("Auto-save failed to start: {e}");
                }
            }
        }
    });
}
