use crate::commands::COMMANDS;
use crate::prelude::*;

/// Default handler for `COMMAND` (no subcommand).
///
/// Returns command info as a map.
pub struct Default;

#[async_trait]
impl Handler for Default {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        COMMANDS
            .get("COMMAND")
            .unwrap()
            .handle(
                writer,
                &mut Args::new(&[Value::String("INFO".into())]),
                session,
            )
            .await
    }
}
