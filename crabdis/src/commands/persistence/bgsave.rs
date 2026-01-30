use crate::prelude::*;
use crate::storage::rdb;

#[derive(Command)]
#[command(
    arity = -1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Asynchronously save the dataset to disk",
    complexity = "O(1)",
    since = "0.1.34",
)]
pub struct BgSave;

#[async_trait]
impl Handler for BgSave {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
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
