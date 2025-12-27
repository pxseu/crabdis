use crate::prelude::*;

pub struct Exists;

#[async_trait]
impl CommandTrait for Exists {
    fn name(&self) -> &str {
        "EXISTS"
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
                    return session
                        .versioned_response(&value_error!("Invalid key"), writer)
                        .await;
                }
            }
        }

        session
            .versioned_response(&Value::Integer(count), writer)
            .await
    }
}
