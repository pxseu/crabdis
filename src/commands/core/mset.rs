use crate::prelude::*;

pub struct MSet;

#[async_trait]
impl CommandTrait for MSet {
    fn name(&self) -> &str {
        "MSET"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        context: ContextRef,
    ) -> Result<()> {
        if args.len() < 2 || args.len() % 2 != 0 {
            return value_error!("Invalid number of arguments")
                .to_resp(writer)
                .await;
        }

        let mut store = context.store.write().await;

        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => {
                    // safe to unwrap because we checked the length of the args
                    store.insert(k, args.pop_front().unwrap());
                }

                _ => {
                    return value_error!("Invalid key").to_resp(writer).await;
                }
            }
        }

        Value::Ok.to_resp(writer).await
    }
}
