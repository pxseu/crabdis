use crate::prelude::*;

pub struct Ttl;

#[async_trait]
impl CommandTrait for Ttl {
    fn name(&self) -> &'static str {
        "TTL"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Get the time to live for a key in seconds",
            complexity: "O(1)",
            since: "1.0.0",
        }
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

        let duration = match store.get_unexpired(key) {
            Ok(Value::Expire((_, ttl))) => {
                ttl.duration_since(tokio::time::Instant::now()).as_secs() as i64
            }
            Ok(_) => -1,  // non-expire keys should return -1
            Err(_) => -2, // not found or expired keys should return -2
        };

        session.respond(&Value::Integer(duration), writer).await
    }
}
