use crate::prelude::*;

pub struct Info;

#[async_trait]
impl CommandTrait for Info {
    fn name(&self) -> &'static str {
        "INFO"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let length = args.len();

        if length > 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        if length == 0 {
            return session
                .respond(
                    &Value::String(
                        format!(
                            "loading:{}\r\n",
                            if session.state.loaded { "0" } else { "1" }
                        )
                        .into(),
                    ),
                    writer,
                )
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        log::debug!("INFO key: {key}");

        match key.to_lowercase().as_str() {
            "keyspace" => {
                let key_count = {
                    let store = session.state.store.read().await;
                    store.len()
                };

                if key_count == 0 {
                    return session
                        .respond(&Value::String("# Keyspace\r\n".into()), writer)
                        .await;
                }

                let expire_keys = {
                    let expire = session.state.expire_keys.read().await;
                    expire.len()
                };

                session
                    .respond(
                        &Value::String(
                            format!(
                                "# Keyspace\r\ndb0:keys={key_count},expires={expire_keys},avg_ttl=0\r\n"
                            )
                            .into(),
                        ),
                        writer,
                    )
                    .await
            }

            "server" => {
                session
                    .respond(&Value::String("redis_version:7.4.0\r\n".into()), writer)
                    .await
            }

            _ => session.respond(&value_error!("Invalid key"), writer).await,
        }
    }
}
