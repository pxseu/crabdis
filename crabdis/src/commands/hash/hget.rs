use crate::prelude::*;

pub struct HGet;

#[async_trait]
impl CommandTrait for HGet {
    fn name(&self) -> &str {
        "HGET"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session
                .versioned_response(&value_error!("Invalid key"), writer)
                .await;
        };

        let Some(field) = args.next_string_owned() else {
            return session
                .versioned_response(&value_error!("Invalid field"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        match store.get(key) {
            Some(Value::Map(map)) => match map.get(&Value::String(field)) {
                Some(value) => session.versioned_response(value, writer).await,
                None => session.versioned_response(&Value::Nil, writer).await,
            },
            Some(_) => {
                session
                    .versioned_response(&value_error!("Key is not a hashmap"), writer)
                    .await
            }
            None => session.versioned_response(&Value::Nil, writer).await,
        }
    }
}
