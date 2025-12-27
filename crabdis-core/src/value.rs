use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::time::Instant;

#[derive(Clone, Debug)]
pub enum Value {
    Ok,   // only for response
    Pong, // only for response
    Nil,
    Simple(Arc<str>),
    Error(Arc<str>),
    Integer(i64),
    String(Arc<str>),
    Multi(Arc<[Value]>),
    Expire((Arc<Value>, Instant)),
    Map(HashMap<Value, Value>),
    Push(Arc<[Value]>), // For RESP3 push messages (pub/sub)

    // not implemented yet
    Set(HashSet<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ok, Self::Ok) | (Self::Pong, Self::Pong) | (Self::Nil, Self::Nil) => true,
            (Self::Simple(a), Self::Simple(b))
            | (Self::Error(a), Self::Error(b))
            | (Self::String(a), Self::String(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Multi(a), Self::Multi(b)) | (Self::Push(a), Self::Push(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Set(a), Self::Set(b)) => a == b,
            // Compare only inner values; ignore Instant to match Hash
            (Self::Expire(_), Self::Expire(_)) => {
                unreachable!("Expire values should not be compared")
            }
            _ => false,
        }
    }
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Ok => "ok".hash(state),
            Self::Nil => "nil".hash(state),
            Self::Pong => "pong".hash(state),
            Self::Simple(s) | Self::Error(s) | Self::String(s) => s.hash(state),
            Self::Integer(i) => i.hash(state),
            Self::Multi(v) | Self::Push(v) => v.hash(state),
            Self::Expire((v, _)) => v.hash(state),
            Self::Map(_) | Self::Set(_) => unreachable!(),
        }
    }
}

#[macro_export]
macro_rules! value_error {
    ($($arg:tt)*) => {
        $crate::value::Value::Error(format!($($arg)*).into())
    };
}

#[macro_export]
macro_rules! value_multi {
    ($($arg:tt)*) => {
        $crate::value::Value::Multi(vec![$($arg)*].into())
    };
}

#[macro_export]
macro_rules! value_push {
    ($($arg:tt)*) => {
        $crate::value::Value::Push(vec![$($arg)*].into())
    };
}

impl Value {
    #[must_use]
    pub fn expired(&self) -> bool {
        match self {
            Self::Expire((_, expires_at)) => Instant::now() > *expires_at,
            _ => false,
        }
    }

    pub fn set_expire(&mut self, expires_at: Instant) {
        match self {
            Self::Expire((v, _)) => *v = v.clone(),
            _ => *self = Self::Expire((Arc::new(self.clone()), expires_at)),
        }
    }

    #[must_use]
    pub fn is_some(&self) -> bool {
        match self {
            Self::Nil => false,
            _ if Self::expired(self) => false,
            _ => true,
        }
    }

    #[must_use]
    pub fn inner(&self) -> &Self {
        match self {
            Self::Expire((v, _)) => v.inner(),
            _ => self,
        }
    }

    #[must_use]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl From<Option<Self>> for Value {
    fn from(value: Option<Self>) -> Self {
        value.unwrap_or(Self::Nil)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<Arc<str>> for Value {
    fn from(value: Arc<str>) -> Self {
        Self::String(value)
    }
}

impl From<Vec<Self>> for Value {
    fn from(value: Vec<Self>) -> Self {
        Self::Multi(value.into())
    }
}

impl From<&[Self]> for Value {
    fn from(value: &[Self]) -> Self {
        Self::Multi(value.into())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<Option<Arc<str>>> for Value {
    fn from(value: Option<Arc<str>>) -> Self {
        value.map_or(Self::Nil, Self::String)
    }
}
