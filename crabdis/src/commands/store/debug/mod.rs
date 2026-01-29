mod help;
mod reload;

use crate::prelude::*;

pub static SUBCOMMANDS: LazyLock<SubcommandRegistry> = LazyLock::new(|| {
    let mut commands = SubcommandRegistry::new("DEBUG");
    commands.register(help::Help);
    commands.register(reload::Reload);
    commands
});

#[derive(Command)]
#[command(
    arity = -2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "A container for debugging commands.",
    complexity = "Depends on subcommand.",
    since = "0.1.34",
    subcommands = SUBCOMMANDS,
)]
pub struct Debug;

#[async_trait]
impl Handler for Debug {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        SUBCOMMANDS.handle(writer, args, session).await
    }
}
