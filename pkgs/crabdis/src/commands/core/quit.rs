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
        _session: SessionRef,
    ) -> Result<()> {
        if !args.is_empty() {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        // Send OK response before closing
        Value::Ok.to_resp2(writer).await?;

        // Close the connection
        writer.shutdown().await?;

        Ok(())
    }
}
