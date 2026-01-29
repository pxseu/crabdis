use std::fmt::Write;

use crate::prelude::*;

#[async_trait::async_trait]
pub trait SubcommandTrait {
    fn name(&self) -> &'static str;

    fn info(&self) -> CommandInfo;

    /// # Errors
    /// - You tell me
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
    commands: AsciiMap<Box<dyn SubcommandTrait + Send + Sync>>,
    default: Option<Box<dyn Handler + Send + Sync>>,
}

impl SubcommandRegistry {
    #[must_use]
    pub fn new(parent: &'static str) -> Self {
        Self {
            parent,
            commands: AsciiMap::new(),
            default: None,
        }
    }

    pub fn register<S: SubcommandTrait + Send + Sync + 'static>(&mut self, sub: S) {
        self.commands
            .insert(sub.name().to_uppercase(), Box::new(sub));
    }

    pub fn register_default<S: Handler + Send + Sync + 'static>(&mut self, sub: S) {
        assert!(
            self.default.is_none(),
            "Default subcommand already registered for {}",
            self.parent
        );

        self.default = Some(Box::new(sub));
    }

    /// Get a subcommand by name (case-insensitive, zero-allocation lookup).
    #[inline]
    pub fn get(&self, name: &str) -> Option<&(dyn SubcommandTrait + Send + Sync)> {
        self.commands.get(name).map(Box::as_ref)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &(dyn SubcommandTrait + Send + Sync))> {
        self.commands.iter().map(|(k, v)| (k, v.as_ref()))
    }

    /// Generate help text listing all subcommands.
    #[must_use]
    pub fn help_text(&self) -> String {
        let mut help = String::new();

        for (name, subcmd) in self.iter() {
            let info = subcmd.info();
            writeln!(help, "{} {}", self.parent, name).unwrap();
            writeln!(help, "    {}", info.summary).unwrap();
        }

        help
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
        // Check if there's a subcommand argument
        let Some(sub_name) = args.next_string() else {
            // No subcommand provided
            if let Some(default) = self.default.as_ref() {
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
