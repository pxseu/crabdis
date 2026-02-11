use std::fmt::{self, Display};
use std::hint::unreachable_unchecked;
use std::sync::atomic::{AtomicU8, Ordering};

macro_rules! impl_tryfrom_number {
    ($src:ty) => {
        impl core::convert::TryFrom<$src> for Version {
            type Error = InvalidVersion;

            fn try_from(v: $src) -> Result<Self, Self::Error> {
                match v {
                    2 => Ok(Self::RESP2),
                    3 => Ok(Self::RESP3),
                    _ => Err(InvalidVersion),
                }
            }
        }
    };
}

use crate::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidVersion;

impl Display for InvalidVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid RESP version, must be one of: 2, 3")
    }
}

impl std::error::Error for InvalidVersion {}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Version {
    RESP2 = 2,
    RESP3,
}

impl Version {
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub(crate) fn from_u8(v: u8) -> Self {
        debug_assert!(v == 2 || v == 3);

        match v {
            2 => Self::RESP2,
            3 => Self::RESP3,
            // internal function, can only be 2 or 3
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

impl_tryfrom_number!(u8);
impl_tryfrom_number!(i64);

impl TryFrom<&str> for Version {
    type Error = InvalidVersion;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "2" => Ok(Self::RESP2),
            "3" => Ok(Self::RESP3),
            _ => Err(InvalidVersion),
        }
    }
}

impl Version {
    /// Parses a [`Version`] from a [`Value`].
    ///
    /// # Errors
    ///
    /// Returns [`InvalidVersion`] if the value is not a valid RESP version.
    pub fn from_value(value: &Value) -> Result<Self, InvalidVersion> {
        match value {
            Value::Integer(i) => Self::try_from(*i),
            Value::String(s) => Self::try_from(s.as_ref()),
            _ => Err(InvalidVersion),
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct AtomicVersion {
    atom: AtomicU8,
}

impl AtomicVersion {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            atom: AtomicU8::new(Version::RESP2.as_u8()),
        }
    }

    #[must_use]
    pub fn get(&self) -> Version {
        Version::from_u8(self.atom.load(Ordering::Relaxed))
    }

    pub fn set(&self, version: Version) {
        self.atom.store(version.as_u8(), Ordering::Relaxed);
    }
}

impl Default for AtomicVersion {
    fn default() -> Self {
        Self::new()
    }
}
