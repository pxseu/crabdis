use super::subcommand::SubcommandRegistry;
use crate::prelude::*;

#[async_trait::async_trait]
pub trait CommandTrait {
    fn name(&self) -> &'static str;

    fn info(&self) -> CommandInfo;

    /// Return the subcommand registry if this command has subcommands.
    fn subcommands(&self) -> Option<&'static SubcommandRegistry> {
        None
    }

    fn requires_auth(&self) -> bool {
        true
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()>;
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: AsciiMap<Box<dyn CommandTrait + Send + Sync>>,
}

impl CommandRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            commands: AsciiMap::new(),
        }
    }

    pub fn register<S: CommandTrait + Send + Sync + 'static>(&mut self, command: S) {
        // Force init of subcommands
        let _ = command.subcommands();

        self.commands
            .insert(command.name().to_uppercase(), Box::new(command));
    }

    /// Get a command by name (case-insensitive, zero-allocation lookup).
    #[inline]
    pub fn get(&self, command: &str) -> Option<&(dyn CommandTrait + Send + Sync)> {
        self.commands.get(command).map(Box::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &(dyn CommandTrait + Send + Sync))> {
        self.commands.iter().map(|(k, v)| (k, v.as_ref()))
    }

    /// # Errors
    /// - You tell me
    #[inline]
    pub async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let Some(command) = args.next_string() else {
            return session
                .respond(&value_error!("ERR No command was specfied"), writer)
                .await;
        };

        let Some(command) = self.commands.get(command) else {
            return session
                .respond(&value_error!("ERR Unknown command: {command}"), writer)
                .await;
        };

        if command.requires_auth() && !session.is_authenticated() {
            return session
                .respond(&value_error!("NOAUTH Authentication required."), writer)
                .await;
        }

        match command.handle(writer, args, session).await {
            Err(Error::Core(CoreError::Store(store_error))) => {
                session.respond(&store_error.into(), writer).await
            }
            result => result,
        }
    }
}
