use crate::prelude::*;

pub struct MGet;

#[async_trait]
impl CommandTrait for MGet {
    fn name(&self) -> &str {
        "MGET"
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

        let store = session.state.store.read().await;

        let mut values = Vec::with_capacity(args.len());

        for key in args {
            match key {
                Value::String(k) => match store.get(k) {
                    Some(value) => values.push(value.clone()),
                    None => values.push(Value::Nil),
                },

                _ => {
                    return session
                        .versioned_response(&value_error!("Invalid key"), writer)
                        .await;
                }
            }
        }

        session
            .versioned_response(&Value::Multi(values.into()), writer)
            .await
    }
}
