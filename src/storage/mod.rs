pub mod value;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

pub type Store = Arc<RwLock<HashMap<String, Value>>>;
