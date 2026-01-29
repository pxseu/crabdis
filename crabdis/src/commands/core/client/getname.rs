use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = 2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns the name of the current connection.",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct GetName;

#[async_trait]
impl Handler for GetName {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        session.respond(&session.name().await.into(), writer).await
    }
}
