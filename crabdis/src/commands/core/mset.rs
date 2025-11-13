use crate::prelude::*;

pub struct MSet;

#[async_trait]
impl CommandTrait for MSet {
    fn name(&self) -> &str {
        "MSET"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() < 2 || !args.len().is_multiple_of(2) {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let mut store = session.state.store.write().await;

        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => {
                    // safe to unwrap because we checked the length of the args
                    store.insert(k, args.pop_front().unwrap());
                }

                _ => {
                    return session
                        .versioned_response(&value_error!("Invalid key"), writer)
                        .await;
                }
            }
        }

        Value::Ok.to_resp2(writer).await
    }
}
