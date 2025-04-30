pub mod core;
pub mod expire;
pub mod hash;
pub mod pubsub;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::prelude::*;

#[async_trait]
pub trait CommandTrait {
    fn name(&self) -> &str;

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()>;
}

macro_rules! register_commands {
    ($handler:expr, $($command:expr),+ $(,)?) => {
        $(
            $handler.register_command($command).await;
        )+
    };
}

#[derive(Clone, Default)]
pub struct CommandHandler {
    commands: Arc<RwLock<HashMap<String, Box<dyn CommandTrait + Send + Sync>>>>,
}

impl CommandHandler {
    pub async fn register(&mut self) {
        register_commands!(
            self,
            core::Command,
            core::Get,
            core::Set,
            core::Del,
            core::MGet,
            core::Ping,
            core::MSet,
            core::Keys,
            core::Hello,
            core::Exists,
            core::FlushDB,
            core::Info,
            core::Incr,
            core::Scan,
            core::Type,
            core::Select,
            core::RenameNx,
        );

        register_commands!(
            self,
            expire::Expire,
            expire::Ttl,
            expire::SetEx,
            expire::PSetEx,
            expire::PTtl,
        );

        register_commands!(self, hash::HSet, hash::HGetAll);

        register_commands!(
            self,
            pubsub::Publish,
            pubsub::Subscribe,
            pubsub::Unsubscribe,
        );
    }

    async fn register_command<C>(&mut self, command: C)
    where
        C: CommandTrait + Send + Sync + 'static,
    {
        self.commands
            .write()
            .await
            .insert(command.name().to_uppercase(), Box::new(command));
    }

    pub async fn handle_command(
        &self,
        writer: &mut WriteHalf<'_>,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        let command = match args.pop_front() {
            Some(Value::String(command)) => command.to_uppercase(),
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid command"), writer)
                    .await
            }
        };

        match self.commands.read().await.get(&command) {
            Some(command) => command.handle_command(writer, args, session).await,
            None => {
                session
                    .versioned_response(&value_error!("Unknown command"), writer)
                    .await
            }
        }
    }
}
