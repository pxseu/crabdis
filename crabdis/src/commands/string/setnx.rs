use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 3,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Set the value of a key, only if the key does not exist",
    complexity = "O(1)",
    since = "0.1.38"
)]
pub struct SetNx;

#[async_trait]
impl Handler for SetNx {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .respond(&value_error!("ERR wrong number of arguments for 'setnx' command"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session
                .respond(&value_error!("ERR Invalid key"), writer)
                .await;
        };

        let Some(value) = args.next_owned() else {
            return session
                .respond(&value_error!("ERR Invalid value"), writer)
                .await;
        };

        let mut store = session.state.store.write().await;

        // Check if key exists and is not expired
        if store.get_unexpired(&key).is_ok() {
            return session.respond(&Value::Integer(0), writer).await;
        }

        // Key doesn't exist or is expired, set it
        store.insert(key, value);
        session.state.notify_change();

        session.respond(&Value::Integer(1), writer).await
    }
}
