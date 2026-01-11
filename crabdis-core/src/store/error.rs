use crate::prelude::*;

/// Errors that can occur when accessing the store
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    /// Key doesn't exist in the store
    NotFound,
    /// Key exists but the value has expired
    Expired,
    /// Key exists but has the wrong type (e.g., expected Map, got String)
    WrongType,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Key not found"),
            Self::Expired => write!(f, "Key expired"),
            Self::WrongType => write!(f, "Wrong type"),
        }
    }
}

impl std::error::Error for StoreError {}

impl StoreError {
    /// Converts the error to the appropriate Value response
    #[must_use]
    pub fn to_value(self) -> Value {
        match self {
            Self::NotFound | Self::Expired => Value::Nil,
            Self::WrongType => Value::Error(
                "WRONGTYPE Operation against a key holding the wrong kind of value".into(),
            ),
        }
    }
}

impl From<StoreError> for Value {
    fn from(err: StoreError) -> Self {
        err.to_value()
    }
}
