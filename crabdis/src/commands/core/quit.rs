use crate::prelude::*;

pub struct Quit;

#[async_trait]
impl CommandTrait for Quit {
    fn name(&self) -> &'static str {
        "QUIT"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if !args.is_empty() {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        // Send OK response before closing
        session.respond(&Value::Ok, writer).await?;

        // Close the connection
        writer.shutdown().await?;

        Ok(())
    }
}
