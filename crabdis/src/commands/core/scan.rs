use glob::Pattern;

use crate::prelude::*;

#[derive(Command)]
#[command(
    arity = -1,
    first_key = 0,
    last_key = 0,
    step = 0,
    summary = "Incrementally iterate the keys space",
    complexity = "O(N) where N is the number of elements returned",
    since = "0.1.34",
)]
pub struct Scan;

#[async_trait]
impl Handler for Scan {
    async fn handle(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: &Session,
    ) -> Result<()> {
        let mut cursor = Option::<usize>::None;
        let mut pattern = Option::<Pattern>::None;
        let mut count = 10;

        while let Some(arg) = args.next() {
            match arg {
                Value::String(s) => {
                    if s.to_uppercase() == "MATCH" {
                        if let Some(Value::String(p)) = args.next() {
                            pattern = Some(Pattern::new(p.as_ref())?);
                        } else {
                            return session
                                .respond(&value_error!("Invalid pattern"), writer)
                                .await;
                        }
                    } else if s.to_uppercase() == "COUNT" {
                        match args.next() {
                            Some(Value::Integer(c)) => {
                                // make sure the count is positive
                                count = c.unsigned_abs() as usize;
                            }
                            Some(Value::String(s)) => {
                                let Ok(c) = s.parse::<usize>() else {
                                    return session
                                        .respond(&value_error!("Invalid count"), writer)
                                        .await;
                                };

                                count = c;
                            }
                            _ => {
                                return session
                                    .respond(&value_error!("Invalid count"), writer)
                                    .await;
                            }
                        }
                    } else if let Ok(c) = s.parse::<usize>()
                        && cursor.is_none()
                    {
                        cursor = Some(c);
                    } else {
                        return session
                            .respond(&value_error!("Invalid argument"), writer)
                            .await;
                    }
                }

                Value::Integer(c) if cursor.is_none() => {
                    cursor = Some(c.unsigned_abs() as usize);
                }

                _ => {
                    return session
                        .respond(&value_error!("Invalid argument"), writer)
                        .await;
                }
            }
        }

        let store = session.state.store.read().await;
        let mut results = Vec::with_capacity(count);
        let cursor_start = cursor.unwrap_or(0);
        let mut next_cursor = 0;

        #[cfg(debug_assertions)]
        log::debug!("SCAN cursor: {cursor:?}, pattern: {pattern:?}, count: {count}");

        for (idx, (key, value)) in store.iter().enumerate().skip(cursor_start) {
            #[cfg(debug_assertions)]
            log::debug!("SCAN key: {key}");

            // Skip expired keys (consistent with KEYS command)
            if value.expired() {
                continue;
            }

            let matches = pattern.as_ref().is_none_or(|p| p.matches(key));

            if matches {
                results.push(Value::String(key.clone()));
                if results.len() >= count {
                    next_cursor = idx + 1;
                    break;
                }
            }
        }

        // If we didn't break early, we've scanned everything - cursor is 0
        session
            .respond(
                &Value::Multi(
                    Vec::from([
                        Value::String(next_cursor.to_string().into()),
                        Value::Multi(results.into()),
                    ])
                    .into(),
                ),
                writer,
            )
            .await
    }
}
