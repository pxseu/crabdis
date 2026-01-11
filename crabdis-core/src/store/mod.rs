pub mod error;

use std::collections::hash_map::Entry;
use std::hint::unreachable_unchecked;
use std::sync::Arc;

use crate::prelude::*;

/// Traits for the store
pub trait StoreTraits {
    /// Helper to get mutable access to the inner map, handling Expire wrapper
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::WrongType`] if the key exists but has the wrong
    /// type.
    fn get_entry_map_mut(&mut self, key: &Arc<str>) -> Result<&mut HashMap<Value, Value>>;

    /// Returns the value if it exists and is not expired, otherwise returns an
    /// error
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] if the key does not exist, or
    /// [`StoreError::Expired`] if the key has expired.
    fn get_unexpired(&self, key: &Arc<str>) -> Result<&Value>;

    /// Returns a mutable reference to the value if it exists and is not expired
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] if the key does not exist, or
    /// [`StoreError::Expired`] if the key has expired.
    fn get_unexpired_mut(&mut self, key: &Arc<str>) -> Result<&mut Value>;

    /// Returns the inner (unwrapped) value if it exists and is not expired
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] if the key does not exist, or
    /// [`StoreError::Expired`] if the key has expired.
    fn get_inner_unexpired(&self, key: &Arc<str>) -> Result<&Value>;
}

impl<S: ::std::hash::BuildHasher> StoreTraits for HashMap<Arc<str>, Value, S> {
    fn get_entry_map_mut(&mut self, key: &Arc<str>) -> Result<&mut HashMap<Value, Value>> {
        match self.entry(key.clone()) {
            Entry::Vacant(v) => match v.insert(Value::Map(HashMap::new())) {
                Value::Map(map) => Ok(map),
                _ => unsafe { unreachable_unchecked() }, // we just inserted a map
            },
            Entry::Occupied(o) => match o.into_mut() {
                Value::Map(map) => Ok(map),
                Value::Expire((arc, _)) => match Arc::make_mut(arc) {
                    Value::Map(map) => Ok(map),
                    _ => Err(self::error::StoreError::WrongType.into()),
                },
                _ => Err(self::error::StoreError::WrongType.into()),
            },
        }
    }

    fn get_unexpired(&self, key: &Arc<str>) -> Result<&Value> {
        match self.get(key) {
            None => Err(self::error::StoreError::NotFound.into()),
            Some(value) if value.expired() => Err(self::error::StoreError::Expired.into()),
            Some(value) => Ok(value),
        }
    }

    fn get_unexpired_mut(&mut self, key: &Arc<str>) -> Result<&mut Value> {
        match self.get_mut(key) {
            None => Err(self::error::StoreError::NotFound.into()),
            Some(value) if value.expired() => Err(self::error::StoreError::Expired.into()),
            Some(value) => Ok(value),
        }
    }

    fn get_inner_unexpired(&self, key: &Arc<str>) -> Result<&Value> {
        self.get_unexpired(key).map(Value::inner)
    }
}
