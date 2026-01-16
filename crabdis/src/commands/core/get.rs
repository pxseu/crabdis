use crate::prelude::*;

pub struct Get;

#[async_trait]
impl CommandTrait for Get {
    fn name(&self) -> &'static str {
        "GET"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Gets the value of a key",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
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

        if value.is_primitive() {
            return session.respond(value, writer).await;
        }

        session
            .respond(
                &value_error!("WRONGTYPE Operation against a key holding the wrong kind of value"),
                writer,
            )
            .await
    }
}
