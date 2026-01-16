use super::SUBCOMMANDS;
use crate::prelude::*;

pub struct Help;

#[async_trait]
impl SubcommandTrait for Help {
    fn name(&self) -> &'static str {
        "HELP"
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: 2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Show helpful text about subcommands.",
            complexity: "O(1)",
            since: "0.1.36",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let help_text = SUBCOMMANDS.help_text();
        session
            .respond(&Value::String(help_text.into()), writer)
            .await
    }
}
