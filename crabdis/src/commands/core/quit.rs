use crate::prelude::*;

pub struct Quit;

#[async_trait]
impl CommandTrait for Quit {
    fn name(&self) -> &'static str {
        "QUIT"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Close the connection",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
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
