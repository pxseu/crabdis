use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::io::Error as IoError;

use glob::PatternError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(IoError),
    Glob(PatternError),
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<PatternError> for Error {
    fn from(e: PatternError) -> Self {
        Self::Glob(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(inner) => fmt::Display::fmt(&inner, f),
            Self::Glob(inner) => fmt::Display::fmt(&inner, f),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(inner) => Some(inner),
            Self::Glob(inner) => Some(inner),
        }
    }
}

pub trait Context<T, E> {
    fn context<C>(self, ctx: C) -> Result<T>
    where
        C: Display + Send + Sync + 'static;
}

impl<T, E> Context<T, E> for std::result::Result<T, E>
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
                format!("{}: {}", ctx, source)
            } else {
                ctx
            };

            Error::Io(IoError::new(std::io::ErrorKind::Other, msg))
        })
    }
}
