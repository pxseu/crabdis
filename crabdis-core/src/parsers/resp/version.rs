use std::fmt::{self, Debug, Display};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub struct AtomicVersion {
    atom: AtomicU8,
}

impl Display for AtomicVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl Debug for AtomicVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
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

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn invalid_version_display_message() {
        let msg = InvalidVersion.to_string();
        assert_eq!(msg, "invalid RESP version, must be one of: 2, 3");
    }

    #[test]
    fn version_as_u8_roundtrip() {
        assert_eq!(Version::RESP2.as_u8(), 2);
        assert_eq!(Version::RESP3.as_u8(), 3);

        assert_eq!(Version::from_u8(2), Version::RESP2);
        assert_eq!(Version::from_u8(3), Version::RESP3);
    }

    #[test]
    fn try_from_u8_valid_and_invalid() {
        assert_eq!(Version::try_from(2u8).unwrap(), Version::RESP2);
        assert_eq!(Version::try_from(3u8).unwrap(), Version::RESP3);

        let err = Version::try_from(0u8).unwrap_err();
        assert_eq!(err.to_string(), InvalidVersion.to_string());

        let err = Version::try_from(4u8).unwrap_err();
        assert_eq!(err.to_string(), InvalidVersion.to_string());
    }

    #[test]
    fn try_from_i64_valid_and_invalid() {
        assert_eq!(Version::try_from(2i64).unwrap(), Version::RESP2);
        assert_eq!(Version::try_from(3i64).unwrap(), Version::RESP3);

        assert!(Version::try_from(-1i64).is_err());
        assert!(Version::try_from(0i64).is_err());
        assert!(Version::try_from(4i64).is_err());
        assert!(Version::try_from(i64::MAX).is_err());
    }

    #[test]
    fn try_from_str_valid_and_invalid() {
        assert_eq!(Version::try_from("2").unwrap(), Version::RESP2);
        assert_eq!(Version::try_from("3").unwrap(), Version::RESP3);

        assert!(Version::try_from("").is_err());
        assert!(Version::try_from("0").is_err());
        assert!(Version::try_from("4").is_err());
        assert!(Version::try_from("resp2").is_err());
        assert!(Version::try_from(" 2").is_err());
        assert!(Version::try_from("2 ").is_err());
    }

    #[test]
    fn from_value_integer_valid_and_invalid() {
        assert_eq!(
            Version::from_value(&Value::Integer(2)).unwrap(),
            Version::RESP2
        );
        assert_eq!(
            Version::from_value(&Value::Integer(3)).unwrap(),
            Version::RESP3
        );

        assert!(Version::from_value(&Value::Integer(0)).is_err());
        assert!(Version::from_value(&Value::Integer(4)).is_err());
        assert!(Version::from_value(&Value::Integer(-1)).is_err());
    }

    #[test]
    fn from_value_string_valid_and_invalid() {
        assert_eq!(
            Version::from_value(&Value::String("2".into())).unwrap(),
            Version::RESP2
        );
        assert_eq!(
            Version::from_value(&Value::String("3".into())).unwrap(),
            Version::RESP3
        );

        assert!(Version::from_value(&Value::String("".into())).is_err());
        assert!(Version::from_value(&Value::String("4".into())).is_err());
        assert!(Version::from_value(&Value::String(" 2".into())).is_err());
    }

    #[test]
    fn from_value_non_integer_non_string_is_invalid() {
        assert!(Version::from_value(&Value::Nil).is_err());
        assert!(Version::from_value(&Value::Ok).is_err());
    }

    #[test]
    fn atomic_version_default_is_resp2() {
        let av = AtomicVersion::new();
        assert_eq!(av.get(), Version::RESP2);

        let av2 = AtomicVersion::default();
        assert_eq!(av2.get(), Version::RESP2);
    }

    #[test]
    fn atomic_version_set_and_get() {
        let av = AtomicVersion::new();
        av.set(Version::RESP3);
        assert_eq!(av.get(), Version::RESP3);

        av.set(Version::RESP2);
        assert_eq!(av.get(), Version::RESP2);
    }
}
