use crate::prelude::*;

pub struct Type;

#[async_trait]
impl CommandTrait for Type {
    fn name(&self) -> &'static str {
        "TYPE"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns the data type of the value stored at key",
            complexity: "O(1) for every call",
            since: "1.0.0",
        }
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let length = args.len();

        if length != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        log::debug!("TYPE key: {key}");

        let store = session.state.store.read().await;

        let value_type = match store.get_inner_unexpired(key) {
            Ok(value) => match value {
                Value::String(_) | Value::Integer(_) => "string",
                Value::Multi(_) => "list",
                Value::Set(_) => "set",
                Value::Map(_) => "hash",
                _ => {
                    return session
                        .respond(&value_error!("Invalid value type"), writer)
                        .await;
                }
            },
            Err(_) => "none",
        };

        session
            .respond(&Value::String(value_type.into()), writer)
            .await
    }
}
