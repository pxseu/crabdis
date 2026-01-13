pub mod core;
pub mod exp;
pub mod hash;
pub mod pubsub;
pub mod store;

use std::sync::LazyLock;

use crabdis_core::error::Error as CoreError;

use crate::prelude::*;

pub struct CommandInfo {
    pub arity: i64,
    pub first_key: i64,
    pub last_key: i64,
    pub step: i64,
    pub summary: &'static str,
    pub complexity: &'static str,
    pub since: &'static str,
}

#[async_trait]
pub trait CommandTrait {
    fn name(&self) -> &'static str;

    fn info(&self) -> CommandInfo;

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()>;
}

pub type CommandMap = HashMap<String, Box<dyn CommandTrait + Send + Sync>>;

pub fn register_command<C>(cmds: &mut CommandMap, command: C)
where
    C: CommandTrait + Send + Sync + 'static,
{
    cmds.insert(command.name().to_uppercase(), Box::new(command));
}

static COMMANDS: LazyLock<CommandMap> = LazyLock::new(|| {
    let mut cmds = HashMap::new();

    core::register(&mut cmds);
    store::register(&mut cmds);
    exp::register(&mut cmds);
    hash::register(&mut cmds);
    pubsub::register(&mut cmds);

    cmds
});

#[inline]
pub fn initialize_commands() {
    LazyLock::force(&COMMANDS);
}

/// Get a command by name (case-insensitive).
#[inline]
pub fn get_command(name: &str) -> Option<&'static (dyn CommandTrait + Send + Sync)> {
    COMMANDS.get(&name.to_uppercase()).map(Box::as_ref)
}

/// Iterate over all registered commands.
#[inline]
pub fn all_commands()
-> impl Iterator<Item = (&'static String, &'static (dyn CommandTrait + Send + Sync))> {
    COMMANDS.iter().map(|(k, v)| (k, v.as_ref()))
}

/// Handle a command by name (case-insensitive).
#[inline]
pub async fn handle_command(
    writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
    args: &mut Args<'_>,
    session: SessionRef,
) -> Result<()> {
    let Some(command) = args.next_string() else {
        #[cfg(debug_assertions)]
        log::debug!("Invalid command: {args:?}");

        return session
            .respond(&value_error!("Invalid command"), writer)
            .await;
    };

    if let Some(cmd) = get_command(command) {
        match cmd.handle_command(writer, args, session.clone()).await {
            Err(Error::Core(CoreError::Store(store_err))) => {
                session.respond(&store_err.into(), writer).await
            }
            any => any,
        }
    } else {
        #[cfg(debug_assertions)]
        log::debug!("Unknown command: {command} {args:?}");

        session
            .respond(&value_error!("ERR Unknown command: {command}"), writer)
            .await
    }
}
