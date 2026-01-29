mod core;
mod exp;
mod hash;
mod pubsub;
mod store;

use crate::prelude::*;

pub static COMMANDS: LazyLock<CommandRegistry> = LazyLock::new(|| {
    let mut command_registry = CommandRegistry::new();

    core::register(&mut command_registry);
    exp::register(&mut command_registry);
    hash::register(&mut command_registry);
    pubsub::register(&mut command_registry);
    store::register(&mut command_registry);

    command_registry
});
