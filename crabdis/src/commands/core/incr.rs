use crate::prelude::*;

pub struct Incr;

#[async_trait]
impl CommandTrait for Incr {
    fn name(&self) -> &'static str {
        "INCR"
    }

    fn info(&self) -> CommandInfo {
        CommandInfo {
            arity: 2,
            first_key: 1,
            last_key: 1,
            step: 1,
            summary: "Increments the integer value of a key by one",
            complexity: "O(1)",
            since: "0.1.34",
        }
    }

    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        if args.len() != 1 {
            return session
                .respond(&value_error!("Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let mut store = session.state.store.write().await;
        let mut value = match store.get_inner_unexpired(&key) {
            Ok(v) => match v {
                Value::String(s) => s.parse::<i64>().unwrap_or(0),
                Value::Integer(i) => *i,
                _ => {
                    return session
                        .respond(
                            &value_error!("ERR value is not an integer or out of range"),
                            writer,
                        )
                        .await;
                }
            },
            Err(_) => 0,
        };

        value += 1;

        // store it as int at some point
        store.insert(key, Value::String(value.to_string().into()));

        session.state.notify_change();

        session.respond(&Value::Integer(value), writer).await
    }
}
