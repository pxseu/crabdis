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
        context: ContextRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return value_error!("Invalid number of arguments")
                .to_resp(writer)
                .await;
        }

        context.store.write().await.clear();

        Value::Ok.to_resp(writer).await
    }
}
