use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -3,
    first_key = 1,
    last_key = -1,
    step = 2,
    summary = "Set multiple keys to multiple values, only if none of the keys exist",
    complexity = "O(N) where N is the number of keys being set",
    since = "0.1.38",
)]
pub struct MSetNx;

#[async_trait]
impl Handler for MSetNx {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() < 2 || !args.len().is_multiple_of(2) {
            return session
                .respond(&value_error!("ERR wrong number of arguments"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;

        // Collect all keys first to check if any exist
        let mut keys_values: Vec<(Arc<str>, Value)> = Vec::with_capacity(args.len() / 2);

        while let Some(key) = args.next() {
            match key {
                Value::String(k) => {
                    // safe to unwrap because we checked the length of the args
                    keys_values.push((k.clone(), args.next_owned().unwrap()));
                }

                _ => {
                    return session
                        .respond(&value_error!("ERR Invalid key"), writer)
                        .await;
                }
            }
        }

        // Check if ANY key exists (atomic check)
        for (key, _) in &keys_values {
            if store.get_unexpired(key).is_ok() {
                // At least one key exists, return 0 without setting any
                return session.respond(&Value::Integer(0), writer).await;
            }
        }

        // None exist, set all keys
        for (key, value) in keys_values {
            store.insert(key, value);
        }

        session.state.notify_change();

        session.respond(&Value::Integer(1), writer).await
    }
}
