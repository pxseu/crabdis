use crate::prelude::*;

pub struct Exists;

#[async_trait]
impl CommandTrait for Exists {
    fn name(&self) -> &'static str {
        "EXISTS"
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
