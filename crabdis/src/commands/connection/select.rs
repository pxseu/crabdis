use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Select the Redis logical database having the specified zero-based numeric index",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct Select;

#[async_trait]
impl Handler for Select {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        session.respond(&Value::Ok, writer).await
    }
}
