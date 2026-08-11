use std::net::SocketAddr;

use tokio::sync::RwLock;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use super::auth::{Auth, AuthRef};
use crate::CLI;
use crate::commands::COMMANDS;
use crate::prelude::*;
use crate::storage::{ExpireKey, rdb};

pub static CLIENT_COUNTER: LazyLock<Counter> = LazyLock::new(Counter::new);

pub struct State {
    pub loaded: AtomicBool,
    pub store: Store,
    pub expire_keys: ExpireKey,
    pub rdb_config: RdbConfig,
    pub auto_save_started: AtomicBool,
    pub subscriptions: RwLock<HashMap<Arc<str>, Vec<SessionRef>>>,
    pub sessions: RwLock<HashMap<u64, SessionRef>>,
    pub auth: AuthRef,
    next_session_id: RwLock<u64>,
    available_ids: RwLock<HashSet<u64>>, // For recycling IDs
}

impl State {
    pub fn new(cli: &CLI) -> Arc<Self> {
        // Ensure commands are initialized
        LazyLock::force(&COMMANDS);
        // Initialize client counter to make it faster
        LazyLock::force(&CLIENT_COUNTER);

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
                    SavePoint::new(3600, 1),   // After 1 hour if at least 1 change
                    SavePoint::new(300, 100),  // After 5 mins if at least 100 changes
                    SavePoint::new(60, 10000), // After 1 min if at least 10000 changes
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
            expire_keys: ExpireKey::default(),
            rdb_config,
            auto_save_started: AtomicBool::new(false),
            subscriptions: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
            next_session_id: RwLock::new(1),
            available_ids: RwLock::new(HashSet::new()),
            auth: Auth::new(cli.password.as_deref()),
        };

        let state = Arc::new(state);

        rdb::spawn_load_task(state.clone());
        Self::expire_keys_task(state.clone());

        // Only start auto-save task if RDB persistence is enabled
        if rdb_enabled {
            rdb::spawn_auto_save_task(state.clone());
            state.auto_save_started.store(true, Ordering::Relaxed);
        }

        state
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

    pub async fn new_session(
        self: Arc<Self>,
        socket: SocketAddr,
    ) -> (SessionRef, UnboundedReceiver<Value>) {
        let guard = CLIENT_COUNTER.guard();
        let (tx, rx) = unbounded_channel();
        let session_id = self.get_next_session_id().await;
        let session = Session::new(session_id, socket, self.clone(), tx, guard);
        self.add_session(session.clone()).await;
        (session, rx)
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

    pub async fn subscribe(&self, channel: &str, session_id: u64) {
        // Get the SessionRef from stored sessions
        let sessions_map = self.sessions.read().await;
        let Some(session) = sessions_map.get(&session_id).cloned() else {
            return;
        };
        drop(sessions_map);

        let mut subscriptions = self.subscriptions.write().await;
        let channel_sessions = subscriptions.entry(channel.into()).or_default();

        // Check if session is already subscribed
        if !channel_sessions.iter().any(|s| s.id == session_id) {
            channel_sessions.push(session);
        }
    }

    pub async fn unsubscribe(&self, channel: &str, session_id: u64) {
        let mut subs = self.subscriptions.write().await;

        if let Some(sessions) = subs.get_mut(channel) {
            sessions.retain(|s| s.id != session_id);
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

                // Collect keys to check while holding expire_keys lock briefly
                let keys_to_check: Vec<_> =
                    state.expire_keys.read().await.iter().cloned().collect();

                if keys_to_check.is_empty() {
                    continue;
                }

                let now = tokio::time::Instant::now();
                let mut keys_to_remove = Vec::new();

                // Check expiration times with a single store read lock
                let store = state.store.read().await;
                for key in keys_to_check {
                    if let Some(Value::Expire((_, expire_at))) = store.get(&key) {
                        if now >= *expire_at {
                            // key is expired, mark for removal
                            keys_to_remove.push(key.clone());
                        }
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
