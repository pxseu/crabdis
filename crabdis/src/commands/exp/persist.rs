use crate::prelude::*;

pub struct Persist;

#[async_trait]
impl CommandTrait for Persist {
    fn name(&self) -> &'static str {
        "PERSIST"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Remove the expiration from a key",
            complexity: "O(1)",
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

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let mut store = session.state.store.write().await;

        let result = match store.get_unexpired(&key) {
            Ok(Value::Expire((inner, _))) => {
                let inner_value = inner.as_ref().clone();
                *store.get_mut(&key).unwrap() = inner_value;
                session.state.expire_keys.write().await.remove(&key);
                session.state.notify_change();
                Value::Integer(1)
            }

            _ => Value::Integer(0),
        };

        session.respond(&result, writer).await
    }
}
