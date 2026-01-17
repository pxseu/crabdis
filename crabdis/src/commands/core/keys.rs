use glob::Pattern;

use crate::prelude::*;

pub struct Keys;

#[async_trait]
impl CommandTrait for Keys {
    fn name(&self) -> &'static str {
        "KEYS"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 0,
            last_key: 0,
            step: 0,
            summary: "Returns all keys matching pattern",
            complexity: "O(N) where N is the number of keys in the database",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(pattern_str) = args.next_string() else {
            return session
                .respond(&value_error!("Invalid pattern"), writer)
                .await;
        };
        let pattern = Pattern::new(pattern_str)?;

        let mut keys = Vec::new();

        let store = session.state.store.read().await;
        for (key, value) in store.iter() {
            if !value.expired() && pattern.matches(key) {
                keys.push(Value::String(key.clone()));
            }
        }

        session.respond(&Value::Multi(keys.into()), writer).await
    }
}
