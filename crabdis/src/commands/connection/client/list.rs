use tokio::time::Instant;

use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = 2,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Lists all client connections.",
    complexity = "O(N) where N is the number of clients",
    since = "0.1.34"
)]
pub struct List;

#[async_trait]
impl Handler for List {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        _args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let mut list = String::new();

        for (id, session) in session.state.sessions.read().await.iter() {
            use std::fmt::Write;
            writeln!(
                list,
                "id={id} addr={} name={} age={} user=default",
                session.socket_addr,
                session.name().await.unwrap_or_else(|| "".into()),
                Instant::now().duration_since(session.age).as_secs()
            )
            .expect("Writing to String should not fail");
        }

        session.respond(&Value::String(list.into()), writer).await
    }
}
