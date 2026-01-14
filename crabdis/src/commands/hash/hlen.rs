use crate::prelude::*;

pub struct HLen;

#[async_trait]
impl CommandTrait for HLen {
    fn name(&self) -> &'static str {
        "HLEN"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Returns the number of fields in a hash",
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

        let Some(key) = args.next_string() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let store = session.state.store.read().await;

        match store.get_inner_unexpired(key) {
            Ok(Value::Map(map)) => {
                session
                    .respond(&Value::Integer(map.len() as i64), writer)
                    .await
            }
            Ok(_) => {
                session
                    .respond(&value_error!("Key is not a hashmap"), writer)
                    .await
            }
            Err(_) => session.respond(&Value::Integer(0), writer).await,
        }
    }
}
