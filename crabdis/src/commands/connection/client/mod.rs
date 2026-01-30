use crate::prelude::*;

define_subcommands! {
    parent: "CLIENT",
    registry: SUBCOMMANDS,
    commands: {
        getname => GetName,
        help => Help,
        list => List,
        setname => SetName,
    },
}

#[derive(Command)]
#[command(
    arity = -2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "A container for client connection commands.",
    complexity = "Depends on subcommand.",
    since = "2.4.0",
    subcommands = SUBCOMMANDS,
)]
pub struct Client;

#[async_trait]
impl Handler for Client {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        SUBCOMMANDS.handle(writer, args, session).await
    }
}
