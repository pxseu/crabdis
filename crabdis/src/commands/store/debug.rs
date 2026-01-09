use crate::prelude::*;

pub struct Debug;

#[async_trait]
impl CommandTrait for Debug {
    fn name(&self) -> &'static str {
        "DEBUG"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let Some(subcommand) = args.next_string() else {
            return session
                .respond(
                    &value_error!("ERR wrong number of arguments for 'DEBUG' command"),
                    writer,
                )
                .await;
        };

        match subcommand.to_uppercase().as_str() {
            "RELOAD" => match session.state.load_rdb().await {
                Ok(count) => {
                    session
                        .respond(
                            &Value::Simple(format!("OK, loaded {count} keys").into()),
                            writer,
                        )
                        .await
                }
                Err(e) => session.respond(&value_error!("ERR {e}"), writer).await,
            },
            _ => {
                session
                    .respond(
                        &value_error!("ERR Unknown DEBUG subcommand: {subcommand}"),
                        writer,
                    )
                    .await
            }
        }
    }
}
