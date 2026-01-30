use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -3,
    first_key = 1,
    last_key = -1,
    step = 2,
    summary = "Sets multiple keys to multiple values",
    complexity = "O(N) where N is the number of keys being set",
    since = "0.1.34",
)]
pub struct MSet;

#[async_trait]
impl Handler for MSet {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() < 2 || !args.len().is_multiple_of(2) {
            return session
                .respond(&value_error!("ERR Invalid number of arguments"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;

        while let Some(key) = args.next() {
            match key {
                Value::String(k) => {
                    // safe to unwrap because we checked the length of the args
                    store.insert(k.clone(), args.next_owned().unwrap());
                }

                _ => {
                    return session
                        .respond(&value_error!("ERR Invalid key"), writer)
                        .await;
                }
            }
        }

        session.state.notify_change();

        session.respond(&Value::Ok, writer).await
    }
}
