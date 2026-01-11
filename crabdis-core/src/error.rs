use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::io::Error as IoError;

use crate::store::error::StoreError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Store(StoreError),
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<StoreError> for Error {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(inner) => fmt::Display::fmt(&inner, f),
            Self::Store(inner) => fmt::Display::fmt(&inner, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(inner) => Some(inner),
            Self::Store(_) => None,
        }
    }
}

pub trait Context<T> {
    /// # Errors
    ///
    /// Returns the original `Ok(T)` value if successful, or wraps the error
    /// with additional context as an [`Error::Io`] if it fails.
    fn context<C>(self, ctx: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;
}

impl<T, E> Context<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context<C>(self, ctx: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|err| {
            let ctx = ctx.to_string();

            let msg = if let Some(source) = err.source() {
                format!("{ctx}: {source}")
            } else {
                ctx
            };

            Error::Io(IoError::other(msg))
        })
    }
}
