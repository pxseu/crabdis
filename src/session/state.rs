use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::commands::CommandHandler;
use crate::prelude::*;
use crate::storage::ExpireKey;

#[derive(Clone)]
pub struct State {
    pub loaded: bool,
    pub store: Store,
    pub handler: CommandHandler,
    pub expire_keys: ExpireKey,
    pub subscriptions: Arc<RwLock<HashMap<String, Vec<SessionRef>>>>,
    pub sessions: Arc<RwLock<HashMap<u64, SessionRef>>>,
    next_session_id: Arc<RwLock<u64>>,
    available_ids: Arc<RwLock<HashSet<u64>>>, // For recycling IDs
}

impl State {
    pub async fn new() -> Arc<Self> {
        let mut state = State {
            loaded: false,
            store: Store::default(),
            handler: CommandHandler::default(),
            expire_keys: ExpireKey::default(),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(RwLock::new(1)),
            available_ids: Arc::new(RwLock::new(HashSet::new())),
        };

        state.handler.register().await;
        state.loaded = true;

        let state = Arc::new(state);
        Self::expire_keys_task(state.clone());

        state
    }

    pub async fn new_session(&self) -> SessionRef {
        let session_id = {
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
        };

        let session = Session::new(session_id, Arc::new(self.clone()));
        self.sessions
            .write()
            .await
            .insert(session_id, session.clone());
        session
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

    pub async fn subscribe(&self, channel: String, session: SessionRef) {
        let mut subs = self.subscriptions.write().await;
        let sessions = subs.entry(channel).or_default();

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
            for session in sessions {
                let version = session.get_proto_version().await;
                let pubsub_value = if version == 3 {
                    // RESP3 push message format
                    let mut push_msg = VecDeque::new();
                    push_msg.push_back(Value::String("message".to_string()));
                    push_msg.push_back(Value::String(channel.to_string()));
                    push_msg.push_back(message.clone());
                    Value::Push(push_msg)
                } else {
                    // RESP2 format
                    let mut multi_msg = VecDeque::new();
                    multi_msg.push_back(Value::String("message".to_string()));
                    multi_msg.push_back(Value::String(channel.to_string()));
                    multi_msg.push_back(message.clone());
                    Value::Multi(multi_msg)
                };

                log::debug!(
                    "Sending to session: {} with protocol version {}",
                    session.id,
                    version
                );
                if let Err(e) = session.send_versioned(pubsub_value).await {
                    log::error!("Failed to publish to session: {}", e);
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
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            // skip the first tick
            interval.tick().await;

            loop {
                interval.tick().await;

                #[cfg(debug_assertions)]
                log::debug!("Running expire keys task");

                let now = tokio::time::Instant::now();

                let mut keys_to_remove = Vec::new();

                for key in state.expire_keys.read().await.iter() {
                    let expire_at = state.store.read().await;
                    let expire_at = match expire_at.get(key) {
                        Some(Value::Expire((_, expire_at))) => expire_at,
                        _ => continue,
                    };

                    if now > *expire_at {
                        keys_to_remove.push(key.clone());
                    }
                }

                #[cfg(debug_assertions)]
                log::debug!("Removing keys: {keys_to_remove:?}");

                for key in keys_to_remove {
                    state.store.write().await.remove(&key);
                    state.expire_keys.write().await.remove(&key);
                }
            }
        });
    }
}

pub type StateRef = Arc<State>;
