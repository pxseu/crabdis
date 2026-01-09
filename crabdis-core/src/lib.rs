#![forbid(clippy::nursery)]
#![deny(clippy::cargo, clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)]

pub mod args;
pub mod error;
pub mod parsers;
pub mod prelude;
pub mod shutdown;
pub mod value;
