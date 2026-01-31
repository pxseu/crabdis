use crate::prelude::*;

#[derive(Command)]
#[command(
	arity = -3,
    first_key = 1,
    last_key = 1,
    step = 1,
    complexity = "O(N) where N is the number of fields being requested",
	summary = "Get the values of all the given fields.",
	since = "0.1.38"
)]
pub struct HMGet;

#[async_trait]
impl Handler for HMGet {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() < 2 {
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

        let mut results = vec![Value::Nil; args.len()];

        let Some(map) = map else {
            return session.respond(&Value::Multi(results.into()), writer).await;
        };

        let mut index = 0usize;
        while let Some(field) = args.next_string_owned() {
            if let Some(value) = map.get(&Value::String(field)) {
                results[index] = value.clone();
            }
            index += 1;
        }

        session.respond(&Value::Multi(results.into()), writer).await
    }
}
