use crate::prelude::*;

/// Zero-copy argument parser that iterates over a slice of Values.
/// Avoids cloning the entire args vector for each command.
pub struct Args<'a> {
    slice: &'a [Value],
    pos: usize,
}

#[allow(dead_code)]
impl<'a> Args<'a> {
    #[inline]
    pub fn new(slice: &'a [Value]) -> Self {
        Self { slice, pos: 0 }
    }

    #[inline]
    pub fn next(&mut self) -> Option<&'a Value> {
        if self.pos < self.slice.len() {
            let val = &self.slice[self.pos];
            self.pos += 1;
            Some(val)
        } else {
            None
        }
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
    pub fn len(&self) -> usize {
        self.slice.len() - self.pos
    }

    /// Returns true if no arguments remain.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.pos >= self.slice.len()
    }

    /// Returns an iterator over the remaining arguments.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'a Value> {
        self.slice[self.pos..].iter()
    }

    /// Peeks at the next argument without consuming it.
    #[inline]
    pub fn peek(&self) -> Option<&'a Value> {
        self.slice.get(self.pos)
    }
}

impl std::fmt::Debug for Args<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
