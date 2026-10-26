use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 2,
    first_key = 1,
    last_key = 1,
    step = 1,
    summary = "Decrements the number stored at key by one",
    complexity = "O(1)",
    since = "0.1.34"
)]
pub struct Decr;

#[async_trait]
impl Handler for Decr {
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

        let Some(key) = args.next_string_owned() else {
            return session.respond(&value_error!("Invalid key"), writer).await;
        };

        let mut store = session.state.store.write().await;
        let value = match store.get_inner_unexpired(&key) {
            Ok(v) => match v {
                Value::String(s) => match s.parse::<i64>() {
                    Ok(i) => i,
                    Err(_) => {
                        return session
                            .respond(
                                &value_error!("ERR value is not an integer or out of range"),
                                writer,
                            )
                            .await;
                    }
                },
                Value::Integer(i) => *i,
                _ => {
                    return session
                        .respond(&value_error!("Invalid value"), writer)
                        .await;
                }
            },
            Err(_) => 0,
        };

        let Some(value) = value.checked_sub(1) else {
            return session
                .respond(
                    &value_error!("ERR increment or decrement would overflow"),
                    writer,
                )
                .await;
        };

        // store it as int at some point
        store.insert(key, Value::String(value.to_string().into()));

        session.state.notify_change();

        session.respond(&Value::Integer(value), writer).await
    }
}
