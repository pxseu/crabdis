mod default;
mod docs;
mod help;

use crate::prelude::*;

pub static SUBCOMMANDS: LazyLock<SubcommandRegistry> = LazyLock::new(|| {
    let mut commands = SubcommandRegistry::new("COMMAND").with_default(default::Default);
    commands.register(docs::Docs);
    commands.register(help::Help);
    commands
});

pub struct Command;

#[async_trait]
impl CommandTrait for Command {
    fn name(&self) -> &'static str {
        "COMMAND"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "A container for command introspection commands.",
            complexity: "Depends on subcommand.",
            since: "2.8.13",
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
