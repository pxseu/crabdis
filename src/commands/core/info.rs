use crate::prelude::*;

pub struct Info;

#[async_trait]
impl CommandTrait for Info {
    fn name(&self) -> &str {
        "INFO"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        let length = args.len();

        if length > 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        if length == 0 {
            return session
                .versioned_response(
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

        let key = match args.pop_front() {
            Some(Value::String(key)) => key,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid key"), writer)
                    .await
            }
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
                        .versioned_response(&Value::String("# Keyspace\r\n".into()), writer)
                        .await;
                }

                let expire_keys = {
                    let expire = session.state.expire_keys.read().await;
                    expire.len()
                };

                session
                    .versioned_response(
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
                    .versioned_response(&Value::String("redis_version:7.4.0\r\n".into()), writer)
                    .await
            }

            _ => {
                session
                    .versioned_response(&value_error!("Invalid key"), writer)
                    .await
            }
        }
    }
}
