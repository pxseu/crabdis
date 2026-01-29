use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 3,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Determines whether a field exists in a hash",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct HExists;

#[async_trait]
impl Handler for HExists {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let Some(field) = args.next_owned() else {
            return session
                .respond(&value_error!("Invalid field"), writer)
                .await;
        };

        let store = session.state.store.read().await;

        let count = match store.get_inner_unexpired(key) {
            Ok(Value::Map(map)) => i64::from(map.contains_key(&field)),
            Ok(_) => {
                return session
                    .respond(&value_error!("Key is not a hashmap"), writer)
                    .await;
            }
            Err(_) => 0,
        };

        session.respond(&Value::Integer(count), writer).await
    }
}
