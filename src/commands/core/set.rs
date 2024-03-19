use crate::prelude::*;

pub struct Set;

#[async_trait]
impl CommandTrait for Set {
    fn name(&self) -> &str {
        "SET"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        context: ContextRef,
    ) -> Result<()> {
        if args.len() != 2 {
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

        let value = match args.pop_front() {
            Some(value) => value,
            _ => {
                return value_error!("Invalid value").to_resp(writer).await;
            }
        };

        // TODO: add support for options  (https://redis.io/commands/set/)

        context.store.write().await.insert(key, value);

        Value::Ok.to_resp(writer).await
    }
}
