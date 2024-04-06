use crate::prelude::*;

pub struct FlushDB;

#[async_trait]
impl CommandTrait for FlushDB {
    fn name(&self) -> &str {
        "FLUSHDB"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        session.state.store.write().await.clear();

        Value::Ok.to_resp2(writer).await
    }
}
