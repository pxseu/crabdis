use crate::prelude::*;

pub struct HExists;

#[async_trait]
impl CommandTrait for HExists {
    fn name(&self) -> &str {
        "HEXISTS"
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

        let Some(field) = args.next_string_owned() else {
            return session
                .respond(&value_error!("Invalid field"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        match store.get(key) {
            Some(Value::Map(map)) => {
                session
                    .respond(
                        &Value::Integer(map.contains_key(&Value::String(field)) as i64),
                        writer,
                    )
                    .await
            }
            Some(_) => {
                session
                    .respond(&value_error!("Key is not a hashmap"), writer)
                    .await
            }
            None => session.respond(&Value::Nil, writer).await,
        }
    }
}
