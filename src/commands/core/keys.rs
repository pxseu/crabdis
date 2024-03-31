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
        context: ContextRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return value_error!("Invalid number of arguments")
                .to_resp(writer)
                .await;
        }

        let pattern = match args.pop_front() {
            Some(Value::String(s)) => Pattern::new(&s)?,
            _ => {
                return value_error!("Invalid pattern").to_resp(writer).await;
            }
        };

        let mut keys = VecDeque::new();

        for key in context.store.read().await.keys() {
            if pattern.matches(key) {
                keys.push_back(Value::String(key.clone()));
            }
        }

        Value::Multi(keys).to_resp(writer).await
    }
}
