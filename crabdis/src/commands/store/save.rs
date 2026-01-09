use crate::prelude::*;

pub struct Save;

#[async_trait]
impl CommandTrait for Save {
    fn name(&self) -> &'static str {
        "SAVE"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        match session.state.save_rdb().await {
            Ok(()) => session.respond(&Value::Ok, writer).await,
            Err(e) => session.respond(&value_error!("ERR {e}"), writer).await,
        }
    }
}
