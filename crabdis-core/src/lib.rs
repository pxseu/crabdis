#![forbid(clippy::nursery)]
#![deny(clippy::cargo, clippy::pedantic)]
#![allow(
    clippy::multiple_crate_versions,
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]

pub mod args;
pub mod error;
pub mod parsers;
pub mod prelude;
pub mod value;
