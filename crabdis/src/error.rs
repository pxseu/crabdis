use std::error::Error as StdError;
use std::fmt::{self, Display};
use std::io::Error as IoError;

use crabdis_core::error::Error as CoreError;
use glob::PatternError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// IO error from standard library
    Io(IoError),

    /// Core library error
    Core(CoreError),

    /// Glob pattern error
    Glob(PatternError),

    /// Configuration error
    Config(String),

    /// An error with additional context
    WithContext { message: String, source: Box<Error> },
}

impl Error {
    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Add context to this error
    pub fn context(self, msg: impl Into<String>) -> Self {
        Self::WithContext {
            message: msg.into(),
            source: Box::new(self),
        }
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::Io(e)
    }
}

impl From<CoreError> for Error {
    fn from(e: CoreError) -> Self {
        Self::Core(e)
    }
}

impl From<PatternError> for Error {
    fn from(e: PatternError) -> Self {
        Self::Glob(e)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(inner) => write!(f, "IO error: {inner}"),
            Self::Core(inner) => write!(f, "Core error: {inner}"),
            Self::Glob(inner) => write!(f, "Glob pattern error: {inner}"),
            Self::Config(msg) => write!(f, "Configuration error: {msg}"),
            Self::WithContext { message, source } => {
                write!(f, "{message}: {source}")
            }
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(inner) => Some(inner),
            Self::Core(inner) => Some(inner),
            Self::Glob(inner) => Some(inner),
            Self::WithContext { source, .. } => Some(source.as_ref()),
            Self::Config(_) => None,
        }
    }
}

/// Extension trait for adding context to results
pub trait Context<T> {
    /// Add context to an error
    ///
    /// # Errors
    ///
    /// Returns the original `Ok(T)` value if successful, or wraps the error
    /// with additional context if it fails.
    fn context(self, msg: impl Into<String>) -> Result<T>;

    /// Add context using a lazy closure (only evaluated on error)
    ///
    /// # Errors
    ///
    /// Returns the original `Ok(T)` value if successful, or wraps the error
    /// with additional context if it fails.
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> Context<T> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn context(self, msg: impl Into<String>) -> Result<T> {
        self.map_err(|err| err.into().context(msg))
    }

    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|err| err.into().context(f()))
    }
}
