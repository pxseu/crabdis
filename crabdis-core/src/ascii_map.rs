use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::mem::MaybeUninit;

const STACK_KEY_LEN: usize = 256;

#[inline]
fn normalize_ascii_string(mut key: String) -> String {
    if let Some(first_uppercase) = key.bytes().position(|byte| byte.is_ascii_uppercase()) {
        key[first_uppercase..].make_ascii_lowercase();
    }

    key
}

#[inline]
fn normalized_ascii_cow(key: &str) -> Cow<'_, str> {
    key.bytes()
        .position(|byte| byte.is_ascii_uppercase())
        .map_or(Cow::Borrowed(key), |first_uppercase| {
            let mut normalized = key.as_bytes().to_vec();
            normalized[first_uppercase..].make_ascii_lowercase();

            // SAFETY: `normalized` starts as valid UTF-8 from `key`, and ASCII
            // lowercasing only changes ASCII bytes to other ASCII bytes.
            unsafe { Cow::Owned(String::from_utf8_unchecked(normalized)) }
        })
}

#[inline]
fn with_normalized_ascii_key<T>(key: &str, f: impl FnOnce(&str) -> T) -> T {
    match key.bytes().position(|byte| byte.is_ascii_uppercase()) {
        Some(first_uppercase) => {
            let bytes = key.as_bytes();

            if bytes.len() <= STACK_KEY_LEN {
                let len = bytes.len();
                let mut buffer = [MaybeUninit::<u8>::uninit(); STACK_KEY_LEN];

                // SAFETY: copy `len <= STACK_KEY_LEN` source bytes into the
                // uninitialized buffer, then view exactly those `len` bytes as an
                // initialized slice. The source is valid UTF-8 and ASCII lowercasing
                // only rewrites ASCII bytes, so the slice stays valid UTF-8.
                let normalized = unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        buffer.as_mut_ptr().cast::<u8>(),
                        len,
                    );
                    std::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), len)
                };
                normalized[first_uppercase..].make_ascii_lowercase();

                f(unsafe { std::str::from_utf8_unchecked(normalized) })
            } else {
                let mut normalized = bytes.to_vec();
                normalized[first_uppercase..].make_ascii_lowercase();

                // SAFETY: `normalized` starts as valid UTF-8 from `key`, and
                // ASCII lowercasing only changes ASCII bytes to other ASCII bytes.
                let normalized = unsafe { String::from_utf8_unchecked(normalized) };
                f(&normalized)
            }
        }
        None => f(key),
    }
}

/// A normalized ASCII lookup key.
///
/// Already-lowercase ASCII-compatible inputs are borrowed directly. Inputs with
/// ASCII uppercase bytes are copied and lowercased.
#[derive(Debug, Clone)]
pub struct AsciiKey<'a>(Cow<'a, str>);

impl<'a> AsciiKey<'a> {
    /// Creates a normalized ASCII key for lookup.
    #[inline]
    #[must_use]
    pub fn new(key: &'a str) -> Self {
        Self(normalized_ascii_cow(key))
    }

    /// Returns the normalized lookup key.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Returns true when this key borrowed the original input without copying.
    #[inline]
    #[must_use]
    pub const fn is_borrowed(&self) -> bool {
        matches!(self.0, Cow::Borrowed(_))
    }
}

impl AsRef<str> for AsciiKey<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Hash for AsciiKey<'_> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl PartialEq for AsciiKey<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for AsciiKey<'_> {}

/// A case-insensitive ASCII string map backed by lowercase `String` keys.
#[derive(Debug)]
pub struct AsciiMap<V> {
    inner: HashMap<String, V>,
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
    /// The key is stored as an ASCII-lowercase `String`.
    #[inline]
    pub fn insert(&mut self, key: String, value: V) -> Option<V> {
        self.inner.insert(normalize_ascii_string(key), value)
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// Already-normalized lookup keys are borrowed directly. Mixed-case lookup
    /// keys up to 256 bytes use a stack buffer; longer keys allocate.
    #[inline]
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&V> {
        with_normalized_ascii_key(key, |key| self.inner.get(key))
    }

    /// Returns an iterator over the normalized key-value pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.inner.iter().map(|(key, value)| (key.as_str(), value))
    }

    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.inner.values()
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
    fn ascii_key_borrows_normalized_input() {
        let key = AsciiKey::new("get:001");

        assert!(key.is_borrowed());
        assert_eq!(key.as_str(), "get:001");
    }

    #[test]
    fn ascii_key_allocates_mixed_case_input() {
        let key = AsciiKey::new("GeT:001");

        assert!(!key.is_borrowed());
        assert_eq!(key.as_str(), "get:001");
    }

    #[test]
    fn insert_and_get_are_case_insensitive() {
        let mut map = AsciiMap::new();
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
    fn insert_overwrites_with_different_case() {
        let mut map = AsciiMap::new();
        map.insert("GET".to_string(), 1);

        let old = map.insert("get".to_string(), 2);

        assert_eq!(old, Some(1));
        assert_eq!(map.get("GET"), Some(&2));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn iter_returns_normalized_keys() {
        let mut map = AsciiMap::new();
        map.insert("OriginalCase".to_string(), 1);

        let keys: Vec<_> = map.iter().map(|(key, _)| key).collect();

        assert_eq!(keys, ["originalcase"]);
    }

    #[test]
    fn values_returns_values() {
        let mut map = AsciiMap::new();
        map.insert("A".to_string(), 1);
        map.insert("B".to_string(), 2);

        let values: Vec<_> = map.values().copied().collect();

        assert!(values.contains(&1));
        assert!(values.contains(&2));
    }

    #[test]
    fn long_lookup_uses_heap_fallback() {
        let key = "A".repeat(STACK_KEY_LEN + 1);
        let lookup = "a".repeat(STACK_KEY_LEN + 1);
        let mut map = AsciiMap::new();
        map.insert(key, 1);

        assert_eq!(map.get(&lookup), Some(&1));
    }

    #[test]
    fn oversized_lookup_is_rejected() {
        let mut map = AsciiMap::new();
        map.insert("GET".to_string(), 1);

        // Longer than the longest stored key ("get", 3 bytes): guaranteed miss,
        // rejected by the length guard without normalizing the input.
        assert_eq!(map.get(&"a".repeat(1 << 20)), None);
        // A key the same length as the longest stored key still matches.
        assert_eq!(map.get("get"), Some(&1));
    }

    #[test]
    fn empty_map_rejects_any_lookup() {
        let map: AsciiMap<i32> = AsciiMap::new();

        assert_eq!(map.get(""), None);
        assert_eq!(map.get("anything"), None);
    }
}
