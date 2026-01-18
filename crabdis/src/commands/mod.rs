pub mod command;
mod core;
mod exp;
mod hash;
mod pubsub;
mod store;
pub mod subcommand;

pub use command::{CommandInfo, CommandRegistry, CommandTrait};
pub use subcommand::{SubcommandInfo, SubcommandRegistry, SubcommandTrait};

use crate::prelude::*;

pub static COMMANDS: LazyLock<CommandRegistry> = LazyLock::new(|| {
    let mut command_registry = CommandRegistry::new();

    core::register(&mut command_registry);
    store::register(&mut command_registry);
    exp::register(&mut command_registry);
    hash::register(&mut command_registry);
    pubsub::register(&mut command_registry);

    command_registry
});
