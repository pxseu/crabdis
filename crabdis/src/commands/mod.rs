macro_rules! define_commands {
    ($( $module:ident => $command:ident ),+ $(,)?) => {
        $( mod $module; )+

        pub fn register(cmds: &mut crate::prelude::CommandRegistry) {
            $( cmds.register($module::$command); )+
        }
    };
}

macro_rules! define_subcommands {
    (
        parent: $parent:expr,
        registry: $registry:ident,
        commands: { $( $module:ident => $command:ident ),+ $(,)? }
        $(, default: $default_mod:ident => $default_handler:ident)?
        $(,)?
    ) => {
        $( mod $module; )+
        $( mod $default_mod; )?

        pub static $registry: crate::prelude::LazyLock<crate::prelude::SubcommandRegistry> =
            crate::prelude::LazyLock::new(|| {
                let mut registry = crate::prelude::SubcommandRegistry::new($parent);
                $(
                    registry.register_default($default_mod::$default_handler);
                )?
                $( registry.register($module::$command); )+
                registry
            });
    };
}

mod connection;
mod expiration;
mod hash;
mod key;
mod persistence;
mod pubsub;
mod server;
mod string;

use crate::prelude::*;

pub static COMMANDS: LazyLock<CommandRegistry> = LazyLock::new(|| {
    let mut command_registry = CommandRegistry::new();

    connection::register(&mut command_registry);
    expiration::register(&mut command_registry);
    hash::register(&mut command_registry);
    key::register(&mut command_registry);
    persistence::register(&mut command_registry);
    pubsub::register(&mut command_registry);
    server::register(&mut command_registry);
    string::register(&mut command_registry);

    command_registry
});
