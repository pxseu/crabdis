use crate::prelude::*;

#[derive(Subcommand)]
#[command(
    arity = 3,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Sets the connection name.",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct SetName;

#[async_trait]
impl Handler for SetName {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let Some(name) = args.next_string_owned() else {
            return session
                .respond(
                    &value_error!("ERR wrong number of arguments for 'CLIENT SETNAME' command"),
                    writer,
                )
                .await;
        };

        session.set_name(name).await;
        session.respond(&Value::Ok, writer).await
    }
}
