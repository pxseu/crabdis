use crate::prelude::*;

define_subcommands! {
    parent: "CONFIG",
    registry: SUBCOMMANDS,
    commands: {
        get => Get,
        help => Help,
        set => Set,
    },
}

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
