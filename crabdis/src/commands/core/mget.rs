use crate::prelude::*;

pub struct MGet;

#[async_trait]
impl CommandTrait for MGet {
    fn name(&self) -> &'static str {
        "MGET"
    }

    async fn handle_command(
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
