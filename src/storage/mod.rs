pub mod value;

use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

pub type Store = Arc<RwLock<HashMap<String, Value>>>;
pub type ExpireKey = Arc<RwLock<HashSet<String>>>;
