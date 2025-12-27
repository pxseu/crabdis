use crate::prelude::*;

pub struct Client;

#[async_trait]
impl CommandTrait for Client {
    fn name(&self) -> &str {
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
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(command) = args.next_string() else {
            return session
                .versioned_response(&value_error!("Invalid command"), writer)
                .await;
        };
        let command = command.to_uppercase();

        match command.as_ref() {
            "GETNAME" => {
                session
                    .versioned_response(&session.get_name().await.into(), writer)
                    .await
            }

            "SETNAME" => {
                let Some(name) = args.next_string_owned() else {
                    return session
                        .versioned_response(&value_error!("Invalid name"), writer)
                        .await;
                };

                session.set_name(name).await;
                session.versioned_response(&Value::Ok, writer).await
            }

            "LIST" => {
                let mut list = String::new();

                for (id, session) in session.state.sessions.read().await.iter() {
                    list.push_str(&format!(
                        "id={id} name={}\n",
                        session.get_name().await.unwrap_or("(nil)".into())
                    ));
                }

                session
                    .versioned_response(&Value::String(list.into()), writer)
                    .await
            }

            _ => {
                return session
                    .versioned_response(&value_error!("Invalid command"), writer)
                    .await;
            }
        }
    }
}
