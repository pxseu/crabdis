use crate::prelude::*;

pub struct Type;

#[async_trait]
impl CommandTrait for Type {
    fn name(&self) -> &str {
        "TYPE"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        let length = args.len();

        if length != 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let key = match args.pop_front() {
            Some(Value::String(key)) => key,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid key"), writer)
                    .await
            }
        };

        log::debug!("TYPE key: {key}");

        let store = session.state.store.read().await;
        let value = store.get(&key);

        let value_type = match value {
            Some(Value::String(_) | Value::Integer(_)) => "string",
            Some(Value::Multi(_)) => "list",
            Some(Value::Set(_)) => "set",
            Some(Value::Map(_)) => "hash",
            None => "none",
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid value type"), writer)
                    .await;
            }
        };

        Value::String(value_type.into()).to_resp2(writer).await
    }
}
