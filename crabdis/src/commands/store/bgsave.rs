use crate::prelude::*;
use crate::storage::rdb;

pub struct BgSave;

#[async_trait]
impl CommandTrait for BgSave {
    fn name(&self) -> &'static str {
        "BGSAVE"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: -1,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Asynchronously save the dataset to disk",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        match rdb::bgsave_rdb(session.state.clone()) {
            Ok(()) => {
                session
                    .respond(&Value::Simple("Background saving started".into()), writer)
                    .await
            }
            Err(e) => session.respond(&value_error!("ERR {e}"), writer).await,
        }
    }
}
