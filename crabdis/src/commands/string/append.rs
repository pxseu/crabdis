use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 3,
    first_key = 1,
    last_key = 1,
    step = 1,
    since = "0.1.38",
    complexity = "O(1)",
    summary = "Append a value to a key"
)]
pub struct Append;

#[async_trait]
impl Handler for Append {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 2 {
            return session
                .respond(&value_error!("ERR Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string_owned() else {
            return session
                .respond(&value_error!("ERR Invalid key"), writer)
                .await;
        };

        let Some(value) = args.next_string() else {
            return session
                .respond(&value_error!("ERR Invalid value"), writer)
                .await;
        };

        let mut store = session.state.store.write().await;

        let string = match store.get_inner_unexpired(&key) {
            Ok(Value::String(string)) => Some(string),
            Ok(_) => {
                return session
                    .respond(
                        &value_error!(
                            "WRONGTYPE Operation against a key holding the wrong kind of value"
                        ),
                        writer,
                    )
                    .await;
            }
            Err(_) => None,
        };

        let new_value = string.map_or_else(
            || value.clone(),
            |string| {
                let mut new_string = String::with_capacity(string.len() + value.len());
                new_string.push_str(string);
                new_string.push_str(value);
                new_string.into()
            },
        );

        let length = new_value.len() as i64;

        store.insert(key, Value::String(new_value));

        session.respond(&Value::Integer(length), writer).await
    }
}
