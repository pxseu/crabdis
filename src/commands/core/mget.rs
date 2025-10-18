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

        let mut values = Vec::with_capacity(args.len());

        let store = session.state.store.write().await;

        while let Some(key) = args.pop_front() {
            match key {
                Value::String(k) => match store.get(&k) {
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

        Value::Multi(Arc::new(values.into_boxed_slice()))
            .to_resp2(writer)
            .await
    }
}
