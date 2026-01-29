use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -2,
    first_key = 1,
    last_key = -1,
    step = 1,
    summary = "Removes the specified keys",
    complexity = "O(N) where N is the number of keys to be removed",
    since = "0.1.34",
)]
pub struct Del;

#[async_trait]
impl Handler for Del {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.is_empty() {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;
        let mut expire_keys = session.state.expire_keys.write().await;
        let mut count = 0;
        for key in args {
            match key {
                Value::String(k) => {
                    if store.remove(k).is_some() {
                        expire_keys.remove(k);
                        count += 1;
                    }
                }

                _ => {
                    return session.respond(&value_error!("Invalid key"), writer).await;
                }
            }
        }

        if count > 0 {
            session.state.notify_change();
        }

        session.respond(&Value::Integer(count), writer).await
    }
}
