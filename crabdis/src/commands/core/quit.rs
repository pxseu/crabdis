use crate::prelude::*;

pub struct Quit;

#[async_trait]
impl CommandTrait for Quit {
    fn name(&self) -> &str {
        "QUIT"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if !args.is_empty() {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        // Send OK response before closing
        session.versioned_response(&Value::Ok, writer).await?;

        // Close the connection
        writer.shutdown().await?;

        Ok(())
    }
}
