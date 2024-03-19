use crate::prelude::*;

pub struct Get;

#[async_trait]
impl CommandTrait for Get {
    fn name(&self) -> &str {
        "GET"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        context: ContextRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return value_error!("Invalid number of arguments")
                .to_resp(writer)
                .await;
        }

        let key = match args.pop_front() {
            Some(Value::String(key)) => key,
            _ => {
                return value_error!("Invalid key").to_resp(writer).await;
            }
        };

        match context.store.read().await.get(&key) {
            Some(value) => value.to_resp(writer).await,
            None => Value::Nil.to_resp(writer).await,
        }
    }
}
