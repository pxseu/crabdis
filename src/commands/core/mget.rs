use crate::prelude::*;

pub struct MGet;

#[async_trait]
impl CommandTrait for MGet {
    fn name(&self) -> &str {
        "MGET"
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

        let mut values = VecDeque::with_capacity(args.len());

        let store = session.state.store.write().await;

        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => match store.get(&k) {
                    Some(value) => values.push_back(value.clone()),
                    None => values.push_back(Value::Nil),
                },

                _ => {
                    return value_error!("Invalid key").to_resp2(writer).await;
                }
            }
        }

        Value::Multi(values).to_resp2(writer).await
    }
}
