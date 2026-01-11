use crate::prelude::*;

pub struct Get;

#[async_trait]
impl CommandTrait for Get {
    fn name(&self) -> &'static str {
        "GET"
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

        match session.state.store.read().await.get(key) {
            Some(value) if value.primitive() => session.respond(value, writer).await,
            Some(_) => {
                session
                    .respond(&value_error!("Value is not a simple string"), writer)
                    .await
            }
            None => session.respond(&Value::Nil, writer).await,
        }
    }
}
