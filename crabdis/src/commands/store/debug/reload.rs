use crate::prelude::*;
use crate::storage::rdb;

pub struct Reload;

#[async_trait]
impl SubcommandTrait for Reload {
    fn name(&self) -> &'static str {
        "RELOAD"
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: 2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Reloads the RDB file.",
            complexity: "O(N) where N is the number of keys in the database",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        match rdb::load_rdb(&session.state).await {
            Ok(count) => {
                session
                    .respond(
                        &Value::Simple(format!("OK, loaded {count} keys").into()),
                        writer,
                    )
                    .await
            }
            Err(e) => session.respond(&value_error!("ERR {e}"), writer).await,
        }
    }
}
