use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crabdis_core::parsers::rdb::Rdb;
use tokio::sync::RwLock;

use crate::CLI;
use crate::commands::CommandHandler;
use crate::prelude::*;
use crate::storage::ExpireKey;

pub struct State {
    pub loaded: AtomicBool,
    pub store: Store,
    pub handler: CommandHandler,
    pub expire_keys: ExpireKey,
    pub rdb_config: RdbConfig,
    pub subscriptions: RwLock<HashMap<Arc<str>, Vec<SessionRef>>>,
    pub sessions: RwLock<HashMap<u64, SessionRef>>,
    next_session_id: RwLock<u64>,
    available_ids: RwLock<HashSet<u64>>, // For recycling IDs
}

impl State {
    pub async fn new(cli: &CLI) -> Arc<Self> {
        // Check if user explicitly disabled RDB persistence with --save ""
        let rdb_disabled = cli.save_points.iter().any(|s| s.trim().is_empty());

        // Parse save points
        let save_points: Vec<SavePoint> = cli
            .save_points
            .iter()
            .filter_map(|s| SavePoint::parse(s))
            .collect();

        // Use default save points if none specified (and not explicitly disabled)
        let (save_points, rdb_enabled) = if rdb_disabled {
            log::info!("RDB persistence disabled via --save \"\"");
            (vec![], false)
        } else if save_points.is_empty() {
            (
                vec![
                    SavePoint {
                        seconds: 3600,
                        changes: 1,
                    }, // After 1 hour if at least 1 change
                    SavePoint {
                        seconds: 300,
                        changes: 100,
                    }, // After 5 mins if at least 100 changes
                    SavePoint {
                        seconds: 60,
                        changes: 10000,
                    }, // After 1 min if at least 10000 changes
                ],
                true,
            )
        } else {
            (save_points, true)
        };

        let rdb_config = RdbConfig::new(
            cli.dir.clone(),
            cli.dbfilename.clone(),
            save_points,
            rdb_enabled,
        );

        let state = Self {
            loaded: AtomicBool::new(false),
            store: Store::default(),
            handler: CommandHandler::default(),
            expire_keys: ExpireKey::default(),
            rdb_config,
            subscriptions: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            next_session_id: RwLock::new(1),
            available_ids: RwLock::new(HashSet::new()),
        };

        state.handler.register().await;

        let state = Arc::new(state);

        Self::load_task(state.clone());
        Self::expire_keys_task(state.clone());

        // Only start auto-save task if RDB persistence is enabled
        if rdb_enabled {
            Self::auto_save_task(state.clone());
        }

        state
    }

    /// Loads data from RDB file if it exists.
    pub async fn load_rdb(&self) -> Result<usize> {
        let path = self.rdb_config.rdb_path();

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
        let mut store = self.store.write().await;
        let mut expire_keys = self.expire_keys.write().await;

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
    pub async fn save_rdb(&self) -> Result<()> {
        let path = self.rdb_config.rdb_path();

        // Create temp file
        let temp_path = path.with_extension("rdb.tmp");

        log::info!("Saving RDB to {}", path.display());

        // Convert store to HashMap<Value, Value>
        let store = self.store.read().await;
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

        self.rdb_config.mark_saved();

        log::info!("RDB saved successfully ({} keys)", db.len());

        Ok(())
    }

    /// Starts a background save (for BGSAVE command).
    pub fn bgsave_rdb(state: Arc<Self>) -> Result<()> {
        // Check if already in progress
        if state.rdb_config.bgsave_in_progress.load(Ordering::Relaxed) != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Background save already in progress",
            )
            .into());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        state
            .rdb_config
            .bgsave_in_progress
            .store(now, Ordering::Relaxed);

        tokio::spawn(async move {
            if let Err(e) = state.save_rdb().await {
                log::error!("Background save failed: {e}");
            }
            state
                .rdb_config
                .bgsave_in_progress
                .store(0, Ordering::Relaxed);
        });

        Ok(())
    }

    /// Load task that attempts to load RDB on startup.
    fn load_task(state: Arc<Self>) {
        tokio::spawn(async move {
            match state.load_rdb().await {
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
    fn auto_save_task(state: Arc<Self>) {
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
                    if let Err(e) = Self::bgsave_rdb(state.clone()) {
                        log::error!("Auto-save failed to start: {e}");
                    }
                }
            }
        });
    }

    /// Notifies that data has changed (for auto-save tracking).
    pub fn notify_change(&self) {
        self.rdb_config.increment_changes();
    }

    pub async fn get_next_session_id(&self) -> u64 {
        let mut available = self.available_ids.write().await;
        if let Some(id) = available.iter().next().copied() {
            available.remove(&id);
            id
        } else {
            let mut next_id = self.next_session_id.write().await;
            let id = *next_id;
            // Check for overflow and wrap around to 1 if needed
            *next_id = if id == u64::MAX { 1 } else { id + 1 };
            id
        }
    }

    #[inline]
    pub async fn add_session(&self, session: SessionRef) {
        self.sessions
            .write()
            .await
            .insert(session.id, session.clone());
    }

    pub async fn remove_session(&self, id: u64) {
        // Remove from sessions map
        self.sessions.write().await.remove(&id);

        // Add ID back to available pool
        self.available_ids.write().await.insert(id);

        // Remove from all subscriptions
        let mut subs = self.subscriptions.write().await;
        for sessions in subs.values_mut() {
            sessions.retain(|s| s.id != id);
        }
        // Clean up empty channels
        subs.retain(|_, sessions| !sessions.is_empty());
    }

    pub async fn subscribe(&self, channel: &str, session: SessionRef) {
        let mut sessions = self.subscriptions.write().await;
        let sessions = sessions.entry(channel.into()).or_default();

        // Check if session is already subscribed
        if !sessions.iter().any(|s| s.id == session.id) {
            sessions.push(session);
        }
    }

    pub async fn unsubscribe(&self, channel: &str, session: &SessionRef) {
        let mut subs = self.subscriptions.write().await;

        if let Some(sessions) = subs.get_mut(channel) {
            sessions.retain(|s| s.id != session.id);
            if sessions.is_empty() {
                subs.remove(channel);
            }
        }
    }

    pub async fn publish(&self, channel: &str, message: Value) -> Result<i64> {
        let mut count = 0;
        let subs = self.subscriptions.read().await;

        if let Some(sessions) = subs.get(channel) {
            let pubsub_value = value_push![
                Value::String("message".into()),
                Value::String(channel.into()),
                message.clone()
            ];

            for session in sessions {
                #[cfg(debug_assertions)]
                log::debug!("Sending to session: {session:?}");

                if let Err(e) = session.send(pubsub_value.clone()) {
                    log::error!("Failed to publish to session: {e}");
                    continue;
                }

                count += 1;
            }
        }
        Ok(count)
    }

    fn expire_keys_task(state: Arc<Self>) {
        tokio::spawn(async move {
            // run every 60 seconds
            let mut interval = interval(60).await;

            loop {
                interval.tick().await;

                #[cfg(debug_assertions)]
                log::debug!("Running expire keys task");

                let now = tokio::time::Instant::now();

                // Collect keys to check while holding expire_keys lock briefly
                let keys_to_check: Vec<_> =
                    state.expire_keys.read().await.iter().cloned().collect();

                if keys_to_check.is_empty() {
                    continue;
                }

                let mut keys_to_remove = Vec::new();

                // Check expiration times with a single store read lock
                let store = state.store.read().await;
                for key in keys_to_check {
                    if let Some(Value::Expire((_, expire_at))) = store.get(&key)
                        && now > *expire_at
                    {
                        keys_to_remove.push(key);
                    } else {
                        // key is not expired, remove from expire_keys
                        state.expire_keys.write().await.remove(&key);
                    }
                }
                drop(store);

                #[cfg(debug_assertions)]
                log::debug!("Removing keys: {keys_to_remove:?}");

                if keys_to_remove.is_empty() {
                    continue;
                }

                let mut store = state.store.write().await;
                let mut expire_keys = state.expire_keys.write().await;
                for key in keys_to_remove {
                    store.remove(&key);
                    expire_keys.remove(&key);
                }
            }
        });
    }
}

pub type StateRef = Arc<State>;
