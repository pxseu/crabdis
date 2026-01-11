use crate::prelude::*;

pub struct Decr;

#[async_trait]
impl CommandTrait for Decr {
    fn name(&self) -> &'static str {
        "DECR"
    }

    async fn handle_command(
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
                        .respond(&value_error!("Invalid value"), writer)
                        .await;
                }
            },
            Err(_) => 0,
        };

        value -= 1;

        // store it as int at some point
        store.insert(key, Value::String(value.to_string().into()));

        session.state.notify_change();

        session.respond(&Value::Integer(value), writer).await
    }
}
