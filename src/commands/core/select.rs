use crate::prelude::*;

pub struct Select;

#[async_trait]
impl CommandTrait for Select {
    fn name(&self) -> &str {
        "Select"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        _session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        Value::Ok.to_resp2(writer).await
    }
}
