use crate::prelude::*;

pub struct GetName;

#[async_trait]
impl SubcommandTrait for GetName {
    fn name(&self) -> &'static str {
        "GETNAME"
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: 2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns the name of the current connection.",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        session.respond(&session.name().await.into(), writer).await
    }
}
