mod value;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::RwLock;

pub use self::value::*;

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

    pub async fn get(&self, key: &str) -> Value {
        self.inner.read().await.get(key).cloned().into()
    }

    pub async fn mget(&self, keys: &[String]) -> Value {
        let inner = self.inner.read().await;

        keys.iter()
            .map(|key| inner.get(key).cloned().into())
            .collect::<VecDeque<_>>()
            .into()
    }

    pub async fn remove(&mut self, key: &[String]) -> Value {
        let mut count: i64 = 0;

        for k in key {
            if self.inner.write().await.remove(k).is_some() {
                count += 1;
            }
        }

        count.into()
    }

    pub async fn keys(&self) -> Value {
        self.inner
            .read()
            .await
            .keys()
            .cloned()
            .map(Value::String)
            .collect::<VecDeque<_>>()
            .into()
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
