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
        session: SessionRef,
    ) -> Result<()> {
        if args.len() < 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let store = session.state.store.read().await;

        let mut count = 0;
        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => match store.get(&k) {
                    // Expired keys are not counted
                    Some(v) if v.expired() => {}
                    Some(_) => {
                        count += 1;
                    }
                    None => {}
                },

                _ => {
                    return value_error!("Invalid key").to_resp2(writer).await;
                }
            }
        }

        Value::Integer(count).to_resp2(writer).await
    }
}
