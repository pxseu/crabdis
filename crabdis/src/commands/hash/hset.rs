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

        // Check if value exists and is expired - if so, remove it first
        if store.get(&key).is_some_and(Value::expired) {
            store.remove(&key);
            session.state.expire_keys.write().await.remove(&key);
        }

        let mut count = 0;

        // Get or create the map, then insert fields
        let map = store.get_entry_map_mut(&key)?;

        while let Some(field) = args.next_owned() {
            let val = args.next_owned().unwrap();
            map.insert(field, val);
            count += 1;
        }

        if count > 0 {
            session.state.notify_change();
        }

        session.respond(&Value::Integer(count), writer).await
    }
}
