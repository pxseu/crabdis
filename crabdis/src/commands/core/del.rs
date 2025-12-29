use crate::prelude::*;

pub struct Del;

#[async_trait]
impl CommandTrait for Del {
    fn name(&self) -> &str {
        "DEL"
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

        session.respond(&Value::Integer(count), writer).await
    }
}
