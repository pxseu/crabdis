use crate::prelude::*;

pub struct DBSize;

#[async_trait]
impl CommandTrait for DBSize {
    fn name(&self) -> &str {
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
            store.len()
        };

        session
            .versioned_response(&Value::Integer(key_count as i64), writer)
            .await
    }
}
