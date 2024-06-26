use crate::prelude::*;

pub struct Del;

#[async_trait]
impl CommandTrait for Del {
    fn name(&self) -> &str {
        "DEL"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() < 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let mut store = session.state.store.write().await;
        let mut count = 0;
        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => {
                    if store.remove(&k).is_some() {
                        count += 1;
                    }
                }

                _ => {
                    return value_error!("Invalid key").to_resp2(writer).await;
                }
            }
        }

        Value::Integer(count).to_resp2(writer).await
    }
}
