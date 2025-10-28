use crate::prelude::*;

pub struct HSet;

#[async_trait]
impl CommandTrait for HSet {
    fn name(&self) -> &str {
        "HSET"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        // HSET key field value [field value ...]
        // so the number of arguments should be at least 3 and odd
        if args.len() < 3 || args.len() % 2 != 1 {
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

        let mut count = 0;

        while let Some(field) = args.pop_front() {
            // SAFETY: we know that we have a field, so we can unwrap
            let value = args.pop_front().unwrap();

            let fields = store
                .entry(key.clone())
                .or_insert_with(|| Value::Map(HashMap::new()));

            match fields {
                Value::Map(fields) => {
                    fields.insert(field, value);
                }
                _ => {
                    return session
                        .versioned_response(&value_error!("Key is not a hashmap"), writer)
                        .await;
                }
            }

            count += 1;
        }

        Value::Integer(count).to_resp2(writer).await
    }
}
