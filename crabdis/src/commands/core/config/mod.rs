mod get;
mod help;
mod set;

use crate::prelude::*;

pub static SUBCOMMANDS: LazyLock<SubcommandRegistry> = LazyLock::new(|| {
    let mut commands = SubcommandRegistry::new("CONFIG");
    commands.register(get::Get);
    commands.register(help::Help);
    commands.register(set::Set);
    commands
});

#[derive(Command)]
#[command(
    arity = -2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "A container for server configuration commands.",
    complexity = "Depends on subcommand.",
    since = "0.1.36",
    subcommands = SUBCOMMANDS,
)]
pub struct Config;

#[async_trait]
impl Handler for Config {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        SUBCOMMANDS.handle(writer, args, session).await
    }
}
