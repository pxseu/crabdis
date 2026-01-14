use crate::prelude::*;

pub struct HGet;

#[async_trait]
impl CommandTrait for HGet {
    fn name(&self) -> &'static str {
        "HGET"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 3,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Get the value of a hash field",
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
        if args.len() != 2 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let Some(field) = args.next_owned() else {
            return session
                .respond(&value_error!("Invalid field"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        let value = match store.get_inner_unexpired(key)? {
            Value::Map(map) => map.get(&field).cloned().unwrap_or(Value::Nil),
            _ => {
                return session
                    .respond(&value_error!("Key is not a hashmap"), writer)
                    .await;
            }
        };

        session.respond(&value, writer).await
    }
}
