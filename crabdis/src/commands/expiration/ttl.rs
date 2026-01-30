use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 2,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Get the time to live for a key in seconds",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct Ttl;

#[async_trait]
impl Handler for Ttl {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
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
