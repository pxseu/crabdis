use crate::prelude::*;
use crate::storage::rdb;

#[derive(Command)]
#[command(
    arity = 1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Saves the dataset to disk",
    complexity = "O(N) where N is the number of keys in the database",
    since = "0.1.34"
)]
pub struct Save;

#[async_trait]
impl Handler for Save {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        match rdb::save_rdb(&session.state).await {
            Ok(()) => session.respond(&Value::Ok, writer).await,
            Err(e) => session.respond(&value_error!("ERR {e}"), writer).await,
        }
    }
}
