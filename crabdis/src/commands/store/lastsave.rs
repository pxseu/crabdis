use crate::prelude::*;

pub struct LastSave;

#[async_trait]
impl CommandTrait for LastSave {
    fn name(&self) -> &'static str {
        "LASTSAVE"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
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
