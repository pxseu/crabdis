use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Echo the given string",
    complexity = "O(1)",
    since = "0.1.38"
)]
pub struct Echo;

#[async_trait]
impl Handler for Echo {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("ERR wrong number of arguments"), writer)
                .await;
        }

        let message = args.next_owned().unwrap_or(Value::Nil);
        session.respond(&message, writer).await
    }
}
