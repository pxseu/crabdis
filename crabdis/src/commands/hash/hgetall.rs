use crate::prelude::*;

pub struct HGetAll;

#[async_trait]
impl CommandTrait for HGetAll {
    fn name(&self) -> &'static str {
        "HGETALL"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Returns all fields and values of the hash stored at key",
            complexity: "O(N) where N is the number of fields in the hash",
            since: "0.1.34",
        }
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let store = session.state.store.read().await;

        let value = store.get_inner_unexpired(key)?;

        if !matches!(value, Value::Map(_)) {
            return session
                .respond(&value_error!("Key is not a hashmap"), writer)
                .await;
        }

        session.respond(value, writer).await
    }
}
