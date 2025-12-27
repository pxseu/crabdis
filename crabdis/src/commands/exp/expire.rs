use crate::prelude::*;

pub struct Expire;

#[async_trait]
impl CommandTrait for Expire {
    fn name(&self) -> &str {
        "EXPIRE"
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

        let key = match args.next() {
            Some(Value::String(key)) => key.clone(),
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

        let seconds = match args.next() {
            Some(Value::Integer(seconds)) => *seconds,
            Some(Value::String(seconds)) => seconds.parse::<i64>().unwrap_or(-1),
            Some(_) => {
                return session
                    .versioned_response(&value_error!("Invalid seconds"), writer)
                    .await;
            }
            None => {
                return session
                    .versioned_response(&value_error!("Missing seconds"), writer)
                    .await;
            }
        };

        if seconds < 0 {
            return session
                .versioned_response(&value_error!("Invalid seconds"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;

        let value = match store.get_mut(&key) {
            Some(value) if value.expired() => {
                return session
                    .versioned_response(&value_error!("Key is expired"), writer)
                    .await;
            }
            Some(value) => value,
            None => {
                return session
                    .versioned_response(&value_error!("Key not found"), writer)
                    .await;
            }
        };

        *value = Value::Expire((
            value.inner().clone().into(),
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(seconds as u64),
        ));
        session.state.expire_keys.write().await.insert(key);

        session.versioned_response(&Value::Ok, writer).await
    }
}
