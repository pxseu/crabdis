use crate::prelude::*;

pub struct Client;

#[async_trait]
impl CommandTrait for Client {
    fn name(&self) -> &'static str {
        "CLIENT"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(command) = args.next_string() else {
            return session
                .respond(&value_error!("Invalid command"), writer)
                .await;
        };
        let command = command.to_uppercase();

        match command.as_ref() {
            "GETNAME" => session.respond(&session.name().await.into(), writer).await,

            "SETNAME" => {
                let Some(name) = args.next_string_owned() else {
                    return session.respond(&value_error!("Invalid name"), writer).await;
                };

                session.set_name(name).await;
                session.respond(&Value::Ok, writer).await
            }

            "LIST" => {
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

            _ => {
                return session
                    .respond(&value_error!("Invalid command"), writer)
                    .await;
            }
        }
    }
}
