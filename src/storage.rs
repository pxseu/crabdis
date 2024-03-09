use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

#[derive(Clone, Debug)]
pub enum Value {
    String(String),
}

#[derive(Clone, Debug)]
pub struct Store(Arc<RwLock<HashMap<String, Value>>>);

impl Store {
    pub fn new() -> Self {
        Store(Arc::new(RwLock::new(HashMap::new())))
    }

    pub async fn set(&mut self, key: String, value: Value) {
        self.0.write().await.insert(key, value);
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        self.0.read().await.get(key).cloned()
    }

    pub async fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.write().await.remove(key)
    }
}
