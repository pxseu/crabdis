use crate::prelude::*;

/// Zero-copy argument parser that iterates over a slice of Values.
/// Avoids cloning the entire args vector for each request / response.
///
/// All functions that return an argument will advance the internal position,
/// so the next call will return the next argument. The `peek` function allows
/// you to look at the next argument without advancing the position.
///
/// ```
/// use crabdis_core::args::Args;
/// use crabdis_core::value::Value;
///
/// let array = vec![Value::String("example".into()), Value::Integer(67)];
///
/// let mut args = Args::new(&array);
///
/// assert_eq!(args.len(), 2);
/// assert_eq!(args.next(), Some(&Value::String("example".into())));
/// assert_eq!(args.next(), Some(&Value::Integer(67)));
/// assert_eq!(args.next(), None);
/// assert!(args.is_empty());
/// assert_eq!(array.len(), 2);
/// ```
pub struct Args<'slice> {
    slice: &'slice [Value],
    pos: usize,
}

impl<'slice> Args<'slice> {
    #[inline]
    #[must_use]
    pub const fn new(slice: &'slice [Value]) -> Self {
        Self { slice, pos: 0 }
    }

    #[inline]
    pub fn next_owned(&mut self) -> Option<Value> {
        self.next().cloned()
    }

    /// Returns the next argument if it's a String, otherwise None.
    #[inline]
    pub fn next_string(&mut self) -> Option<&'slice Arc<str>> {
        match self.peek()? {
            Value::String(s) => {
                // advance the position
                self.pos += 1;
                Some(s)
            }

            _ => None,
        }
    }

    /// Returns the next argument if it's a String (cloned), otherwise None.
    #[inline]
    pub fn next_string_owned(&mut self) -> Option<Arc<str>> {
        self.next_string().cloned()
    }

    #[inline]
    pub fn next_integer(&mut self) -> Option<i64> {
        match self.peek()? {
            Value::String(s) => {
                if let Ok(i) = s.parse::<i64>() {
                    self.pos += 1;
                    Some(i)
                } else {
                    None
                }
            }
            Value::Integer(i) => {
                self.pos += 1;
                Some(*i)
            }
            _ => None,
        }
    }

    /// Returns the number of remaining arguments.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        debug_assert!(self.pos <= self.slice.len(), "Args.pos is out of bounds");

        self.slice.len() - self.pos
    }

    /// Returns true if no arguments remain.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.pos >= self.slice.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'slice Value> {
        self.into_iter()
    }

    /// Peeks at the next argument without "consuming" it.
    #[inline]
    #[must_use]
    pub fn peek(&self) -> Option<&'slice Value> {
        self.slice.get(self.pos)
    }
}

impl std::fmt::Debug for Args<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<'slice> Iterator for Args<'slice> {
    type Item = &'slice Value;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.peek().inspect(|_| {
            self.pos += 1;
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.len();
        (remaining, Some(remaining))
    }
}

impl<'slice> IntoIterator for &Args<'slice> {
    type IntoIter = std::slice::Iter<'slice, Value>;
    type Item = &'slice Value;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.slice[self.pos..].iter()
    }
}
