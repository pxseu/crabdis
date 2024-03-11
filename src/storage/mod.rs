pub mod value;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

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
        match self.inner.read().await.get(key) {
            Some(Value::Hashmap(_)) => Value::Error(
                "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
            ),
            a => a.cloned().into(),
        }
    }

    pub async fn mset(&mut self, data: HashMap<String, Value>) {
        self.inner.write().await.extend(data);
    }

    pub async fn mget(&self, keys: &[String]) -> Value {
        let inner = self.inner.read().await;

        keys.iter()
            .map(|key| inner.get(key).cloned().into())
            .collect::<VecDeque<_>>()
            .into()
    }

    pub async fn del(&mut self, key: &[String]) -> Value {
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

    pub async fn hset(&mut self, key: String, hashmap: HashMap<String, Value>) -> Value {
        let mut lock = self.inner.write().await;

        match lock
            .entry(key)
            .or_insert_with(|| Value::Hashmap(Default::default()))
        {
            Value::Hashmap(entry) => {
                let len = entry.len() as i64;
                entry.extend(hashmap);
                Value::Integer(len)
            }
            _ => Value::Error(
                "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
            ),
        }
    }

    pub async fn hget(&self, key: &str, field: &str) -> Value {
        self.inner
            .read()
            .await
            .get(key)
            .and_then(|value| match value {
                Value::Hashmap(hashmap) => hashmap.get(field).cloned(),
                _ => Some(Value::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
                )),
            })
            .unwrap_or(Value::Nil)
    }

    pub async fn hgetall(&self, key: &str) -> Value {
        let value = self
            .inner
            .read()
            .await
            .get(key)
            .unwrap_or(&Value::Nil)
            .clone();

        match value {
            Value::Hashmap(_) => value,
            _ => Value::Error(
                "WRONGTYPE Operation against a key holding the wrong kind of value".to_string(),
            ),
        }
    }
}
