use crate::prelude::*;
use crate::value::Value;

/// Zero-copy argument parser that iterates over a slice of Values.
/// Avoids cloning the entire args vector for each request / response.
pub struct Args<'a> {
    slice: &'a [Value],
    pos: usize,
}

#[allow(dead_code)]
impl<'a> Args<'a> {
    #[inline]
    #[must_use]
    pub const fn new(slice: &'a [Value]) -> Self {
        Self { slice, pos: 0 }
    }

    #[inline]
    pub fn next_owned(&mut self) -> Option<Value> {
        self.next().cloned()
    }

    /// Returns the next argument if it's a String, otherwise None.
    #[inline]
    pub fn next_string(&mut self) -> Option<&'a Arc<str>> {
        match self.next()? {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the next argument if it's a String (cloned), otherwise None.
    #[inline]
    pub fn next_string_owned(&mut self) -> Option<Arc<str>> {
        self.next_string().cloned()
    }

    /// Returns the number of remaining arguments.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.slice.len() - self.pos
    }

    /// Returns true if no arguments remain.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.pos >= self.slice.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'a Value> {
        self.into_iter()
    }

    /// Peeks at the next argument without consuming it.
    #[inline]
    #[must_use]
    pub fn peek(&self) -> Option<&'a Value> {
        self.slice.get(self.pos)
    }
}

impl std::fmt::Debug for Args<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<'a> Iterator for Args<'a> {
    type Item = &'a Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.slice.len() {
            let val = &self.slice[self.pos];
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Args<'_> {}

impl<'a> IntoIterator for &Args<'a> {
    type IntoIter = std::slice::Iter<'a, Value>;
    type Item = &'a Value;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.slice[self.pos..].iter()
    }
}
