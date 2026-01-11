use crate::prelude::*;

pub struct HDel;

#[async_trait]
impl CommandTrait for HDel {
    fn name(&self) -> &'static str {
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
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let mut store = session.state.store.write().await;

        // Get mutable access to the map
        let map = store.get_entry_map_mut(&key)?;

        let mut count = 0;
        for field in args {
            if map.remove(field).is_some() {
                count += 1;
            }
        }

        // Check if map is now empty and should be removed
        let should_remove = map.is_empty();

        if should_remove {
            store.remove(&key);
            session.state.expire_keys.write().await.remove(&key);
        }

        if count > 0 {
            session.state.notify_change();
        }

        session.respond(&Value::Integer(count), writer).await
    }
}
