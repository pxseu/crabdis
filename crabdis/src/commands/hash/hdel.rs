use crate::prelude::*;

pub struct HDel;

#[async_trait]
impl CommandTrait for HDel {
    fn name(&self) -> &str {
        "HDEL"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() < 2 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session
                .versioned_response(&value_error!("Invalid key"), writer)
                .await;
        };

        let mut store = session.state.store.write().await;
        let mut count = 0;

        let map = match store.get_mut(&key) {
            Some(Value::Map(map)) => map,
            Some(_) => {
                return session
                    .versioned_response(&value_error!("Key is not a hashmap"), writer)
                    .await;
            }
            // https://redis.io/docs/latest/commands/hdel/
            None => return session.versioned_response(&Value::Integer(0), writer).await,
        };

        for field in args {
            if map.remove(field).is_some() {
                count += 1;
            }
        }

        if map.is_empty() {
            store.remove(&key);
        }

        session
            .versioned_response(&Value::Integer(count), writer)
            .await
    }
}
