use crate::prelude::*;

pub struct List;

#[async_trait]
impl SubcommandTrait for List {
    fn name(&self) -> &'static str {
        "LIST"
    }

    fn info(&self) -> SubcommandInfo {
        SubcommandInfo {
            arity: 2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Lists all client connections.",
            complexity: "O(N) where N is the number of clients",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let mut list = String::new();

        for (id, session) in session.state.sessions.read().await.iter() {
            use std::fmt::Write;
            writeln!(
                list,
                "id={id} name={}",
                session.name().await.unwrap_or_else(|| "(nil)".into())
            )
            .expect("Writing to String should not fail");
        }

        session.respond(&Value::String(list.into()), writer).await
    }
}
