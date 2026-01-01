use crate::prelude::*;

pub struct Ping;

#[async_trait]
impl CommandTrait for Ping {
    fn name(&self) -> &'static str {
        "PING"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() > 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let response = args.next().unwrap_or_else(|| &Value::Pong);
        session.respond(response, writer).await
    }
}
