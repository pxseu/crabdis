#![forbid(clippy::nursery)]
#![deny(clippy::cargo, clippy::pedantic)]
#![allow(clippy::multiple_crate_versions)]

pub mod args;
pub mod ascii_map;
pub mod counter;
pub mod eq;
pub mod error;
pub mod parsers;
pub mod prelude;
pub mod shutdown;
pub mod store;
pub mod value;
