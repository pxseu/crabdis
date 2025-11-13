pub mod parser;
pub mod value;

use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

pub type Store = Arc<RwLock<HashMap<Arc<str>, Value>>>;
pub type ExpireKey = Arc<RwLock<HashSet<Arc<str>>>>;
