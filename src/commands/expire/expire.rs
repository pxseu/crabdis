use crate::prelude::*;

pub struct Expire;

#[async_trait]
impl CommandTrait for Expire {
    fn name(&self) -> &str {
        "EXPIRE"
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

        let seconds = match args.pop_front() {
            Some(Value::Integer(seconds)) => seconds,
            Some(Value::String(seconds)) => seconds.parse::<i64>().unwrap_or(-1),
            Some(_) => {
                return value_error!("Invalid seconds").to_resp2(writer).await;
            }
            None => {
                return value_error!("Missing seconds").to_resp2(writer).await;
            }
        };

        if seconds < 0 {
            return value_error!("Invalid seconds").to_resp2(writer).await;
        }

        let mut store = session.state.store.write().await;

        let value = match store.get_mut(&key) {
            Some(Value::Expire((inner, _))) => inner,
            Some(inner) => inner,
            _ => {
                return value_error!("Key not found").to_resp2(writer).await;
            }
        };

        *value = Value::Expire((
            Box::new(value.to_owned()),
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds as u64),
        ));
        session.state.expire_keys.write().await.insert(key);

        Value::Ok.to_resp2(writer).await
    }
}
