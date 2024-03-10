mod value;

pub use self::value::*;
use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub struct Store {
    inner: Arc<RwLock<HashMap<String, Value>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
        }
    }

    pub async fn set(&mut self, key: String, value: Value) {
        self.inner.write().await.insert(key, value);
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        self.inner.read().await.get(key).cloned()
    }

    pub async fn remove(&mut self, key: &str) -> Option<Value> {
        self.inner.write().await.remove(key)
    }

    pub async fn keys(&self) -> Vec<String> {
        self.inner.read().await.keys().cloned().collect()
    }

    pub async fn exists(&self, key: &str) -> bool {
        self.inner.read().await.contains_key(key)
    }

    pub async fn clear(&mut self) {
        self.inner.write().await.clear();
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}
