use crate::prelude::*;

pub struct Exists;

#[async_trait]
impl CommandTrait for Exists {
    fn name(&self) -> &str {
        "EXISTS"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        context: ContextRef,
    ) -> Result<()> {
        if args.len() < 1 {
            return value_error!("Invalid number of arguments")
                .to_resp(writer)
                .await;
        }

        let store = context.store.read().await;

        let mut count = 0;
        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => {
                    if store.contains_key(&k) {
                        count += 1;
                    }
                }

                _ => {
                    return value_error!("Invalid key").to_resp(writer).await;
                }
            }
        }

        Value::Integer(count).to_resp(writer).await
    }
}
