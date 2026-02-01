use crate::prelude::*;

define_subcommands! {
    parent: "COMMAND",
    registry: SUBCOMMANDS,
    commands: {
        count => Count,
        docs => Docs,
        help => Help,
        info => Info,
    },
    default: default => Default,
}

#[derive(Command)]
#[command(
    arity = 1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns details about all commands.",
    complexity = "O(N) where N is the number of commands",
    since = "0.1.34",
    subcommands = SUBCOMMANDS,
)]
pub struct Command;

#[async_trait]
impl Handler for Command {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        SUBCOMMANDS.handle(writer, args, session).await
    }
}
