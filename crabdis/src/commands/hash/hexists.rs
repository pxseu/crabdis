use crate::prelude::*;

pub struct HExists;

#[async_trait]
impl CommandTrait for HExists {
    fn name(&self) -> &'static str {
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

        let Some(field) = args.next_owned() else {
            return session
                .respond(&value_error!("Invalid field"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        let count = match store.get_inner_unexpired(key) {
            Ok(Value::Map(map)) => i64::from(map.contains_key(&field)),
            Ok(_) => {
                return session
                    .respond(&value_error!("Key is not a hashmap"), writer)
                    .await;
            }
            Err(_) => 0,
        };

        session.respond(&Value::Integer(count), writer).await
    }
}
