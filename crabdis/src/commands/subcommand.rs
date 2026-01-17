use std::fmt::Write;

use crate::prelude::*;
use crate::session::Session;

pub struct SubcommandInfo {
    pub arity: i64,
    pub first_key: i64,
    pub last_key: i64,
    pub step: i64,
    pub summary: &'static str,
    pub complexity: &'static str,
    pub since: &'static str,
}

#[async_trait]
pub trait SubcommandTrait {
    fn name(&self) -> &'static str;

    fn info(&self) -> SubcommandInfo;

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()>;
}

/// Registry of subcommands for a parent command.
pub struct SubcommandRegistry {
    parent: &'static str,
    commands: HashMap<String, Box<dyn SubcommandTrait + Send + Sync>>,
    default: Option<Box<dyn SubcommandTrait + Send + Sync>>,
}

impl SubcommandRegistry {
    pub fn new(parent: &'static str) -> Self {
        Self {
            parent,
            commands: HashMap::new(),
            default: None,
        }
    }

    pub fn register<S: SubcommandTrait + Send + Sync + 'static>(&mut self, sub: S) {
        self.commands
            .insert(sub.name().to_uppercase(), Box::new(sub));
    }

    pub fn with_default<S: SubcommandTrait + Send + Sync + 'static>(mut self, handler: S) -> Self {
        self.default = Some(Box::new(handler));
        self
    }

    /// Get a subcommand by name (case-insensitive, zero-allocation lookup).
    #[inline]
    pub fn get(&self, name: &str) -> Option<&(dyn SubcommandTrait + Send + Sync)> {
        self.commands.get(&name.to_uppercase()).map(Box::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &(dyn SubcommandTrait + Send + Sync))> {
        self.commands.iter().map(|(k, v)| (k, v.as_ref()))
    }

    /// Generate help text listing all subcommands.
    pub fn help_text(&self) -> String {
        let mut help = String::new();

        for (name, subcmd) in self.iter() {
            let info = subcmd.info();
            writeln!(help, "{} {}", self.parent, name.as_str()).unwrap();
            writeln!(help, "    {}", info.summary).unwrap();
        }

        help
    }

    pub async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        // Check if there's a subcommand argument
        let Some(sub_name) = args.next_string() else {
            // No subcommand provided
            if let Some(default) = &self.default {
                return default.handle(writer, args, session).await;
            }

            return session
                .respond(
                    &value_error!(
                        "ERR wrong number of arguments for '{}' command",
                        self.parent
                    ),
                    writer,
                )
                .await;
        };

        if let Some(subcmd) = self.get(sub_name) {
            return subcmd.handle(writer, args, session).await;
        }

        // Unknown subcommand
        session
            .respond(
                &value_error!(
                    "ERR unknown subcommand '{sub_name}'. Try {} HELP.",
                    self.parent
                ),
                writer,
            )
            .await
    }
}
