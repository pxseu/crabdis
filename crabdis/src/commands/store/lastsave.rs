use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns the UNIX time stamp of the last successful save to disk",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct LastSave;

#[async_trait]
impl Handler for LastSave {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let last_save = session
            .state
            .rdb_config
            .last_save_time
            .load(Ordering::Relaxed);

        session
            .respond(&Value::Integer(last_save as i64), writer)
            .await
    }
}
