use super::SUBCOMMANDS;
use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = 2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Show helpful text about subcommands.",
    complexity = "O(1)",
    since = "0.1.36"
)]
pub struct Help;

#[async_trait]
impl Handler for Help {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let help_text = SUBCOMMANDS.help_text();
        session
            .respond(&Value::String(help_text.into()), writer)
            .await
    }
}
