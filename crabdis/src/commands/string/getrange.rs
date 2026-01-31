use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 4,
    first_key = 1,
    last_key = 1,
    step = 1,
    since = "0.1.38",
    summary = "Returns the substring of the string value stored at key, determined by the offsets start and end (both are inclusive).",
    complexity = "O(N) where N is the length of the returned string."
)]
pub struct GetRange;

#[async_trait]
impl Handler for GetRange {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if args.len() != 3 {
            return session
                .respond(&value_error!("ERR Invalid number of arguments"), writer)
                .await;
        }

        let Some(key) = args.next_string() else {
            return session
                .respond(&value_error!("ERR Invalid key"), writer)
                .await;
        };

        let Some(start) = args.next_integer() else {
            return session
                .respond(
                    &value_error!("ERR start is not an integer or out of range"),
                    writer,
                )
                .await;
        };

        let Some(end) = args.next_integer() else {
            return session
                .respond(
                    &value_error!("ERR end is not an integer or out of range"),
                    writer,
                )
                .await;
        };

        let store = session.state.store.read().await;

        let reader = match store.get_unexpired(key) {
            Ok(Value::String(s)) => s.to_string(),
            _ => String::new(),
        };

        let len = reader.len() as i64;

        // Handle negative indices (Redis style: -1 is last character)
        let start_idx = if start < 0 { len + start } else { start };
        let end_idx = if end < 0 { len + end } else { end };

        // Clamp to valid range
        let start_idx = start_idx.max(0) as usize;
        let end_idx = end_idx.max(0) as usize;

        let result = reader
            .get(start_idx..=end_idx.min(reader.len().saturating_sub(1)))
            .unwrap_or_default();

        session.respond(&Value::String(result.into()), writer).await
    }
}
