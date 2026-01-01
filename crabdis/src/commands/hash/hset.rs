use crate::prelude::*;

pub struct HSet;

#[async_trait]
impl CommandTrait for HSet {
    fn name(&self) -> &'static str {
        "HSET"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        // HSET key field value [field value ...]
        // so the number of arguments should be at least 3 and odd
        if args.len() < 3 || args.len() % 2 != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let mut store = session.state.store.write().await;

        let mut count = 0;

        while let Some(field) = args.next_owned() {
            // SAFETY: we know that we have a value, so we can unwrap
            let value = args.next_owned().unwrap();

            let fields = store
                .entry(key.clone())
                .or_insert_with(|| Value::Map(HashMap::new()));

            match fields {
                Value::Map(fields) => {
                    fields.insert(field, value);
                }
                _ => {
                    return session
                        .respond(&value_error!("Key is not a hashmap"), writer)
                        .await;
                }
            }

            count += 1;
        }

        session.respond(&Value::Integer(count), writer).await
    }
}
