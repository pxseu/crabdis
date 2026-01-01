use crate::prelude::*;

pub struct Scan;

#[async_trait]
impl CommandTrait for Scan {
    fn name(&self) -> &'static str {
        "SCAN"
    }

    async fn handle_command(
        &self,
        writer: &mut (dyn tokio::io::AsyncWrite + Unpin + Send),
        args: &mut Args<'_>,
        session: SessionRef,
    ) -> Result<()> {
        let mut cursor = Option::<usize>::None;
        let mut pattern = Option::<Arc<str>>::None;
        let mut count = 10;

        while let Some(arg) = args.next() {
            match arg {
                Value::String(s) => {
                    if s.to_uppercase() == "MATCH" {
                        if let Some(Value::String(p)) = args.next() {
                            pattern = Some(p.clone());
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

        for (idx, key) in store.keys().enumerate().skip(cursor_start) {
            #[cfg(debug_assertions)]
            log::debug!("SCAN key: {key}");

            let matches = pattern.as_ref().is_none_or(|p| key.contains(p.as_ref()));

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
