use crate::prelude::*;

pub struct Scan;

#[async_trait]
impl CommandTrait for Scan {
    fn name(&self) -> &str {
        "SCAN"
    }

    async fn handle_command(
        &self,
        writer: &mut WriteHalf,
        args: &mut VecDeque<Value>,
        session: SessionRef,
    ) -> Result<()> {
        let mut cursor = 0;
        let mut pattern: Option<String> = None;
        let mut count = 10;

        while let Some(arg) = args.pop_front() {
            match arg {
                Value::String(s) => {
                    if s.to_uppercase() == "MATCH" {
                        if let Some(Value::String(p)) = args.pop_front() {
                            pattern = Some(p);
                        } else {
                            return session
                                .versioned_response(&value_error!("Invalid pattern"), writer)
                                .await;
                        }
                    } else if s.to_uppercase() == "COUNT" {
                        match args.pop_front() {
                            Some(Value::Integer(c)) => {
                                // make sure the count is positive
                                count = c.abs() as usize;
                            }
                            Some(Value::String(s)) => {
                                let Ok(c) = s.parse::<usize>() else {
                                    return session
                                        .versioned_response(&value_error!("Invalid count"), writer)
                                        .await;
                                };

                                count = c;
                            }
                            _ => {
                                return session
                                    .versioned_response(&value_error!("Invalid count"), writer)
                                    .await;
                            }
                        }
                    } else if let Ok(c) = s.parse::<usize>() {
                        cursor = c;
                    } else {
                        return session
                            .versioned_response(&value_error!("Invalid argument"), writer)
                            .await;
                    }
                }

                Value::Integer(c) => {
                    cursor = c.abs() as usize;
                }

                _ => {
                    return session
                        .versioned_response(&value_error!("Invalid argument"), writer)
                        .await;
                }
            }
        }

        let store = session.state.store.read().await;
        let keys = store.keys().cloned().collect::<Vec<String>>();
        let mut results = VecDeque::new();

        log::debug!("SCAN cursor: {cursor}, pattern: {pattern:?}, count: {count}");

        for key in keys.iter().skip(cursor) {
            log::debug!("SCAN key: {key}");

            if let Some(pattern) = &pattern {
                if !key.contains(pattern) {
                    continue;
                }
            } else {
                results.push_back(Value::String(key.clone()));
            }

            if results.len() >= count {
                break;
            }
        }

        Value::Multi(VecDeque::from([
            Value::String(results.len().to_string()),
            Value::Multi(results),
        ]))
        .to_resp2(writer)
        .await
    }
}
