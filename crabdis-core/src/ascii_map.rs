use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// A wrapper for ASCII strings that provides case-insensitive hashing and
/// equality.
///
/// This is designed for zero-allocation lookups: when you have a
/// `HashMap<AsciiKey<String>, V>`, you can look up with `AsciiKey<&str>`
/// without allocating a new `String`.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// use crabdis_core::ascii_map::AsciiKey;
///
/// let mut map: HashMap<AsciiKey<String>, i32> = HashMap::new();
/// map.insert(AsciiKey("GET".to_string()), 1);
///
/// // Zero-allocation lookup with &str
/// assert_eq!(map.get(&AsciiKey("get")), Some(&1));
/// assert_eq!(map.get(&AsciiKey("GET")), Some(&1));
/// assert_eq!(map.get(&AsciiKey("Get")), Some(&1));
/// ```
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct AsciiKey<T: ?Sized>(pub T);

impl<T: AsRef<str> + ?Sized> Hash for AsciiKey<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        for byte in self.0.as_ref().bytes() {
            state.write_u8(byte.to_ascii_uppercase());
        }
        // Hash the length to distinguish "a" from "aa" etc.
        state.write_usize(self.0.as_ref().len());
    }
}

impl<T: AsRef<str> + ?Sized, U: AsRef<str> + ?Sized> PartialEq<AsciiKey<U>> for AsciiKey<T> {
    #[inline]
    fn eq(&self, other: &AsciiKey<U>) -> bool {
        self.0.as_ref().eq_ignore_ascii_case(other.0.as_ref())
    }
}

impl<T: AsRef<str> + ?Sized> Eq for AsciiKey<T> {}

// This is the magic that enables zero-allocation lookups.
// It allows `HashMap<AsciiKey<String>, V>` to be queried with `&AsciiKey<str>`.
impl Borrow<AsciiKey<str>> for AsciiKey<String> {
    #[inline]
    fn borrow(&self) -> &AsciiKey<str> {
        // SAFETY: AsciiKey is #[repr(transparent)] so AsciiKey<String> and
        // AsciiKey<str> have the same memory layout when accessed through
        // references. We're converting &AsciiKey<String> to &AsciiKey<str> by
        // borrowing the inner String as &str. This is safe because:
        // 1. AsciiKey<T> is repr(transparent), meaning it has the same layout as T
        // 2. String can be borrowed as &str
        // 3. The lifetime of the returned reference is tied to self
        unsafe { &*(std::ptr::from_ref::<str>(self.0.as_str()) as *const AsciiKey<str>) }
    }
}

/// A case-insensitive ASCII string map.
///
/// This is a thin wrapper around `HashMap` that uses `AsciiKey` for
/// case-insensitive lookups. Commands like "GET", "get", and "Get" will all map
/// to the same entry.
#[derive(Debug)]
pub struct AsciiMap<V> {
    inner: HashMap<AsciiKey<String>, V>,
}

impl<V> AsciiMap<V> {
    /// Creates a new empty `AsciiMap`.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Inserts a key-value pair into the map.
    ///
    /// The key will be stored as-is, but lookups are case-insensitive.
    #[inline]
    pub fn insert(&mut self, key: String, value: V) -> Option<V> {
        self.inner.insert(AsciiKey(key), value)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The lookup is case-insensitive and does not allocate.
    #[inline]
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&V> {
        // The magic sauce
        let key_ref: &AsciiKey<str> =
            unsafe { &*(std::ptr::from_ref::<str>(key) as *const AsciiKey<str>) };
        self.inner.get(key_ref)
    }

    /// Returns an iterator over the key-value pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.inner.iter().map(|(k, v)| (k.0.as_str(), v))
    }

    /// Returns the number of elements in the map.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the map contains no elements.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<V> Default for AsciiMap<V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_key_case_insensitive_hash() {
        use std::collections::hash_map::DefaultHasher;

        fn hash(s: &str) -> u64 {
            let mut hasher = DefaultHasher::new();
            AsciiKey(s).hash(&mut hasher);
            hasher.finish()
        }

        assert_eq!(hash("GET"), hash("get"));
        assert_eq!(hash("GET"), hash("Get"));
        assert_eq!(hash("GET"), hash("gEt"));
        assert_eq!(hash("HELLO"), hash("hello"));
        assert_eq!(hash("Hello"), hash("hElLo"));

        // Different strings should have different hashes (usually)
        assert_ne!(hash("GET"), hash("SET"));
        assert_ne!(hash("GET"), hash("GETS"));
    }

    #[test]
    fn test_ascii_key_equality() {
        assert_eq!(AsciiKey("GET"), AsciiKey("get"));
        assert_eq!(AsciiKey("GET"), AsciiKey("Get"));
        assert_eq!(AsciiKey("HELLO"), AsciiKey("hello"));

        assert_ne!(AsciiKey("GET"), AsciiKey("SET"));
        assert_ne!(AsciiKey("GET"), AsciiKey("GETS"));
    }

    #[test]
    fn test_ascii_map_insert_and_get() {
        let mut map: AsciiMap<i32> = AsciiMap::new();
        map.insert("GET".to_string(), 1);
        map.insert("SET".to_string(), 2);

        assert_eq!(map.get("GET"), Some(&1));
        assert_eq!(map.get("get"), Some(&1));
        assert_eq!(map.get("Get"), Some(&1));
        assert_eq!(map.get("SET"), Some(&2));
        assert_eq!(map.get("set"), Some(&2));
        assert_eq!(map.get("UNKNOWN"), None);
    }

    #[test]
    fn test_ascii_map_overwrite() {
        let mut map: AsciiMap<i32> = AsciiMap::new();
        map.insert("GET".to_string(), 1);

        // Inserting with different case should overwrite
        let old = map.insert("get".to_string(), 2);
        assert_eq!(old, Some(1));
        assert_eq!(map.get("GET"), Some(&2));
    }

    #[test]
    fn test_ascii_map_iter() {
        let mut map: AsciiMap<i32> = AsciiMap::new();
        map.insert("A".to_string(), 1);
        map.insert("B".to_string(), 2);

        let mut count = 0u8;

        for _ in map.iter() {
            count += 1;
        }

        assert_eq!(count, 2);
    }

    #[test]
    fn test_ascii_map_len_and_empty() {
        let mut map: AsciiMap<i32> = AsciiMap::new();
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);

        map.insert("GET".to_string(), 1);
        assert!(!map.is_empty());
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_zero_allocation_lookup() {
        // This test verifies that we can look up with &str without allocating
        let mut map: AsciiMap<i32> = AsciiMap::new();
        map.insert("COMMAND".to_string(), 42);

        // This lookup should not allocate a String
        let key: &str = "command";
        assert_eq!(map.get(key), Some(&42));
    }
}
