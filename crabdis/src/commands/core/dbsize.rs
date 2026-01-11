use crate::prelude::*;

pub struct DBSize;

#[async_trait]
impl CommandTrait for DBSize {
    fn name(&self) -> &'static str {
        "DBSIZE"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let key_count = {
            let store = session.state.store.read().await;
            store.values().filter(|v| !v.expired()).count()
        };

        session
            .respond(
                &Value::Integer(i64::try_from(key_count).unwrap_or(i64::MAX)),
                writer,
            )
            .await
    }
}
