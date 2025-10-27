use glob::Pattern;

use crate::prelude::*;

pub struct Keys;

#[async_trait]
impl CommandTrait for Keys {
    fn name(&self) -> &str {
        "KEYS"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return value_error!("Invalid number of arguments")
                .to_resp2(writer)
                .await;
        }

        let pattern = match args.pop_front() {
            Some(Value::String(s)) => Pattern::new(&s)?,
            _ => {
                return session
                    .versioned_response(&value_error!("Invalid pattern"), writer)
                    .await;
            }
        };

        let mut keys = Vec::new();

        for key in session.state.store.read().await.keys() {
            if pattern.matches(key) {
                keys.push(Value::String(key.clone()));
            }
        }

        session
            .versioned_response(&Value::Multi(keys.into()), writer)
            .await
    }
}
