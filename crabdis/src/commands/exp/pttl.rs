use crate::prelude::*;

pub struct PTtl;

#[async_trait]
impl CommandTrait for PTtl {
    fn name(&self) -> &str {
        "PTTL"
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

        let ttl = match store.get(key) {
            Some(Value::Expire((_, ttl))) => {
                let duration = ttl.duration_since(tokio::time::Instant::now()).as_millis() as i64;

                if duration != 0 { duration } else { -2 }
            }

            // non-expire keys should return -1
            Some(_) => -1,

            // not found keys should return -2
            None => -2,
        };

        session
            .versioned_response(&Value::Integer(ttl), writer)
            .await
    }
}
