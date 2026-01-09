use crate::prelude::*;

pub struct FlushDB;

#[async_trait]
impl CommandTrait for FlushDB {
    fn name(&self) -> &'static str {
        "FLUSHDB"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        session.state.store.write().await.clear();
        session.state.expire_keys.write().await.clear();

        session.state.notify_change();

        session.respond(&Value::Ok, writer).await
    }
}
