use crate::prelude::*;

pub struct PTtl;

#[async_trait]
impl CommandTrait for PTtl {
    fn name(&self) -> &'static str {
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
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
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

        session.respond(&Value::Integer(ttl), writer).await
    }
}
