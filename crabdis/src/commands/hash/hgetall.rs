use crate::prelude::*;

pub struct HGetAll;

#[async_trait]
impl CommandTrait for HGetAll {
    fn name(&self) -> &str {
        "HGETALL"
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

        match store.get(key) {
            Some(value @ Value::Map(_)) => session.respond(value, writer).await,

            Some(_) => {
                session
                    .respond(&value_error!("Key is not a hashmap"), writer)
                    .await
            }

            None => session.respond(&Value::Nil, writer).await,
        }
    }
}
