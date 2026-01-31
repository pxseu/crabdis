use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 2,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Returns all fields and values of the hash stored at key",
    complexity = "O(N) where N is the number of fields in the hash",
    since = "0.1.34"
)]
pub struct HGetAll;

#[async_trait]
impl Handler for HGetAll {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("ERR Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session
                .respond(&value_error!("ERR Invalid key"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        let value = store.get_inner_unexpired(key)?;

        if !matches!(value, Value::Map(_)) {
            return session
                .respond(
                    &value_error!(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                    ),
                    writer,
                )
                .await;
        }

        session.respond(value, writer).await
    }
}
