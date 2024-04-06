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
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 2 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let key = match args.pop_front() {
            Some(Value::String(key)) => key,
            Some(_) => {
                return value_error!("Invalid key").to_resp2(writer).await;
            }
            None => {
                return value_error!("Missing key").to_resp2(writer).await;
            }
        };

        let value = match args.pop_front() {
            Some(value) => value,
            _ => {
                return value_error!("Missing value").to_resp2(writer).await;
            }
        };

        // TODO: add support for options  (https://redis.io/commands/set/)

        session.state.store.write().await.insert(key, value);

        Value::Ok.to_resp2(writer).await
    }
}
