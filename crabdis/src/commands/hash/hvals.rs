use crate::prelude::*;

#[derive(Command)]
#[command(
	arity = -2,
	first_key = 1,
	last_key = 1,
	step = 1,
	summary = "Get all the values in a hash.",
	complexity = "O(N) where N is the number of fields in the hash",
	since = "0.1.38"
)]
pub struct HVals;

#[async_trait]
impl Handler for HVals {
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

        let map = match store.get_inner_unexpired(key) {
            Ok(Value::Map(map)) => Some(map),
            Ok(_) => {
                return session
                    .respond(
                        &value_error!(
                            "WRONGTYPE Operation against a key holding the wrong kind of value"
                        ),
                        writer,
                    )
                    .await;
            }
            Err(_) => None,
        };

        let Some(map) = map else {
            return session.respond(&value_multi!(), writer).await;
        };

        let keys = map.values().cloned().collect::<Arc<_>>();

        session.respond(&Value::Multi(keys), writer).await
    }
}
