use crate::prelude::*;

pub struct Get;

#[async_trait]
impl CommandTrait for Get {
    fn name(&self) -> &str {
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
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let key = match args.next() {
            Some(Value::String(key)) => key,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid key"), writer)
                    .await;
            }
        };

        match session.state.store.read().await.get(key) {
            Some(value) => session.versioned_response(value, writer).await,
            None => session.versioned_response(&Value::Nil, writer).await,
        }
    }
}
