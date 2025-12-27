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
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let key = match args.next() {
            Some(Value::String(key)) => key,
            Some(_) => {
                return session
                    .versioned_response(&value_error!("Invalid key"), writer)
                    .await;
            }
            None => {
                return session
                    .versioned_response(&value_error!("Missing key"), writer)
                    .await;
            }
        };

        let store = session.state.store.read().await;

        match store.get(key) {
            Some(value @ Value::Map(_)) => session.versioned_response(value, writer).await,

            Some(_) => {
                session
                    .versioned_response(&value_error!("Key is not a hashmap"), writer)
                    .await
            }

            None => session.versioned_response(&Value::Nil, writer).await,
        }
    }
}
