use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Ping the server",
    complexity = "O(1)",
    since = "0.1.34",
)]
pub struct Ping;

#[async_trait]
impl Handler for Ping {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
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
