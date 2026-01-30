use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 2,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Get the length of the value stored at key",
    complexity = "O(1)",
    since = "0.1.38"
)]
pub struct StrLen;

#[async_trait]
impl Handler for StrLen {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("ERR wrong number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session
                .respond(&value_error!("ERR Invalid key"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        let Ok(value) = store.get_inner_unexpired(key) else {
            return session.respond(&Value::Integer(0), writer).await;
        };

        let Some(s) = value.as_string() else {
            return session
                .respond(
                    &value_error!(
                        "WRONGTYPE Operation against a key holding the wrong kind of value"
                    ),
                    writer,
                )
                .await;
        };

        return session
            .respond(&Value::Integer(s.len() as i64), writer)
            .await;
    }
}
