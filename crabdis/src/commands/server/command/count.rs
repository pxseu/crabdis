use crate::commands::COMMANDS;
use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = -2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns the number of registered commands.",
    complexity = "O(1)",
    since = "0.1.38",
)]
pub struct Count;

#[async_trait]
impl Handler for Count {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        session
            .respond(&Value::Integer(COMMANDS.count()), writer)
            .await
    }
}
