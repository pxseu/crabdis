use crate::prelude::*;

pub struct HGetAll;

#[async_trait]
impl CommandTrait for HGetAll {
    fn name(&self) -> &str {
        "HGETALL"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
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

        let store = session.state.store.read().await;

        match store.get(&key) {
            Some(value @ Value::Hashmap(_)) => session.versioned_response(value, writer).await,

            Some(_) => value_error!("Key is not a hashmap").to_resp2(writer).await,

            None => session.versioned_response(&Value::Nil, writer).await,
        }
    }
}
