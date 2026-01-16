use crate::prelude::*;

pub struct Exists;

#[async_trait]
impl CommandTrait for Exists {
    fn name(&self) -> &'static str {
        "EXISTS"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -2,
            first_key: 1,
            last_key: -1,
            step: 1,
            summary: "Returns the number of keys existing among the given keys",
            complexity: "O(N) where N is the number of keys to check",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let store = session.state.store.read().await;

        let mut count = 0;
        for key in args {
            match key {
                Value::String(k) => {
                    if store.get_unexpired(k).is_ok() {
                        count += 1;
                    }
                }

                _ => {
                    return session.respond(&value_error!("Invalid key"), writer).await;
                }
            }
        }

        session.respond(&Value::Integer(count), writer).await
    }
}
