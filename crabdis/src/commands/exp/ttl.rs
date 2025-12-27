use crate::prelude::*;

pub struct Ttl;

#[async_trait]
impl CommandTrait for Ttl {
    fn name(&self) -> &str {
        "TTL"
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

        let Some(key) = args.next_string() else {
            return session
                .versioned_response(&value_error!("Invalid key"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        let duration = match store.get(key) {
            Some(Value::Expire((_, ttl))) => {
                let duration = ttl.duration_since(tokio::time::Instant::now()).as_secs() as i64;

                if duration != 0 { duration } else { -2 }
            }

            // non-expire keys should return -1
            Some(_) => -1,

            // not found keys should return -2
            None => -2,
        };

        session
            .versioned_response(&Value::Integer(duration), writer)
            .await
    }
}
