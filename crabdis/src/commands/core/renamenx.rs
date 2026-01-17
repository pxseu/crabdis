use crate::prelude::*;

pub struct RenameNx;

#[async_trait]
impl CommandTrait for RenameNx {
    fn name(&self) -> &'static str {
        "RENAMENX"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 3,
            first_key: 1,
            last_key: 2,
            step: 1,
            summary: "Renames a key only if the new key does not exist",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session
                .respond(&value_error!("Invalid argument"), writer)
                .await;
        };

        let Some(new_key) = args.next_string_owned() else {
            return session
                .respond(&value_error!("Invalid argument"), writer)
                .await;
        };

        let mut locked = session.state.store.write().await;

        // Check if source key exists and is not expired
        locked.get_unexpired(&key)?;

        // Check if destination key exists and is not expired
        if locked.get_unexpired(&new_key).is_ok() {
            return session.respond(&Value::Integer(0), writer).await;
        }
        // If destination is expired or doesn't exist, we can overwrite it - clean up
        // expire_keys
        if locked.get(&new_key).is_some() {
            session.state.expire_keys.write().await.remove(&new_key);
        }

        let (_, old_data) = locked.remove_entry(&key).unwrap();

        // Update expire_keys if the source key had a TTL
        let mut expire_keys = session.state.expire_keys.write().await;
        if expire_keys.remove(&key) {
            expire_keys.insert(new_key.clone());
        }
        drop(expire_keys);

        locked.insert(new_key, old_data);

        session.state.notify_change();

        session.respond(&Value::Integer(1), writer).await
    }
}
