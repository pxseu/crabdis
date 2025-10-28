use crate::prelude::*;

pub struct Incr;

#[async_trait]
impl CommandTrait for Incr {
    fn name(&self) -> &str {
        "INCR"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
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
                return session
                    .versioned_response(&value_error!("Invalid key"), writer)
                    .await;
            }
            None => {
                return session
                    .versioned_response(&value_error!("Missing key"), writer)
                    .await;
            }
        };

        let mut store = session.state.store.write().await;

        let mut value = match store.get(&key).map(|v| v.inner()) {
            Some(Value::String(s)) => s.parse::<i64>().unwrap_or(0),
            Some(Value::Integer(i)) => *i,
            Some(_) => {
                return session
                    .versioned_response(&value_error!("Invalid value"), writer)
                    .await;
            }
            None => 0,
        };

        value += 1;

        // store it as int at some point
        store.insert(key, Value::String(value.to_string().into()));

        Value::Integer(value).to_resp2(writer).await
    }
}
