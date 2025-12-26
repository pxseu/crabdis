use crate::prelude::*;

pub struct Del;

#[async_trait]
impl CommandTrait for Del {
    fn name(&self) -> &str {
        "DEL"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.is_empty() {
            return session
                .versioned_response(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let mut store = session.state.store.write().await;
        let mut count = 0;
        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => {
                    if store.remove(&k).is_some() {
                        session.state.expire_keys.write().await.remove(&k);
                        count += 1;
                    }
                }

                _ => {
                    return session
                        .versioned_response(&value_error!("Invalid key"), writer)
                        .await;
                }
            }
        }

        Value::Integer(count).to_resp2(writer).await
    }
}
