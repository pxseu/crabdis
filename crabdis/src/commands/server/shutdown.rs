use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Gracefully shuts down the server",
    complexity = "O(1)",
    since = "0.1.40"
)]
pub struct Shutdown;

#[async_trait]
impl Handler for Shutdown {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if !args.is_empty() {
            return session
                .respond(&value_error!("ERR wrong number of arguments"), writer)
                .await;
        }

        #[cfg(debug_assertions)]
        log::debug!("Received SHUTDOWN command from session: {session:?}, initiating shutdown");

        // Send shutdown signal
        shutdown::send_shutdown();

        Ok(())
    }
}
