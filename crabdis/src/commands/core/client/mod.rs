mod getname;
mod help;
mod list;
mod setname;

use crate::prelude::*;

pub static SUBCOMMANDS: LazyLock<SubcommandRegistry> = LazyLock::new(|| {
    let mut commands = SubcommandRegistry::new("CLIENT");
    commands.register(getname::GetName);
    commands.register(setname::SetName);
    commands.register(list::List);
    commands.register(help::Help);
    commands
});

pub struct Client;

#[async_trait]
impl CommandTrait for Client {
    fn name(&self) -> &'static str {
        "CLIENT"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "A container for client connection commands.",
            complexity: "Depends on subcommand.",
            since: "2.4.0",
        }
    }

    fn subcommands(&self) -> Option<&'static SubcommandRegistry> {
        Some(&SUBCOMMANDS)
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        SUBCOMMANDS.handle(writer, args, session).await
    }
}
