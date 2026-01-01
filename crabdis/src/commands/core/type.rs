use crate::prelude::*;

pub struct Type;

#[async_trait]
impl CommandTrait for Type {
    fn name(&self) -> &'static str {
        "TYPE"
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
        let value = store.get(key);

        let value_type = match value {
            Some(Value::String(_) | Value::Integer(_)) => "string",
            Some(Value::Multi(_)) => "list",
            Some(Value::Set(_)) => "set",
            Some(Value::Map(_)) => "hash",
            None => "none",
            _ => {
                return session
                    .respond(&value_error!("Invalid value type"), writer)
                    .await;
            }
        };

        session
            .respond(&Value::String(value_type.into()), writer)
            .await
    }
}
