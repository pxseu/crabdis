use std::time::{SystemTime, UNIX_EPOCH};

use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = 1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Returns the current server time",
    complexity = "O(1)",
    since = "0.1.38"
)]
pub struct Time;

#[async_trait]
impl Handler for Time {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        if !args.is_empty() {
            return session
                .respond(&value_error!("ERR wrong number of arguments"), writer)
                .await;
        }

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let seconds = duration.as_secs() as i64;
        let microseconds = duration.subsec_micros().into();

        session
            .respond(
                &value_multi!(Value::Integer(seconds), Value::Integer(microseconds)),
                writer,
            )
            .await
    }
}
