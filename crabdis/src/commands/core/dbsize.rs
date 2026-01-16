use crate::prelude::*;

pub struct DBSize;

#[async_trait]
impl CommandTrait for DBSize {
    fn name(&self) -> &'static str {
        "DBSIZE"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns the number of keys in the selected database",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
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
