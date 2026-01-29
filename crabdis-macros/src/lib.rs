#![forbid(clippy::cargo)]
#![deny(clippy::nursery, clippy::pedantic)]

mod command;
mod subcommand;
mod utils;

use proc_macro::TokenStream;

#[proc_macro_derive(Command, attributes(command))]
pub fn derive_command(input: TokenStream) -> TokenStream {
    command::derive_command(input)
}

#[proc_macro_derive(Subcommand, attributes(command))]
pub fn derive_subcommand(input: TokenStream) -> TokenStream {
    subcommand::derive_subcommand(input)
}
