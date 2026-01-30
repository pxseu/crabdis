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
