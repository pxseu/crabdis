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

    /// Return the subcommand registry if this command has subcommands.
    fn subcommands(&self) -> Option<&'static SubcommandRegistry> {
        None
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()>;
}

pub struct CommandRegistry {
    commands: HashMap<Arc<str>, Box<dyn CommandTrait + Send + Sync>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register<S: CommandTrait + Send + Sync + 'static>(&mut self, command: S) {
        self.commands
            .insert(command.name().into(), Box::new(command));
    }

    pub fn get(&self, command: &Arc<str>) -> Option<&(dyn CommandTrait + Send + Sync)> {
        self.commands.get(command).map(Box::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Arc<str>, &(dyn CommandTrait + Send + Sync))> {
        self.commands.iter().map(|(k, v)| (k, v.as_ref()))
    }

    pub async fn handle(
        &self,
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

        let upper: Arc<str> = command.to_uppercase().into();

        if let Some(cmd) = self.get(&upper) {
            match cmd.handle(writer, args, session.clone()).await {
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
}
