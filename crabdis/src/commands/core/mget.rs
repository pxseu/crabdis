use crate::prelude::*;

pub struct MGet;

#[async_trait]
impl CommandTrait for MGet {
    fn name(&self) -> &'static str {
        "MGET"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -2,
            first_key: 1,
            last_key: -1,
            step: 1,
            summary: "Gets the values of all the given keys",
            complexity: "O(N) where N is the number of keys to retrieve",
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

        let mut values = Vec::with_capacity(args.len());

        for key in args {
            match key {
                Value::String(k) => {
                    values.push(store.get_inner_unexpired(k).cloned().unwrap_or(Value::Nil));
                }

                _ => {
                    return session.respond(&value_error!("Invalid key"), writer).await;
                }
            }
        }

        session.respond(&Value::Multi(values.into()), writer).await
    }
}
