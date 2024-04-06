use std::sync::Arc;

use crate::commands::CommandHandler;
use crate::prelude::*;
use crate::storage::ExpireKey;

#[derive(Clone, Default)]
pub struct State {
    pub store: Store,
    pub commands: CommandHandler,
    pub expire_keys: ExpireKey,
}

impl State {
    pub async fn new() -> Arc<Self> {
        let mut state = State::default();

        state.commands.register().await;

        let state = Arc::new(state);

        Self::expire_keys_task(state.clone());

        state
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
